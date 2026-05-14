"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import { useSession, signOut } from "@/lib/auth/client";
import { useRouter } from "next/navigation";
import dynamic from "next/dynamic";
import { ChatArea } from "@/components/ChatArea";
import { StatusBar } from "@/components/StatusBar";

const CommandPalette = dynamic(() => import("@/components/CommandPalette").then(m => ({ default: m.CommandPalette })), { ssr: false });
import { PermissionPrompt } from "@/components/PermissionPrompt";
import type { ChatMessage, PermissionRequest } from "@/lib/types";
import { DEFAULT_PREFERENCES } from "@/lib/types";
import { LogOut, MessageSquare } from "lucide-react";

interface ToolCallEntry {
  id: string;
  name: string;
  input: Record<string, unknown>;
  result?: string;
  isError?: boolean;
}

const STREAM_TIMEOUT_MS = 5 * 60 * 1000; // 5 minutes

export default function ChatPageContent() {
  const { data: session, isPending } = useSession();
  const router = useRouter();
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [streamingText, setStreamingText] = useState("");
  const [currentTools, setCurrentTools] = useState<ToolCallEntry[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showPalette, setShowPalette] = useState(false);
  const [pendingPermission, setPendingPermission] = useState<PermissionRequest | null>(null);

  const [tokenUsage, setTokenUsage] = useState({ used: 0, limit: DEFAULT_PREFERENCES.contextLimit });
  const [roundCount, setRoundCount] = useState(0);
  const [sessionStartTime] = useState(Date.now());

  // ── Refs to keep latest streaming state without stale-closure issues ──
  const wsRef = useRef<WebSocket | null>(null);
  const streamingTextRef = useRef("");
  const currentToolsRef = useRef<ToolCallEntry[]>([]);
  const isStreamingRef = useRef(false);
  const streamTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const permissionResolver = useRef<((value: boolean) => void) | null>(null);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);

  // Keep refs in sync with state
  useEffect(() => { streamingTextRef.current = streamingText; }, [streamingText]);
  useEffect(() => { currentToolsRef.current = currentTools; }, [currentTools]);
  useEffect(() => { isStreamingRef.current = isStreaming; }, [isStreaming]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamingText]);

  // ── Clear stream timeout helper ──────────────────────────────────────
  const clearStreamTimeout = useCallback(() => {
    if (streamTimeoutRef.current) {
      clearTimeout(streamTimeoutRef.current);
      streamTimeoutRef.current = null;
    }
  }, []);

  // ── Reset streaming state (on error, close, abort) ───────────────────
  const resetStreaming = useCallback(() => {
    clearStreamTimeout();
    setIsStreaming(false);
    isStreamingRef.current = false;
    setStreamingText("");
    streamingTextRef.current = "";
    setCurrentTools([]);
    currentToolsRef.current = [];
  }, [clearStreamTimeout]);

  // ── Connect / get-or-reconnect WebSocket ─────────────────────────────
  const getOrConnectWS = useCallback((): WebSocket => {
    const existing = wsRef.current;
    // If we have an open WS, reuse it
    if (existing && existing.readyState === WebSocket.OPEN) return existing;

    // Clean up stale WS
    if (existing && existing.readyState !== WebSocket.CLOSED) {
      existing.close();
    }

    // Connect via the same origin (goes through K8s ingress with TLS on /ws path)
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${window.location.host}/ws`;
    const ws = new WebSocket(wsUrl);

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        switch (data.type) {
          case "text":
            setStreamingText((prev) => prev + data.text);
            break;
          case "tool_call":
            setCurrentTools((prev) => [...prev, { id: crypto.randomUUID(), name: data.name, input: data.input }]);
            break;
          case "tool_result":
            setCurrentTools((prev) =>
              prev.map((t) =>
                // Match by name AND no existing result, to handle duplicate tool names
                t.name === data.name && !t.result
                  ? { ...t, result: data.content, isError: data.isError }
                  : t
              ),
            );
            break;
          case "done": {
            // Use refs to get the latest streaming state (avoid stale closure)
            const finalText = data.text || streamingTextRef.current;
            const finalTools = [...currentToolsRef.current];
            // Don't add ghost messages — if there's no text and no tools, surface an error
            if (!finalText.trim() && finalTools.length === 0) {
              setError("Agent completed with an empty response. Try rephrasing your prompt or check the daemon status.");
            } else {
              setMessages((prev) => [...prev, { role: "assistant", content: finalText, toolCalls: finalTools }]);
            }
            resetStreaming();
            setTokenUsage({ used: data.tokens || 0, limit: DEFAULT_PREFERENCES.contextLimit });
            setRoundCount((r) => r + 1);
            break;
          }
          case "error":
            setError(data.message);
            resetStreaming();
            break;
        }
      } catch { /* ignore malformed messages */ }
    };

    ws.onerror = () => {
      // onclose will fire after onerror; reset there to avoid double-reset
    };

    ws.onclose = (event) => {
      // If we were streaming and the WS closed unexpectedly, surface the error
      if (isStreamingRef.current && event.code !== 1000) {
        const reason = event.reason || "Connection lost";
        setError(`WebSocket closed unexpectedly: ${reason}`);
        resetStreaming();
      } else if (isStreamingRef.current) {
        // Normal closure during streaming (unlikely, but handle gracefully)
        setError("Agent connection closed before completing the task.");
        resetStreaming();
      }
    };

    wsRef.current = ws;
    return ws;
  }, [resetStreaming]);

  // ── Start stream timeout ─────────────────────────────────────────────
  const startStreamTimeout = useCallback(() => {
    clearStreamTimeout();
    streamTimeoutRef.current = setTimeout(() => {
      if (isStreamingRef.current) {
        setError("Agent is taking too long to respond. The task may be hung.");
        resetStreaming();
        // Also abort the server-side processing
        try {
          if (wsRef.current?.readyState === WebSocket.OPEN) {
            wsRef.current.send(JSON.stringify({ type: "abort" }));
          }
        } catch { /* best effort */ }
      }
    }, STREAM_TIMEOUT_MS);
  }, [clearStreamTimeout, resetStreaming]);

  // ── Clean up on unmount ──────────────────────────────────────────────
  useEffect(() => {
    return () => {
      clearStreamTimeout();
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        wsRef.current.send(JSON.stringify({ type: "abort" }));
      }
      wsRef.current?.close();
    };
  }, [clearStreamTimeout]);

  // ── Submit handler ───────────────────────────────────────────────────
  const handleSubmit = useCallback(
    (text: string) => {
      if (!text.trim()) return;

      // If mid-task correction (currently streaming), allow submission
      if (isStreaming) {
        // Mid-task correction path
        setError(null);
        setMessages((prev) => [...prev, { role: "user", content: text, _midTask: true }]);
        try {
          const ws = getOrConnectWS();
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: "chat", prompt: text, midTask: true }));
          } else {
            ws.onopen = () => ws.send(JSON.stringify({ type: "chat", prompt: text, midTask: true }));
          }
        } catch {
          setError("Failed to send correction.");
        }
        // Reset timeout — new correction extends the wait
        startStreamTimeout();
        return;
      }

      // Fresh message
      setError(null);
      setMessages((prev) => [...prev, { role: "user", content: text }]);
      setStreamingText("");
      setCurrentTools([]);
      setIsStreaming(true);
      isStreamingRef.current = true;

      try {
        const ws = getOrConnectWS();
        const send = () => {
          ws.send(JSON.stringify({ type: "chat", prompt: text }));
          startStreamTimeout();
        };

        if (ws.readyState === WebSocket.OPEN) {
          send();
        } else {
          // Wait for connection, with a timeout
          const connectTimeout = setTimeout(() => {
            setError("Failed to connect to agent server. Is the backend running?");
            resetStreaming();
          }, 10_000);

          ws.onopen = () => {
            clearTimeout(connectTimeout);
            send();
          };
        }
      } catch (err) {
        setError(`Connection error: ${(err as Error).message || "Unknown error"}`);
        resetStreaming();
      }
    },
    [isStreaming, getOrConnectWS, startStreamTimeout, resetStreaming],
  );

  const handlePermission = useCallback(async (request: PermissionRequest): Promise<boolean> => {
    setPendingPermission(request);
    return new Promise((resolve) => {
      permissionResolver.current = (approved: boolean) => {
        setPendingPermission(null);
        permissionResolver.current = null;
        resolve(approved);
      };
    });
  }, []);

  const handleAbort = useCallback(() => {
    clearStreamTimeout();
    try {
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        wsRef.current.send(JSON.stringify({ type: "abort" }));
      }
    } catch { /* best effort */ }
    resetStreaming();
  }, [clearStreamTimeout, resetStreaming]);

  // ── Render ───────────────────────────────────────────────────────────
  useEffect(() => {
    if (!isPending && !session) {
      router.push("/login");
    }
  }, [isPending, session, router]);

  if (isPending) {
    return (
      <div className="flex h-screen items-center justify-center bg-[#0d0d0d]" aria-busy="true">
        <div className="animate-pulse text-[#888888]">Loading…</div>
      </div>
    );
  }

  if (!session) {
    return (
      <div className="flex h-screen items-center justify-center bg-[#0d0d0d]" aria-busy="true">
        <div className="animate-pulse text-[#888888]">Redirecting…</div>
      </div>
    );
  }

  return (
    <div className="flex h-screen flex-col bg-[#0d0d0d] text-[#d4d4d4]">
      {/* Minimal header */}
      <header className="flex items-center gap-3 border-b border-[#2a2a2a] px-4 py-3">
        <MessageSquare className="h-4 w-4 text-amber" />
        <h1 className="text-sm font-bold text-[#f0c040]">DeepSeek Chat</h1>
        <span className="ml-auto text-xs text-[#888888]">{session.user?.email}</span>
        <button
          onClick={() => signOut()}
          className="ml-2 rounded p-1 text-[#888888] hover:text-[#d75f5f] transition-colors"
          title="Sign out"
          aria-label="Sign out"
        >
          <LogOut className="h-4 w-4" />
        </button>
      </header>

      {/* Command palette */}
      {showPalette && (
        <div className="border-b border-[#2a2a2a] px-4 py-2">
          <CommandPalette onSelect={(cmd) => { setError(null); handleSubmit(cmd); }} onClose={() => setShowPalette(false)} />
        </div>
      )}

      {/* Permission prompt */}
      {pendingPermission && (
        <div className="border-b border-[#e6a817]/30 bg-[#e6a817]/5 px-4 py-3">
          <PermissionPrompt request={pendingPermission} onApprove={() => permissionResolver.current?.(true)} onDeny={() => permissionResolver.current?.(false)} />
        </div>
      )}

      {/* Error */}
      {error && (
        <div className="border-b border-[#d75f5f]/30 bg-[#d75f5f]/5 px-4 py-2" role="alert">
          <p className="text-sm text-[#d75f5f]">{error}</p>
        </div>
      )}

      {/* Chat area */}
      <main className="flex flex-1 min-h-0" aria-label="Chat">
        <ChatArea
          messages={messages}
          streamingText={streamingText}
          currentTools={currentTools}
          isStreaming={isStreaming}
          onSubmit={handleSubmit}
          onOpenPalette={() => setShowPalette(true)}
          messagesEndRef={messagesEndRef}
          onAbort={handleAbort}
          midTaskMode={isStreaming}
        />
      </main>

      {/* Status bar */}
      <StatusBar
        isStreaming={isStreaming}
        contextUsed={tokenUsage.used}
        contextLimit={tokenUsage.limit}
        roundCount={roundCount}
        sessionDuration={Math.round((Date.now() - sessionStartTime) / 1000)}
      />
    </div>
  );
}
