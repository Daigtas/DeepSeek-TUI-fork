"use client";

import { useState, useCallback, useEffect, useRef } from "react";
import type { ChatMessage } from "@/lib/types";

const STORAGE_KEY = "deepseek-tui-session";
const SESSION_ID_KEY = "deepseek-tui-session-id";

interface StoredSession {
  sessionId: string;
  messages: ChatMessage[];
  savedAt: number;
}

/**
 * Persists chat messages to localStorage (for navigation survival)
 * and optionally to the database (for login survival).
 */
export function useSessionPersistence() {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loaded, setLoaded] = useState(false);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const messagesRef = useRef(messages);
  messagesRef.current = messages;

  // Load from localStorage on mount
  useEffect(() => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      const storedId = localStorage.getItem(SESSION_ID_KEY);

      if (stored) {
        const data: StoredSession = JSON.parse(stored);
        if (data.messages?.length) {
          setMessages(data.messages);
          setSessionId(data.sessionId || storedId || null);
        }
      }
      if (storedId && !sessionId) {
        setSessionId(storedId);
      }
    } catch { /* corrupted storage, ignore */ }
    setLoaded(true);
  }, []);

  // Save to localStorage on message changes (debounced)
  useEffect(() => {
    if (!loaded) return;
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify({
          sessionId,
          messages: messagesRef.current,
          savedAt: Date.now(),
        }));
      } catch { /* storage full */ }
    }, 500);
    return () => { if (saveTimer.current) clearTimeout(saveTimer.current); };
  }, [messages, sessionId, loaded]);

  // Create or get session from DB
  const ensureSession = useCallback(async (): Promise<string> => {
    // If we already have a sessionId in localStorage, use it
    const storedId = localStorage.getItem(SESSION_ID_KEY);
    if (storedId) {
      setSessionId(storedId);
      return storedId;
    }

    // Also check state
    if (sessionId) return sessionId;

    // Create a new session via API
    try {
      const res = await fetch("/api/sessions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ title: getSessionTitle() }),
      });
      if (res.ok) {
        const data = await res.json();
        const id = data.session.id;
        localStorage.setItem(SESSION_ID_KEY, id);
        setSessionId(id);
        return id;
      }
    } catch { /* offline — use a temp id */ }

    // Fallback: generate a client-side ID
    const fallbackId = `local-${Date.now()}`;
    localStorage.setItem(SESSION_ID_KEY, fallbackId);
    setSessionId(fallbackId);
    return fallbackId;
  }, [sessionId]);

  // Save a message to the DB (fire-and-forget)
  const persistMessage = useCallback(async (msg: ChatMessage, sid?: string) => {
    const id = sid || sessionId || localStorage.getItem(SESSION_ID_KEY);
    if (!id || id.startsWith("local-")) return; // don't persist local-only sessions

    try {
      await fetch(`/api/sessions/${id}/messages`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          role: msg.role,
          content: msg.content,
          toolCalls: msg.toolCalls || undefined,
        }),
      });
    } catch { /* offline */ }
  }, [sessionId]);

  // Load messages from a DB session
  const loadSession = useCallback(async (sid: string) => {
    try {
      // Fetch from sessions list to get the messages
      const res = await fetch("/api/sessions");
      if (res.ok) {
        const data = await res.json();
        const found = data.sessions?.find((s: any) => s.id === sid);
        if (found?.messages?.length) {
          const msgs: ChatMessage[] = found.messages.map((m: any) => ({
            role: m.role,
            content: m.content,
            toolCalls: m.toolCalls || undefined,
          }));
          setMessages(msgs);
          setSessionId(sid);
          localStorage.setItem(SESSION_ID_KEY, sid);
          return msgs;
        }
      }
    } catch { /* offline */ }
    return null;
  }, []);

  // Start a fresh session
  const newSession = useCallback(() => {
    setMessages([]);
    setSessionId(null);
    localStorage.removeItem(STORAGE_KEY);
    localStorage.removeItem(SESSION_ID_KEY);
    // Create new session
    ensureSession();
  }, [ensureSession]);

  // Clear current session
  const clearSession = useCallback(() => {
    setMessages([]);
    localStorage.removeItem(STORAGE_KEY);
  }, []);

  return {
    sessionId,
    messages,
    setMessages,
    loaded,
    ensureSession,
    persistMessage,
    loadSession,
    newSession,
    clearSession,
  };
}

function getSessionTitle(): string {
  const now = new Date();
  return `Chat — ${now.toLocaleDateString()} ${now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
}
