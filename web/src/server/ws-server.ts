/**
 * WebSocket server — bridges browser clients to DeepSeek AI.
 *
 * Architecture:
 *   Browser (port 3100) ←→ WS server (port 3101) ←→ DeepSeek API
 *
 * Features:
 *   - Streaming SSE responses from DeepSeek API
 *   - Per-user preferences (model, mode, context limit) from DB
 *   - Mid-prompt interruption: abort, capture partial, inject correction, restart
 *   - DAEMONIZED TASKS: tasks continue executing even if the client disconnects.
 *     Results are persisted to the session's messages in the database.
 *     When the client reconnects, it receives any messages that accumulated
 *     while it was away.
 */
import { WebSocketServer, WebSocket } from "ws";
import { IncomingMessage } from "http";
import { PrismaClient } from "@prisma/client";

const WS_PORT = parseInt(process.env.WS_PORT || "3101");
const DEEPSEEK_API_KEY = process.env.DEEPSEEK_API_KEY || "";
const DEEPSEEK_BASE_URL = process.env.DEEPSEEK_BASE_URL || "https://api.deepseek.com/beta";
const FALLBACK_MODEL = process.env.DEEPSEEK_MODEL || "deepseek-v4-pro";
const FALLBACK_CONTEXT_LIMIT = parseInt(process.env.DEEPSEEK_CONTEXT_LIMIT || "128000");
const DATABASE_URL = process.env.DATABASE_URL || "";

let prisma: PrismaClient | null = null;
function getPrisma(): PrismaClient {
  if (!prisma) prisma = new PrismaClient({ datasourceUrl: DATABASE_URL });
  return prisma;
}

// ── Types ───────────────────────────────────────────────────────────────

interface UserPrefs {
  agentMode: string;
  model: string;
  contextLimit: number;
}

interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

interface ClientState {
  ws: WebSocket | null;
  userId: string | null;
  sessionId: string | null;
  prefs: UserPrefs;
  hooks: HooksConfig;
  currentAbort: AbortController | null;
  conversation: ChatMessage[];
  partialResponse: string;
  streaming: boolean;
  taskRunning: boolean;
}

// Global client states — survive WebSocket disconnects
const clients = new Map<string, ClientState>();

// ── Auth ────────────────────────────────────────────────────────────────

function parseBetterAuthCookie(req: IncomingMessage): string | null {
  const cookieHeader = req.headers.cookie;
  if (!cookieHeader) return null;
  for (const cookie of cookieHeader.split(";").map((c) => c.trim())) {
    const [name, ...rest] = cookie.split("=");
    const value = rest.join("=");
    if (name === "better-auth.session_token" || name === "__Host-better-auth.session_token" || name.endsWith("session_token")) {
      return decodeURIComponent(value);
    }
  }
  return null;
}

async function validateSession(token: string): Promise<string | null> {
  try {
    const session = await getPrisma().session.findFirst({
      where: { id: token, expiresAt: { gt: new Date() } },
      select: { userId: true },
    });
    return session?.userId ?? null;
  } catch { return null; }
}

async function loadUserPrefs(userId: string): Promise<UserPrefs & { hooks?: HooksConfig }> {
  try {
    const prefs = await getPrisma().userPreferences.findUnique({ where: { userId } });
    if (prefs) return {
      agentMode: prefs.agentMode, model: prefs.model, contextLimit: prefs.contextLimit,
      hooks: parseHooks(prefs.hooks),
    };
  } catch (err) {
    console.error(`[ws] failed to load preferences:`, (err as Error).message);
  }
  return { agentMode: "agent", model: FALLBACK_MODEL, contextLimit: FALLBACK_CONTEXT_LIMIT, hooks: DEFAULT_HOOKS };
}

// ── DB persistence ─────────────────────────────────────────────────────

async function persistMessageToDb(userId: string, sessionId: string, role: string, content: string) {
  if (!sessionId || sessionId.startsWith("local-")) return;
  try {
    await getPrisma().chatMessage.create({
      data: { userId, sessionId, role, content },
    });
    await getPrisma().chatSession.update({
      where: { id: sessionId },
      data: { updatedAt: new Date() },
    });
  } catch (err) {
    console.error(`[ws] failed to persist message:`, (err as Error).message);
  }
}

async function loadSessionMessages(sessionId: string): Promise<ChatMessage[]> {
  if (!sessionId || sessionId.startsWith("local-")) return [];
  try {
    const messages = await getPrisma().chatMessage.findMany({
      where: { sessionId },
      orderBy: { createdAt: "asc" },
      select: { role: true, content: true },
      take: 100,
    });
    return messages.map(m => ({ role: m.role as ChatMessage["role"], content: m.content }));
  } catch (err) {
    console.error("[ws] loadSessionMessages failed:", err instanceof Error ? err.message : String(err));
    return [];
  }
}

// ── Hooks system ──────────────────────────────────────────────────────

interface HooksConfig {
  systemPromptExtension?: string;
  prePromptHooks?: string[];
  postResponseHooks?: string[];
  enabledPlugins?: string[];
  customInstructions?: string;
}

const DEFAULT_HOOKS: HooksConfig = {
  enabledPlugins: ["code-review", "file-search", "command-exec"],
  systemPromptExtension: "",
};

function parseHooks(raw: unknown): HooksConfig {
  if (!raw || typeof raw !== "object") return DEFAULT_HOOKS;
  const h = raw as Record<string, unknown>;
  return {
    systemPromptExtension: typeof h.systemPromptExtension === "string" ? h.systemPromptExtension : "",
    prePromptHooks: Array.isArray(h.prePromptHooks) ? h.prePromptHooks.filter((x): x is string => typeof x === "string") : [],
    postResponseHooks: Array.isArray(h.postResponseHooks) ? h.postResponseHooks.filter((x): x is string => typeof x === "string") : [],
    enabledPlugins: Array.isArray(h.enabledPlugins) ? h.enabledPlugins.filter((x): x is string => typeof x === "string") : DEFAULT_HOOKS.enabledPlugins,
    customInstructions: typeof h.customInstructions === "string" ? h.customInstructions : "",
  };
}

// ── System prompt (with hooks) ────────────────────────────────────────

function buildSystemPrompt(mode: string, hooks?: HooksConfig): string {
  let base = "";
  switch (mode) {
    case "agency": {
      // Dynamic import to avoid bundling agency engine at startup
      const { buildAgencyPrompt } = require("../lib/agency/engine");
      base = buildAgencyPrompt({
        role: "pm",
        showTeam: true,
        task: "Analyze the user's request and delegate to the appropriate team members",
      });
      break;
    }
    case "yolo":
      base = "You are DeepSeek TUI, an AI coding assistant running in YOLO mode. You auto-approve all tool calls without asking the user. Execute tasks freely and efficiently.";
      break;
    case "plan":
      base = "You are DeepSeek TUI, an AI coding assistant running in PLAN mode. Before taking any action, research the problem thoroughly and outline your approach. Think step by step, then execute.";
      break;
    default:
      base = "You are DeepSeek TUI, an AI coding assistant. You have access to tools. Use them when needed. Provide clear, concise responses.";
  }

  // Apply hooks
  if (hooks) {
    if (hooks.customInstructions) {
      base += `\n\n[CUSTOM INSTRUCTIONS]\n${hooks.customInstructions}`;
    }
    if (hooks.systemPromptExtension) {
      base += `\n\n[SYSTEM EXTENSION]\n${hooks.systemPromptExtension}`;
    }
    if (hooks.enabledPlugins?.length) {
      base += `\n\n[ENABLED PLUGINS: ${hooks.enabledPlugins.join(", ")}]`;
    }
  }

  return base;
}

// ── Stream engine (daemonized — survives disconnect) ──────────────────

async function runStream(state: ClientState, prefs: UserPrefs): Promise<void> {
  const { conversation, userId, sessionId } = state;
  state.currentAbort = new AbortController();
  const signal = state.currentAbort.signal;
  state.partialResponse = "";
  state.streaming = true;
  state.taskRunning = true;

  if (!DEEPSEEK_API_KEY || DEEPSEEK_API_KEY === "sk-placeholder") {
    sendToClient(state, { type: "error", message: "API key not configured" });
    state.streaming = false;
    state.taskRunning = false;
    return;
  }

  try {
    const response = await fetch(`${DEEPSEEK_BASE_URL}/v1/chat/completions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${DEEPSEEK_API_KEY}`,
      },
      body: JSON.stringify({
        model: prefs.model,
        messages: conversation,
        max_tokens: 32768,
        temperature: 0.7,
        stream: true,
      }),
      signal,
    });

    if (!response.ok) {
      const errText = await response.text();
      throw new Error(`API error ${response.status}: ${errText.substring(0, 200)}`);
    }

    const reader = response.body?.getReader();
    if (!reader) throw new Error("No response body");

    const decoder = new TextDecoder();
    let buffer = "";

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });

      const lines = buffer.split("\n");
      buffer = lines.pop() || "";

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed || !trimmed.startsWith("data: ")) continue;
        const data = trimmed.slice(6);
        if (data === "[DONE]") continue;

        try {
          const parsed = JSON.parse(data);
          const delta = parsed.choices?.[0]?.delta;
          if (delta?.content) {
            state.partialResponse += delta.content;
            // Stream to client if connected — otherwise just accumulate
            sendToClient(state, { type: "text", text: delta.content });
          }
        } catch { /* skip malformed SSE */ }
      }
    }

    // Task completed — persist full response
    conversation.push({ role: "assistant", content: state.partialResponse });

    if (userId && sessionId) {
      persistMessageToDb(userId, sessionId, "assistant", state.partialResponse);
    }

    sendToClient(state, {
      type: "done",
      text: state.partialResponse,
      tokens: state.partialResponse.length,
      toolCalls: 0,
    });
  } catch (err) {
    if ((err as Error).name === "AbortError") {
      // Stream aborted (mid-task interruption) — don't error
      return;
    }
    console.error(`[ws] stream error:`, (err as Error).message);
    sendToClient(state, {
      type: "error",
      message: (err as Error).message || "Stream failed",
    });
  } finally {
    state.streaming = false;
    state.taskRunning = false;
    state.currentAbort = null;
  }
}

/** Send to client WebSocket if connected. No-op if disconnected. */
function sendToClient(state: ClientState, msg: Record<string, unknown>) {
  if (state.ws?.readyState === WebSocket.OPEN) {
    state.ws.send(JSON.stringify(msg));
  }
}

// ── Server ──────────────────────────────────────────────────────────────

export function startWSServer(): WebSocketServer {
  const wss = new WebSocketServer({ port: WS_PORT });

  console.log(`[ws] WebSocket → ws://0.0.0.0:${WS_PORT}`);
  console.log(`[ws] API: ${DEEPSEEK_BASE_URL}/v1/chat/completions`);
  console.log(`[ws] Tasks: DAEMONIZED — survive client disconnects`);

  wss.on("connection", async (ws, req) => {
    let userId: string | null = null;
    const isLocalhost = req.socket.remoteAddress === "::ffff:127.0.0.1" ||
                        req.socket.remoteAddress === "127.0.0.1" ||
                        req.socket.remoteAddress === "::1";
    const isProxied = req.socket.remoteAddress?.startsWith("::ffff:10.") ||
                      req.socket.remoteAddress?.startsWith("10.");

    // Auth
    if (isLocalhost) {
      // no auth needed
    } else if (isProxied && DATABASE_URL) {
      const token = parseBetterAuthCookie(req);
      if (token) userId = await validateSession(token);
    } else if (DATABASE_URL) {
      const token = parseBetterAuthCookie(req);
      if (token) userId = await validateSession(token);
      if (!userId) {
        ws.close(4401, "Unauthorized");
        return;
      }
    }

    // Look up or create client state
    const clientKey = userId || `anon-${req.socket.remoteAddress}`;
    let state = clients.get(clientKey);

    if (!state) {
      const loadedPrefs = userId ? await loadUserPrefs(userId) : {
        agentMode: "agent", model: FALLBACK_MODEL, contextLimit: FALLBACK_CONTEXT_LIMIT, hooks: DEFAULT_HOOKS,
      };
      state = {
        ws, userId, sessionId: null,
        prefs: { agentMode: loadedPrefs.agentMode, model: loadedPrefs.model, contextLimit: loadedPrefs.contextLimit },
        hooks: loadedPrefs.hooks || DEFAULT_HOOKS,
        currentAbort: null,
        conversation: [],
        partialResponse: "",
        streaming: false,
        taskRunning: false,
      };
      clients.set(clientKey, state);
    } else {
      // Reconnecting client — attach new WebSocket
      state.ws = ws;
    }

    console.log(`[ws] ${state.ws === ws ? "new" : "reconnect"} | user=${clientKey} | tasks=${state.taskRunning ? "running" : "idle"}`);

    // If a task is running, send current status
    if (state.taskRunning) {
      ws.send(JSON.stringify({ type: "status", message: "Task is still running...", partial: state.partialResponse.length }));
    }

    // Replay any partial response accumulated while disconnected
    if (state.partialResponse && !state.streaming) {
      // Task completed while disconnected — send full response
      ws.send(JSON.stringify({ type: "text", text: state.partialResponse }));
      ws.send(JSON.stringify({ type: "done", text: state.partialResponse, tokens: state.partialResponse.length, toolCalls: 0 }));
      state.partialResponse = "";
    } else if (state.partialResponse && state.streaming) {
      // Task still running — send what we have so far
      ws.send(JSON.stringify({ type: "text", text: state.partialResponse }));
    }

    ws.on("message", async (raw) => {
      let msg: { type: string; prompt?: string; mode?: string; midTask?: boolean; attachments?: unknown[]; sessionId?: string };
      try { msg = JSON.parse(raw.toString()); } catch {
        ws.send(JSON.stringify({ type: "error", message: "Invalid JSON" }));
        return;
      }

      if (msg.type === "ping") { ws.send(JSON.stringify({ type: "pong" })); return; }
      if (msg.type === "abort") {
        state!.currentAbort?.abort();
        state!.conversation = [];
        return;
      }

      if (msg.type === "restore") {
        // Client wants to restore a session
        if (msg.sessionId) {
          state!.sessionId = msg.sessionId;
          const dbMessages = await loadSessionMessages(msg.sessionId);
          if (dbMessages.length > 0) {
            ws.send(JSON.stringify({ type: "session_loaded", messages: dbMessages }));
            state!.conversation = dbMessages;
          }
        }
        return;
      }

      if (msg.type !== "chat" || !msg.prompt) {
        ws.send(JSON.stringify({ type: "error", message: 'Expected { type: "chat", prompt: "..." }' }));
        return;
      }

      // Refresh prefs
      if (userId) state!.prefs = await loadUserPrefs(userId);
      if (msg.mode && ["agent", "plan", "yolo"].includes(msg.mode)) {
        state!.prefs.agentMode = msg.mode;
      }

      let userPrompt = msg.prompt;

      // Attachments
      if (msg.attachments?.length) {
        const refs = (msg.attachments as Record<string, unknown>[]).map(
          (a) => `[Attached: ${a.fileName}]`
        ).join("\n");
        userPrompt = `${userPrompt}\n\n[ATTACHED FILES]\n${refs}`;
      }

      // Persist user message to DB
      if (userId && state!.sessionId) {
        persistMessageToDb(userId, state!.sessionId!, "user", userPrompt);
      }

      // ── MID-TASK INTERRUPTION ──────────────────────────────────────
      if (msg.midTask && state!.streaming) {
        console.log(`[ws] mid-task | partial=${state!.partialResponse.length} chars | "${userPrompt.substring(0, 80)}"`);
        state!.currentAbort?.abort();
        state!.streaming = false;
        if (state!.partialResponse) {
          state!.conversation.push({ role: "assistant", content: state!.partialResponse });
        }
        state!.conversation.push({ role: "user", content: userPrompt });
        ws.send(JSON.stringify({ type: "stream_reset" }));
        runStream(state!, state!.prefs);
        return;
      }

      // ── NEW TASK ───────────────────────────────────────────────────
      const systemPrompt = buildSystemPrompt(state!.prefs.agentMode, state!.hooks);
      state!.conversation = [
        { role: "system", content: systemPrompt },
        { role: "user", content: userPrompt },
      ];

      console.log(`[ws] task | user=${clientKey} | mode=${state!.prefs.agentMode} | "${userPrompt.substring(0, 100)}"`);

      // Start task — runs asynchronously, survives disconnect
      runStream(state!, state!.prefs);
    });

    ws.on("close", () => {
      // Don't delete state — task may still be running
      state!.ws = null;
      console.log(`[ws] disconnected | user=${clientKey} | tasks=${state!.taskRunning ? "STILL RUNNING" : "idle"}`);
      // Clean up idle clients after 5 minutes
      if (!state!.taskRunning) {
        setTimeout(() => {
          if (!state!.taskRunning && !state!.ws) {
            clients.delete(clientKey);
            console.log(`[ws] cleaned up idle client: ${clientKey}`);
          }
        }, 5 * 60 * 1000);
      }
    });
  });

  return wss;
}

const isMain = process.argv[1]?.includes("ws-server");
if (isMain) startWSServer();
