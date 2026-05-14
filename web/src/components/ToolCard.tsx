"use client";

import {
  FileText, Pencil, Terminal, Search, Globe, Bot,
  CheckCircle2, XCircle, Loader2,
} from "lucide-react";
import type { ToolCallEntry } from "@/lib/types";

const TOOL_META: Record<string, { icon: typeof FileText; color: string }> = {
  Read: { icon: FileText, color: "text-green" },
  Write: { icon: Pencil, color: "text-rose" },
  Edit: { icon: Pencil, color: "text-rose" },
  Bash: { icon: Terminal, color: "text-amber" },
  Glob: { icon: Search, color: "text-cyan" },
  Grep: { icon: Search, color: "text-cyan" },
  WebSearch: { icon: Globe, color: "text-cyan" },
  WebFetch: { icon: Globe, color: "text-cyan" },
  Agent: { icon: Bot, color: "text-amber-light" },
};

function summarizeInput(input: Record<string, unknown>): string {
  const e = Object.entries(input).slice(0, 2);
  return e.map(([k, v]) => {
    const s = typeof v === "string" ? v : JSON.stringify(v);
    return `${k}=${s.slice(0, 40)}`;
  }).join("  ");
}

export function ToolCard({ tool }: { tool: ToolCallEntry }) {
  const meta = TOOL_META[tool.name] || { icon: Bot, color: "text-amber-light" };
  const Icon = meta.icon;

  const statusIcon = tool.isError ? (
    <XCircle className="h-3.5 w-3.5 text-rose" />
  ) : tool.result ? (
    <CheckCircle2 className="h-3.5 w-3.5 text-green" />
  ) : (
    <Loader2 className="h-3.5 w-3.5 animate-spin text-amber" />
  );

  return (
    <div className="mb-2 ml-6 border border-border bg-alt px-3 py-2">
      <div className="flex items-center gap-2">
        {statusIcon}
        <Icon className={`h-3.5 w-3.5 ${meta.color}`} />
        <span className={`text-xs font-medium ${meta.color}`}>{tool.name}</span>
        <span className="text-xs text-fg-faint">{summarizeInput(tool.input)}</span>
      </div>
      {tool.result && tool.result.length > 0 && (
        <div className="mt-1 ml-10 text-xs text-fg-faint line-clamp-2 font-mono">
          {tool.result.slice(0, 100).replace(/\n/g, " ")}
          {tool.result.length > 100 && "…"}
        </div>
      )}
    </div>
  );
}
