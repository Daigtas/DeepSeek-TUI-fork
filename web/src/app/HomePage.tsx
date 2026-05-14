"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import { useSession, signOut } from "@/lib/auth/client";
import { useRouter } from "next/navigation";
import dynamic from "next/dynamic";
import { Sidebar } from "@/components/Sidebar";
import { ChatArea } from "@/components/ChatArea";
import { StatusBar } from "@/components/StatusBar";
import { PermissionPrompt } from "@/components/PermissionPrompt";
import { ToastProvider, useToast } from "@/components/Toast";

const CommandPalette = dynamic(() => import("@/components/CommandPalette").then(m => ({ default: m.CommandPalette })), { ssr: false });
import type { ChatMessage, PermissionRequest, FileAttachment, UserPreferences, SessionSummary } from "@/lib/types";
import { DEFAULT_PREFERENCES } from "@/lib/types";
import { useSessionPersistence } from "@/lib/useSessionPersistence";
import { LogOut, MessageSquare, Settings, User, Bot, Plus, RefreshCw } from "lucide-react";

interface ToolCallEntry {
  id: string;
  name: string;
  input: Record<string, unknown>;
  result?: string;
  isError?: boolean;
}

function HomePageInner() {
  const { data: session, isPending } = useSession();
  const router = useRouter();
  const { messages, setMessages, loaded: sessionLoaded, ensureSession, persistMessage, newSession } = useSessionPersistence();
  const { toast } = useToast();
  const [streamingText, setStreamingText] = useState("");
  const [currentTools, setCurrentTools] = useState<ToolCallEntry[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [longWait, setLongWait] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const errorTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [showPalette, setShowPalette] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [pendingPermission, setPendingPermission] = useState<PermissionRequest | null>(null);
  const [preferences, setPreferences] = useState<UserPreferences>(DEFAULT_PREFERENCES);
  const [attachments, setAttachments] = useState<FileAttachment[]>([]);
  const [midTaskMode, setMidTaskMode] = useState(false);
  const [showSwarmPanel, setShowSwarmPanel] = useState(false);
  const [contextFlash, setContextFlash] = useState<"warning" | "critical" | null>(null);
  const [wsConnected, setWsConnected] = useState(false);

  const [tokenUsage, setTokenUsage] = useState({ used: 0, limit: DEFAULT_PREFERENCES.contextLimit });
  const [roundCount, setRoundCount] = useState(0);
  const [sessionStartTime] = useState(Date.now());

  function formatTokenLabel(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
    return String(n);
  }

  const wsRef = useRef<WebSocket | null>(null);
  // Pending message queue — drained when ws.onopen fires (avoids handler clobbering)
  const pendingMessagesRef = useRef<string[]>([]);
  const preferencesRef = useRef(preferences);
  const reconnectAttemptRef = useRef(0);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const connectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const streamTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const longWaitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const permissionResolver = useRef<((value: boolean) => void) | null>(null);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);
  const pendingTaskRef = useRef<boolean>(false); // tracks if agent has pending work

  // Refs so WebSocket callbacks always see latest streaming/tool values (avoids stale closure)
  const streamingTextRef = useRef(streamingText);
  const currentToolsRef = useRef(currentTools);
  streamingTextRef.current = streamingText;
  currentToolsRef.current = currentTools;
  preferencesRef.current = preferences;

  // Load user preferences — refresh when the page gains visibility (after returning from /settings)
  useEffect(() => {
    const load = () => {
      fetch("/api/settings")
        .then(r => r.json())
        .then(d => { if (d.preferences) setPreferences({ ...DEFAULT_PREFERENCES, ...d.preferences }); })
        .catch((err) => console.warn("[HomePage] failed to load settings:", err));
    };
    load();
    document.addEventListener("visibilitychange", load);
    return () => document.removeEventListener("visibilitychange", load);
  }, []);

  // Auto-create/restore session when logged in
  const [sessions, setSessions] = useState<SessionSummary[]>([]);

  // Load saved sessions for sidebar
  useEffect(() => {
    if (!session?.user) return;
    fetch("/api/sessions")
      .then(r => r.json())
      .then(d => {
        if (d.sessions) {
          setSessions(d.sessions.map((s: any) => ({
            id: s.id,
            title: s.title,
            updatedAt: s.updatedAt,
            messageCount: s.messages?.length || 0,
          })));
        }
      })
      .catch((err) => console.warn("[HomePage] failed to load sessions:", err));
  }, [session?.user]);

  // Auto-create/restore session when logged in
  useEffect(() => {
    if (session?.user && sessionLoaded) {
      ensureSession();
    }
  }, [session?.user, sessionLoaded, ensureSession]);

  // Persist user messages to DB
  useEffect(() => {
    if (!sessionLoaded || messages.length === 0) return;
    const lastMsg = messages[messages.length - 1];
    if (lastMsg.role === "user" || lastMsg.role === "assistant") {
      persistMessage(lastMsg);
    }
  }, [messages.length]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamingText]);

  // Auto-dismiss error after 10 seconds
  useEffect(() => {
    if (!error) return;
    if (errorTimerRef.current) clearTimeout(errorTimerRef.current);
    errorTimerRef.current = setTimeout(() => {
      setError(null);
    }, 10_000);
    return () => {
      if (errorTimerRef.current) clearTimeout(errorTimerRef.current);
    };
  }, [error]);

  // Drain pending message queue
  const drainPendingMessages = useCallback((ws: WebSocket) => {
    while (pendingMessagesRef.current.length > 0) {
      const msg = pendingMessagesRef.current.shift()!;
      try { ws.send(msg); } catch { /* connection may close concurrently */ }
    }
  }, []);

  // Queue a message — sends immediately if connected, queues otherwise
  const queueOrSend = useCallback((payload: string, ws: WebSocket) => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(payload);
    } else {
      pendingMessagesRef.current.push(payload);
    }
  }, []);

  const connectWS = useCallback((): WebSocket => {
    // If already connected, return existing socket after draining queued messages
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      drainPendingMessages(wsRef.current);
      return wsRef.current;
    }

    // Close any orphan socket that's in a non-OPEN state
    if (wsRef.current) {
      try { wsRef.current.close(); } catch { /* already closing */ }
      wsRef.current = null;
    }

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${window.location.host}/ws`;
    const ws = new WebSocket(wsUrl);

    wsRef.current = ws;

    ws.onopen = () => {
      console.log("[ws] connected to", wsUrl);
      reconnectAttemptRef.current = 0;
      setWsConnected(true);
      setError(null);
      // Drain any messages queued before the socket opened
      drainPendingMessages(ws);
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        switch (data.type) {
          case "text":
            setIsSending(false);
            setLongWait(false);
            if (longWaitTimerRef.current) { clearTimeout(longWaitTimerRef.current); longWaitTimerRef.current = null; }
            setStreamingText((prev) => {
              const updated = prev + data.text;
              // Live token estimate: ~1.3 chars per token (rough)
              const estimatedTokens = Math.round(updated.length / 1.3);
              const limit = preferencesRef.current.contextLimit;
              setTokenUsage({ used: estimatedTokens, limit });
              const remainingPct = limit > 0 ? ((limit - estimatedTokens) / limit) * 100 : 100;
              if (remainingPct <= 25) setContextFlash("critical");
              else if (remainingPct <= 35) setContextFlash("warning");
              return updated;
            });
            break;
          case "tool_call":
            setIsSending(false);
            setLongWait(false);
            if (longWaitTimerRef.current) { clearTimeout(longWaitTimerRef.current); longWaitTimerRef.current = null; }
            setCurrentTools((prev) => [...prev, { id: crypto.randomUUID(), name: data.name, input: data.input }]);
            break;
          case "tool_result":
            setCurrentTools((prev) =>
              prev.map((t) => (t.name === data.name && !t.result ? { ...t, result: data.content, isError: data.isError } : t)),
            );
            break;
          case "stream_reset":
            if (streamingTextRef.current) {
              setMessages((prev) => [...prev, { role: "assistant", content: streamingTextRef.current + "\n\n_[Correction received — rethinking with full context...]_", toolCalls: [...currentToolsRef.current] }]);
            }
            setStreamingText("");
            setCurrentTools([]);
            break;
          case "done": {
            const finalText = data.text || streamingTextRef.current;
            const finalTools = [...currentToolsRef.current];
            if (!finalText.trim() && finalTools.length === 0) {
              setError("Agent completed with an empty response. Try rephrasing your prompt or check the daemon status.");
            } else {
              setMessages((prev) => [...prev, { role: "assistant", content: finalText, toolCalls: finalTools }]);
            }
            setStreamingText("");
            setCurrentTools([]);
            setIsStreaming(false);
            setIsSending(false);
            setLongWait(false);
            setMidTaskMode(false);
            pendingTaskRef.current = false;
            if (connectTimeoutRef.current) { clearTimeout(connectTimeoutRef.current); connectTimeoutRef.current = null; }
            if (streamTimeoutRef.current) { clearTimeout(streamTimeoutRef.current); streamTimeoutRef.current = null; }
            if (longWaitTimerRef.current) { clearTimeout(longWaitTimerRef.current); longWaitTimerRef.current = null; }
            const used = data.tokens || 0;
            const limit = preferencesRef.current.contextLimit;
            setTokenUsage({ used, limit });
            setRoundCount((r) => r + 1);
            const remainingPct = limit > 0 ? ((limit - used) / limit) * 100 : 100;
            if (remainingPct <= 25) setContextFlash("critical");
            else if (remainingPct <= 35) setContextFlash("warning");
            break;
            }
          case "error":
            setError(data.message);
            setIsStreaming(false);
            setIsSending(false);
            setLongWait(false);
            setMidTaskMode(false);
            pendingTaskRef.current = false;
            if (connectTimeoutRef.current) { clearTimeout(connectTimeoutRef.current); connectTimeoutRef.current = null; }
            if (streamTimeoutRef.current) { clearTimeout(streamTimeoutRef.current); streamTimeoutRef.current = null; }
            if (longWaitTimerRef.current) { clearTimeout(longWaitTimerRef.current); longWaitTimerRef.current = null; }
            break;
          case "session_loaded":
            console.log("[ws] session restored:", data.messages?.length, "messages");
            break;
        }
      } catch (err) {
        if (process.env.NODE_ENV === "development") {
          console.warn("[ws] failed to parse message:", err);
        }
      }
    };

    ws.onerror = () => {
      console.error("[ws] connection error to", wsUrl);
      setWsConnected(false);
      if (reconnectAttemptRef.current === 0) {
        setError("WebSocket connection failed. Check if the server is running.");
      }
    };

    // SINGLE unified onclose — handles session expiry, reconnection, and cleanup
    ws.onclose = (event: CloseEvent) => {
      setWsConnected(false);
      const wasStreaming = pendingTaskRef.current;
      setIsStreaming(false);
      setIsSending(false);
      setLongWait(false);
      setMidTaskMode(false);
      pendingTaskRef.current = false;
      if (connectTimeoutRef.current) { clearTimeout(connectTimeoutRef.current); connectTimeoutRef.current = null; }
      if (streamTimeoutRef.current) { clearTimeout(streamTimeoutRef.current); streamTimeoutRef.current = null; }
      if (longWaitTimerRef.current) { clearTimeout(longWaitTimerRef.current); longWaitTimerRef.current = null; }

      // Session expired — no reconnection
      if (event.code === 4401) {
        setError("Session expired. Please refresh the page.");
        reconnectAttemptRef.current = 0;
        return;
      }

      // Connection dropped while work was in progress
      if (wasStreaming && event.code !== 1000) {
        setError(`Connection lost${event.reason ? ': ' + event.reason : ''}. ${reconnectAttemptRef.current < 5 ? 'Reconnecting…' : 'Refresh to retry.'}`);
      }

      // Exponential backoff reconnection with jitter
      if (wasStreaming && reconnectAttemptRef.current < 5) {
        const delay = Math.min(30000, 1000 * Math.pow(2, reconnectAttemptRef.current) + Math.random() * 1000);
        reconnectAttemptRef.current += 1;
        reconnectTimerRef.current = setTimeout(() => {
          setError(null);
          connectWS();
        }, delay);
      } else if (!wasStreaming || reconnectAttemptRef.current >= 5) {
        reconnectAttemptRef.current = 0;
      }
    };

    return ws;
  }, [drainPendingMessages]);

  const sendToAgent = useCallback(
    (text: string, isMidTask: boolean) => {
      if (!text.trim() || (isStreaming && !isMidTask)) return;
      setError(null);

      if (isMidTask) {
        // Mid-task correction: append to existing stream without clearing
        setMidTaskMode(true);
        const ws = connectWS();
        queueOrSend(JSON.stringify({
          type: "chat",
          prompt: text,
          mode: preferencesRef.current.agentMode,
          midTask: true,
          attachments: attachments.map(a => ({ fileName: a.fileName, fileSize: a.fileSize, mimeType: a.mimeType })),
        }), ws);
        return;
      }

      // New task
      setMessages((prev) => [...prev, { role: "user", content: text, attachments: [...attachments], _midTask: false }]);
      setStreamingText("");
      setCurrentTools([]);
      setIsStreaming(true);
      setIsSending(true);
      setLongWait(false);
      pendingTaskRef.current = true;
      setAttachments([]);

      const ws = connectWS();
      const payload = JSON.stringify({
        type: "chat",
        prompt: text,
        mode: preferencesRef.current.agentMode,
        attachments: attachments.map(a => ({ fileName: a.fileName, fileSize: a.fileSize, mimeType: a.mimeType })),
      });

      // Clear any previous timeouts
      if (connectTimeoutRef.current) clearTimeout(connectTimeoutRef.current);
      if (streamTimeoutRef.current) clearTimeout(streamTimeoutRef.current);

      // Connection timeout — if WS doesn't open within 15s, surface error
      connectTimeoutRef.current = setTimeout(() => {
        if (pendingTaskRef.current && wsRef.current?.readyState !== WebSocket.OPEN) {
          setError("Connection timed out. Check that the agent server is running.");
          setIsStreaming(false);
          setIsSending(false);
          setLongWait(false);
          pendingTaskRef.current = false;
          wsRef.current?.close();
        }
      }, 15000);

      // Long wait indicator — show "Taking longer..." after 30s
      if (longWaitTimerRef.current) clearTimeout(longWaitTimerRef.current);
      longWaitTimerRef.current = setTimeout(() => {
        if (pendingTaskRef.current && !streamingTextRef.current && currentToolsRef.current.length === 0) {
          setLongWait(true);
        }
      }, 30000);

      // Stream timeout — if agent takes > 5 min, abort
      streamTimeoutRef.current = setTimeout(() => {
        if (pendingTaskRef.current) {
          setError("Agent is taking too long. The task may be hung.");
          setIsStreaming(false);
          setIsSending(false);
          setLongWait(false);
          setStreamingText("");
          setCurrentTools([]);
          pendingTaskRef.current = false;
          try { wsRef.current?.send(JSON.stringify({ type: "abort" })); } catch { /* best effort */ }
        }
      }, 5 * 60 * 1000);

      queueOrSend(payload, ws);
    },
    [isStreaming, connectWS, queueOrSend, attachments],
  );

  const handleSubmit = useCallback(
    (text: string) => {
      sendToAgent(text, isStreaming && pendingTaskRef.current);
    },
    [isStreaming, sendToAgent],
  );

  const handlePermission = useCallback(async (request: PermissionRequest): Promise<boolean> => {
    if (preferences.autoApprove) return true;
    setPendingPermission(request);
    return new Promise((resolve) => {
      permissionResolver.current = (approved: boolean) => {
        setPendingPermission(null);
        permissionResolver.current = null;
        resolve(approved);
      };
    });
  }, [preferences.autoApprove]);

  const handleAbort = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: "abort" }));
    }
    setIsStreaming(false);
    setIsSending(false);
    setLongWait(false);
    setStreamingText("");
    setCurrentTools([]);
    setMidTaskMode(false);
    pendingTaskRef.current = false;
    if (connectTimeoutRef.current) { clearTimeout(connectTimeoutRef.current); connectTimeoutRef.current = null; }
    if (streamTimeoutRef.current) { clearTimeout(streamTimeoutRef.current); streamTimeoutRef.current = null; }
    if (longWaitTimerRef.current) { clearTimeout(longWaitTimerRef.current); longWaitTimerRef.current = null; }
  }, []);

  // Slash command routing (aligned with desktop TUI)
  const handleSlashCommand = useCallback((command: string, args?: string) => {
    const cmd = (command || "").replace(/^\//, "").split(/\s+/)[0];
    switch (cmd) {
      case "help": setShowPalette(true); break;
      case "clear": newSession(); setError(null); break;
      case "compact":
        setError("Context compact: request sent to agent for budget management");
        sendToAgent("/compact", isStreaming && pendingTaskRef.current);
        break;
      case "agents": setShowSwarmPanel(!showSwarmPanel); break;
      case "status": setError("Daemon status: connected, agents idle"); break;
      case "save":
        (async () => {
          const sid = await ensureSession();
          for (const msg of messages) {
            await persistMessage(msg, sid);
          }
          setError("Session saved ✓");
        })();
        break;
      case "resume":
        setError("Resume: check sidebar for saved sessions");
        setSidebarOpen(true);
        break;
      default: break;
    }
  }, [isStreaming, sendToAgent, showSwarmPanel, ensureSession, persistMessage, messages]);

  const handleAddAttachments = useCallback((files: FileAttachment[]) => {
    setAttachments(prev => [...prev, ...files]);
  }, []);

  const handleRemoveAttachment = useCallback((id: string) => {
    setAttachments(prev => prev.filter(a => a.id !== id));
  }, []);



  useEffect(() => {
    if (!isPending && !session) {
      router.push("/login");
    }
  }, [isPending, session, router]);

  if (isPending) {
    return <div className="flex h-screen items-center justify-center bg-bg" aria-busy="true"><div className="animate-pulse text-fg-faint text-sm">Loading…</div></div>;
  }

  if (!session) {
    return <div className="flex h-screen items-center justify-center bg-bg"><div className="animate-pulse text-fg-faint text-sm">Redirecting…</div></div>;
  }

  const modeLabel = preferences.agentMode === "yolo" ? "YOLO" : preferences.agentMode === "plan" ? "Plan" : "Agent";

  return (
    <div className="flex h-screen flex-col bg-bg text-fg">
      <div className="flex flex-1 overflow-hidden">
        <Sidebar
          isOpen={sidebarOpen}
          onToggle={() => setSidebarOpen(!sidebarOpen)}
          onSignOut={() => signOut()}
          session={session}
          sessions={sessions}
          activeSession={localStorage.getItem("deepseek-tui-session-id") || undefined}
          onSessionSelect={(id) => {
            localStorage.setItem("deepseek-tui-session-id", id);
            // Load messages from API
            fetch("/api/sessions")
              .then(r => r.json())
              .then(d => {
                const found = d.sessions?.find((s: any) => s.id === id);
                if (found?.messages?.length) {
                  setMessages(found.messages.map((m: any) => ({
                    role: m.role,
                    content: m.content,
                    toolCalls: m.toolCalls || undefined,
                  })));
                  // Also tell WS server to restore
                  if (wsRef.current?.readyState === WebSocket.OPEN) {
                    wsRef.current.send(JSON.stringify({ type: "restore", sessionId: id }));
                  }
                }
              })
              .catch((err) => console.warn("[HomePage] failed to restore session:", err));
            setSidebarOpen(false);
          }}
        />

        <div className="flex flex-1 flex-col min-w-0">
          {/* Header — mobile-first: compact, icon-based */}
          <header className="flex items-center gap-2 border-b border-border px-3 py-2.5">
            <button onClick={() => setSidebarOpen(!sidebarOpen)} className="text-fg-faint hover:text-fg transition-colors font-mono text-sm shrink-0" aria-label={sidebarOpen ? "Close sidebar" : "Open sidebar"}>
              {sidebarOpen ? "▰" : "▱"}
            </button>
            <h1 className="text-sm font-bold text-amber-light truncate">DeepSeek TUI</h1>

            {/* Agent mode badge */}
            <span className={`hidden sm:inline-flex items-center gap-1 rounded border px-2 py-0.5 text-[10px] font-semibold shrink-0 ${
              preferences.agentMode === "yolo"
                ? "border-rose/30 bg-rose/5 text-rose"
                : preferences.agentMode === "plan"
                ? "border-cyan/30 bg-cyan/5 text-cyan"
                : "border-amber/30 bg-amber/5 text-amber"
            }`}>
              <Bot className="h-3 w-3" />
              {modeLabel}
            </span>

            {/* Mid-task indicator */}
            {midTaskMode && (
              <span className="flex items-center gap-1 rounded border border-green/30 bg-green/5 px-2 py-0.5 text-[10px] font-semibold text-green animate-pulse shrink-0">
                Correction
              </span>
            )}

            <span className="ml-auto text-xs text-fg-faint font-mono hidden sm:inline truncate">{session.user?.email}</span>

            {/* Navigation icons — mobile-first */}
            <nav className="flex items-center gap-1 sm:gap-2 shrink-0">
              <button onClick={newSession} className="rounded p-2 sm:p-1.5 text-fg-faint hover:text-green hover:bg-green/5 transition-colors min-w-[44px] min-h-[44px] flex items-center justify-center" title="New session">
                <Plus className="h-5 w-5 sm:h-4 sm:w-4" />
              </button>
              <button onClick={() => router.push("/settings")} className="rounded p-2 sm:p-1.5 text-fg-faint hover:text-amber hover:bg-amber/5 transition-colors min-w-[44px] min-h-[44px] flex items-center justify-center" title="Settings">
                <Settings className="h-5 w-5 sm:h-4 sm:w-4" />
              </button>
              <button onClick={() => router.push("/profile")} className="rounded p-2 sm:p-1.5 text-fg-faint hover:text-amber hover:bg-amber/5 transition-colors min-w-[44px] min-h-[44px] flex items-center justify-center" title="Profile">
                <User className="h-5 w-5 sm:h-4 sm:w-4" />
              </button>
              <button onClick={() => signOut()} className="rounded p-2 sm:p-1.5 text-fg-faint hover:text-rose hover:bg-rose/5 transition-colors min-w-[44px] min-h-[44px] flex items-center justify-center" title="Sign out">
                <LogOut className="h-5 w-5 sm:h-4 sm:w-4" />
              </button>
            </nav>
          </header>

          {/* Command palette */}
          {showPalette && (
            <div className="border-b border-border px-4 py-2">
              <CommandPalette onSelect={(cmd) => { setError(null); handleSubmit(cmd); }} onClose={() => setShowPalette(false)} />
            </div>
          )}

          {/* Permission */}
          {pendingPermission && (
            <div className="border-b border-border px-4 py-3 bg-card">
              <PermissionPrompt request={pendingPermission} onApprove={() => permissionResolver.current?.(true)} onDeny={() => permissionResolver.current?.(false)} />
            </div>
          )}

          {/* Error banner */}
          {error && (
            <div className="border-b border-rose/30 bg-rose/10 px-4 py-3 animate-slide-up-fade" role="alert">
              <div className="flex items-start gap-3">
                <p className="flex-1 text-sm text-rose font-mono leading-relaxed break-words">{error}</p>
                <div className="flex items-center gap-1.5 shrink-0">
                  <button
                    onClick={() => {
                      if (errorTimerRef.current) clearTimeout(errorTimerRef.current);
                      setError(null);
                      connectWS();
                    }}
                    className="inline-flex items-center gap-1 rounded border border-rose/40 px-2.5 py-1 text-xs text-rose hover:bg-rose/20 transition-colors"
                    title="Retry connection"
                  >
                    <RefreshCw className="h-3 w-3" />
                    Retry
                  </button>
                  <button
                    onClick={() => {
                      if (errorTimerRef.current) clearTimeout(errorTimerRef.current);
                      setError(null);
                    }}
                    className="shrink-0 rounded p-1 text-rose/70 hover:text-rose hover:bg-rose/10 transition-colors"
                    aria-label="Dismiss error"
                  >
                    ✕
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* Long wait indicator */}
          {longWait && isStreaming && (
            <div className="border-b border-amber/20 bg-amber/5 px-4 py-2 animate-fade-in" role="status">
              <p className="text-xs text-amber font-mono flex items-center gap-2">
                <span className="inline-block h-1.5 w-1.5 animate-pulse bg-amber" />
                Taking longer than expected… the agent may be processing a large task.
              </p>
            </div>
          )}

          {/* Context flash (desktop UiEvent::ContextWarning/Critical) */}
          {contextFlash && !isStreaming && (
            <div className={`border-b px-4 py-2 text-xs font-mono ${
              contextFlash === "critical"
                ? "border-rose/20 bg-rose/5 text-rose"
                : "border-amber/20 bg-amber/5 text-amber"
            }`}>
              <span className="font-bold">
                {contextFlash === "critical" ? "CONTEXT CRITICAL" : "CONTEXT WARN"}
              </span>
              {" "}
              {contextFlash === "critical"
                ? "Budget nearly exhausted. Use /compact or increase limit in settings."
                : "Budget running low. Consider /compact soon."}
              <button
                onClick={() => setContextFlash(null)}
                className="ml-3 text-fg-faint hover:text-fg"
              >✕</button>
            </div>
          )}

          {/* Swarm / Agents panel */}
          {showSwarmPanel && (
            <div className="border-b border-border bg-alt px-4 py-3">
              <div className="flex items-center justify-between mb-2">
                <h3 className="text-xs font-semibold uppercase tracking-widest text-amber">Agent Status</h3>
                <button onClick={() => setShowSwarmPanel(false)} className="text-fg-faint hover:text-fg text-xs" aria-label="Close agent panel">✕</button>
              </div>
              <div className="grid grid-cols-2 sm:grid-cols-3 gap-x-4 gap-y-1.5 text-xs">
                <div className="flex items-center gap-1.5">
                  <span className={`h-1.5 w-1.5 rounded-full ${isStreaming ? "bg-green animate-pulse" : "bg-fg-faint"}`} />
                  <span className="text-fg-dim">Status:</span>
                  <span className={isStreaming ? "text-green font-mono" : "text-fg-faint font-mono"}>{isStreaming ? "active" : "idle"}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <span className="h-1.5 w-1.5 rounded-full bg-amber" />
                  <span className="text-fg-dim">Mode:</span>
                  <span className="text-amber font-mono">{modeLabel}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <span className="h-1.5 w-1.5 rounded-full bg-cyan" />
                  <span className="text-fg-dim">Model:</span>
                  <span className="text-cyan font-mono text-[10px]">{preferences.model?.split("-").pop()}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <span className="h-1.5 w-1.5 rounded-full bg-fg-faint" />
                  <span className="text-fg-dim">Rounds:</span>
                  <span className="font-mono">{roundCount}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <span className="h-1.5 w-1.5 rounded-full bg-fg-faint" />
                  <span className="text-fg-dim">Tokens:</span>
                  <span className="font-mono">{formatTokenLabel(tokenUsage.used)}/{formatTokenLabel(tokenUsage.limit)}</span>
                </div>
                <div className="flex items-center gap-1.5">
                  <span className="h-1.5 w-1.5 rounded-full bg-fg-faint" />
                  <span className="text-fg-dim">Tools:</span>
                  <span className="font-mono">{currentTools.length}{isStreaming ? "+" : ""}</span>
                </div>
              </div>
              {midTaskMode && (
                <div className="mt-2 border-t border-amber/20 pt-2 text-xs text-amber font-mono">
                  Mid-task correction active — agent re-evaluating
                </div>
              )}
            </div>
          )}

          {/* Chat */}
          <ChatArea
            messages={messages}
            streamingText={streamingText}
            currentTools={currentTools}
            isStreaming={isStreaming}
            onSubmit={handleSubmit}
            onOpenPalette={() => setShowPalette(true)}
            messagesEndRef={messagesEndRef}
            onAbort={handleAbort}
            preferences={preferences}
            attachments={attachments}
            onAddAttachments={handleAddAttachments}
            onRemoveAttachment={handleRemoveAttachment}
            midTaskMode={midTaskMode}
            onSlashCommand={handleSlashCommand}
            isLoading={!sessionLoaded}
            onFillInput={(text) => handleSubmit(text)}
          />

          {/* Status */}
          <StatusBar
            isStreaming={isStreaming}
            contextUsed={tokenUsage.used}
            contextLimit={tokenUsage.limit}
            roundCount={roundCount}
            sessionDuration={Math.round((Date.now() - sessionStartTime) / 1000)}
            model={preferences.model}
            wsConnected={wsConnected}
            reconnectAttempt={reconnectAttemptRef.current}
          />
        </div>
      </div>
    </div>
  );
}

export default function HomePage() {
  return (
    <ToastProvider>
      <HomePageInner />
    </ToastProvider>
  );
}
