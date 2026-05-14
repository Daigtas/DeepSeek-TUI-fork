"use client";

import { Activity, Cpu, Clock, Layers, HardDrive } from "lucide-react";

interface StatusBarProps {
  isStreaming: boolean;
  contextUsed: number;
  contextLimit: number;
  roundCount: number;
  sessionDuration: number;
  model?: string;
  cost?: string;
  wsConnected?: boolean;
  reconnectAttempt?: number;
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return m > 0 ? `${m}m ${s}s` : `${s}s`;
}

// Aligned with desktop DeepSeek TUI ContextBudget: 3-zone gauge
function ContextBar({ used, limit }: { used: number; limit: number }) {
  const pct = limit > 0 ? (used / limit) * 100 : 0;
  const remaining = limit - used;
  const remainingPct = limit > 0 ? (remaining / limit) * 100 : 100;

  // Desktop thresholds: warning at 35% remaining, critical at 25% remaining
  const zone = remainingPct <= 25 ? "critical" : remainingPct <= 35 ? "warning" : "ok";

  const barColor = zone === "critical" ? "bg-rose" : zone === "warning" ? "bg-amber" : "bg-green";
  const labelColor = zone === "critical" ? "text-rose" : zone === "warning" ? "text-amber" : "text-green";

  const tooltip = `${formatTokens(used)} / ${formatTokens(limit)} used · ${formatTokens(remaining)} remaining (${Math.round(remainingPct)}%)`;

  return (
    <div className="flex items-center gap-1.5" title={tooltip}>
      <HardDrive className={`h-3 w-3 shrink-0 ${labelColor}`} />
      <div className="h-1.5 w-12 sm:w-20 bg-bg rounded-full overflow-hidden">
        <div
          className={`h-full transition-all duration-500 rounded-full ${barColor}`}
          style={{ width: `${Math.min(pct, 100)}%` }}
        />
      </div>
      <span className={`text-[11px] tabular-nums ${labelColor}`}>{formatTokens(used)}</span>
      {zone !== "ok" && (
        <span className={`hidden sm:inline text-[10px] font-semibold ${labelColor}`}>
          {zone === "critical" ? "CRIT" : "WARN"}
        </span>
      )}
    </div>
  );
}

export function StatusBar({
  isStreaming, contextUsed, contextLimit, roundCount,
  sessionDuration, model = "deepseek-v4-pro", cost,
  wsConnected = true, reconnectAttempt = 0,
}: StatusBarProps) {
  return (
    <footer className="flex flex-col max-[400px]:gap-0.5 sm:flex-row sm:items-center gap-2 sm:gap-3 border-t border-border bg-alt px-2 sm:px-4 py-1.5 text-[11px] text-fg-faint overflow-x-auto whitespace-nowrap" role="contentinfo" aria-label="Status bar">
      {/* Row 1: Always-visible essentials (streaming + model) */}
      <div className="flex items-center gap-2 shrink-0">
        {/* Streaming */}
        <div className="flex items-center gap-1 shrink-0" style={{ minWidth: "44px", minHeight: "24px" }} aria-label={isStreaming ? "Streaming in progress" : "Idle"}>
          <span className={`inline-block h-1.5 w-1.5 ${isStreaming ? "bg-green animate-pulse" : "bg-fg-faint"}`} />
          <span className="hidden sm:inline">{isStreaming ? "Streaming" : "Idle"}</span>
        </div>

        {/* WS Connection */}
        <div className="flex items-center gap-1 shrink-0" title={wsConnected ? "Connected" : `Disconnected${reconnectAttempt > 0 ? ` (retry ${reconnectAttempt}/5)` : ""}`} aria-label={wsConnected ? "WebSocket connected" : reconnectAttempt > 0 ? `WebSocket reconnecting (attempt ${reconnectAttempt} of 5)` : "WebSocket disconnected"}>
          <span className={`inline-block h-1.5 w-1.5 rounded-full ${wsConnected ? "bg-green" : reconnectAttempt > 0 ? "bg-amber animate-pulse" : "bg-rose"}`} />
          <span className="hidden sm:inline text-[10px]">{wsConnected ? "WS" : reconnectAttempt > 0 ? `R${reconnectAttempt}` : "Off"}</span>
        </div>

        {/* Model — always visible on mobile too */}
        <div className="flex items-center gap-1 shrink-0" aria-label={`Model: ${model}`}>
          <Cpu className="h-3 w-3" />
          <span className="text-amber-dim text-[10px] sm:text-[11px]">{model}</span>
        </div>
      </div>

      {/* Row 2: Secondary info, hidden on smallest screens */}
      <div className="flex items-center gap-2 sm:gap-3 max-[400px]:hidden">
        {/* Context — 3-zone gauge */}
        <ContextBar used={contextUsed} limit={contextLimit} />

        {/* Rounds */}
        {roundCount > 0 && (
          <div className="hidden sm:flex items-center gap-1 shrink-0" style={{ minHeight: "24px" }} aria-label={`${roundCount} rounds`}>
            <Layers className="h-3 w-3" />
            <span>{roundCount}r</span>
          </div>
        )}

        {/* Duration */}
        <div className="flex items-center gap-1 shrink-0" style={{ minHeight: "24px" }} aria-label={`Session duration: ${formatDuration(sessionDuration)}`}>
          <Clock className="h-3 w-3" />
          <span className="text-[10px] sm:text-[11px]">{formatDuration(sessionDuration)}</span>
        </div>

        {/* Cost */}
        {cost && (
          <div className="hidden sm:flex items-center gap-1 shrink-0" aria-label={`Estimated cost: ${cost}`}>
            <span>{cost}</span>
          </div>
        )}
      </div>
    </footer>
  );
}
