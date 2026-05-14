// Shared types for the TUI Web interface

export interface ToolCallEntry {
  id: string;
  name: string;
  input: Record<string, unknown>;
  result?: string;
  isError?: boolean;
}

export interface ChatMessage {
  role: "user" | "assistant" | "tool" | "error";
  content: string;
  toolCalls?: ToolCallEntry[];
  attachments?: FileAttachment[];
  _midTask?: boolean; // indicates this was a mid-task correction
}

export interface PermissionRequest {
  toolName: string;
  description: string;
  details: string;
  risk: "low" | "medium" | "high";
}

export interface SessionSummary {
  id: string;
  title: string;
  updatedAt: string;
  messageCount: number;
}

// ── User & Profile ──────────────────────────────────────────────────────

export interface UserProfile {
  id: string;
  name: string;
  email: string;
  image?: string | null;
  bio?: string | null;
  emailVerified: boolean;
  createdAt: string;
}

// ── User Preferences (settings) ──────────────────────────────────────────

export interface UserPreferences {
  agentMode: "agent" | "plan" | "yolo" | "agency";
  agencyRole?: string; // agency role override
  model: string;
  contextLimit: number;
  theme: "dark" | "light" | "system";
  temperature: number;
  maxTokens: number;
  autoApprove: boolean;
  markdownRender: boolean;
  syntaxHighlight: boolean;
  hooks?: {
    systemPromptExtension?: string;
    customInstructions?: string;
    enabledPlugins?: string[];
  } | null;
}

export const DEFAULT_PREFERENCES: UserPreferences = {
  agentMode: "agent",
  model: "deepseek-v4-pro",
  contextLimit: 128000,
  theme: "dark",
  temperature: 0.7,
  maxTokens: 32768,
  autoApprove: true,
  markdownRender: true,
  syntaxHighlight: true,
};

// ── Available models ────────────────────────────────────────────────────

export const AVAILABLE_MODELS = [
  { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro", desc: "Latest flagship — 1M context" },
  { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", desc: "Fast, cost-effective — 128K context" },
  { id: "deepseek-chat", name: "DeepSeek Chat", desc: "General purpose — 64K context" },
  { id: "deepseek-reasoner", name: "DeepSeek Reasoner", desc: "Deep reasoning — 64K context" },
] as const;

// ── Context limits by model ─────────────────────────────────────────────

export const MODEL_CONTEXT_LIMITS: Record<string, number> = {
  "deepseek-v4-pro": 1_000_000,
  "deepseek-v4-flash": 128_000,
  "deepseek-chat": 64_000,
  "deepseek-reasoner": 64_000,
};

// ── File Attachments ────────────────────────────────────────────────────

export interface FileAttachment {
  id: string;
  fileName: string;
  fileSize: number;
  mimeType: string;
  content?: string;   // base64 for small files
  storagePath?: string; // server path for large files
  preview?: string;    // client-side preview URL
}

// ── WebSocket Protocol ──────────────────────────────────────────────────

export interface WSChatMessage {
  type: "chat";
  prompt: string;
  mode?: "agent" | "plan" | "yolo";
  attachments?: FileAttachment[];
  midTask?: boolean; // true when this is a mid-task correction
}

export interface WSAbortMessage {
  type: "abort";
}

export interface WSTextDelta {
  type: "text";
  text: string;
}

export interface WSToolCall {
  type: "tool_call";
  name: string;
  input: Record<string, unknown>;
}

export interface WSToolResult {
  type: "tool_result";
  name: string;
  content: string;
  isError?: boolean;
}

export interface WSDone {
  type: "done";
  text: string;
  tokens: number;
  toolCalls: number;
}

export interface WSError {
  type: "error";
  message: string;
}

export type WSClientMessage = WSChatMessage | WSAbortMessage;
export type WSServerMessage = WSTextDelta | WSToolCall | WSToolResult | WSDone | WSError;
