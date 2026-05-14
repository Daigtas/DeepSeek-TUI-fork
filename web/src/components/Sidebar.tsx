"use client";

import { LogOut, MessageSquare, Plus, Clock } from "lucide-react";
import type { SessionSummary } from "@/lib/types";

interface SidebarProps {
  isOpen: boolean;
  onToggle: () => void;
  onSignOut: () => void;
  session: { user?: { name?: string | null; email?: string | null; image?: string | null } };
  sessions?: SessionSummary[];
  activeSession?: string;
  onSessionSelect?: (id: string) => void;
}

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    const now = new Date();
    const diff = now.getTime() - d.getTime();
    if (diff < 3600000) return `${Math.round(diff / 60000)}m ago`;
    if (diff < 86400000) return `${Math.round(diff / 3600000)}h ago`;
    return d.toLocaleDateString();
  } catch { return ""; }
}

export function Sidebar({ isOpen, onSignOut, onToggle, session, sessions = [], activeSession, onSessionSelect }: SidebarProps) {
  return (
    <>
      {isOpen && (
        <div
          className="fixed inset-0 z-40 bg-black/50 md:hidden"
          onClick={onToggle}
          role="button"
          aria-label="Close sidebar"
        />
      )}

      <aside
        role="navigation"
        aria-label="Session sidebar"
        className={`
          shrink-0 border-r border-border bg-alt flex flex-col
          transition-transform duration-200
          fixed inset-y-0 left-0 z-50 w-full
          md:static md:z-auto md:w-60
          ${isOpen ? "translate-x-0" : "-translate-x-full md:translate-x-0"}
          ${!isOpen ? "md:w-0 md:overflow-hidden md:border-0" : ""}
        `}
      >
        {/* User */}
        <div className="border-b border-border p-4">
          <div className="flex items-center gap-3">
            <div className="flex h-7 w-7 shrink-0 items-center justify-center bg-amber/15 text-xs font-bold text-amber">
              {session.user?.name?.[0]?.toUpperCase() || "?"}
            </div>
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm text-fg">{session.user?.name || "User"}</p>
              <p className="truncate text-xs text-fg-faint">{session.user?.email}</p>
            </div>
          </div>
        </div>

        {/* New session button */}
        <div className="border-b border-border p-2">
          <button
            onClick={() => onSessionSelect?.("new")}
            className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-xs text-fg-dim hover:bg-green/5 hover:text-green transition-colors"
          >
            <Plus className="h-3.5 w-3.5" />
            New Session
          </button>
        </div>

        {/* Sessions */}
        <div className="flex-1 overflow-y-auto p-2">
          <p className="mb-2 px-2 text-[10px] font-semibold uppercase tracking-widest text-fg-faint">
            Sessions
          </p>
          {sessions.length === 0 ? (
            <p className="px-2 text-xs text-fg-faint/60">No saved sessions yet</p>
          ) : (
            sessions.map((s) => (
              <button
                key={s.id}
                onClick={() => onSessionSelect?.(s.id)}
                className={`flex w-full items-start gap-2 rounded px-2 py-1.5 text-left text-xs transition-colors group ${
                  activeSession === s.id
                    ? "bg-amber/5 text-amber border border-amber/10"
                    : "text-fg-dim hover:bg-card hover:text-fg"
                }`}
                aria-label={`Session: ${s.title}`}
                aria-current={activeSession === s.id ? "page" : undefined}
              >
                <MessageSquare className="h-3.5 w-3.5 mt-0.5 shrink-0 text-fg-faint" />
                <div className="min-w-0 flex-1">
                  <p className="truncate font-medium">{s.title}</p>
                  <div className="mt-0.5 flex items-center gap-2 text-[10px] text-fg-faint/60">
                    <span className="flex items-center gap-0.5">
                      <Clock className="h-2.5 w-2.5" />
                      {formatDate(s.updatedAt)}
                    </span>
                    {s.messageCount > 0 && (
                      <span>{s.messageCount} messages</span>
                    )}
                  </div>
                </div>
              </button>
            ))
          )}
        </div>

        {/* Footer */}
        <div className="border-t border-border p-3">
          <button
            onClick={onSignOut}
            className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-xs text-fg-dim hover:bg-rose/5 hover:text-rose transition-colors"
          >
            <LogOut className="h-3.5 w-3.5" />
            Sign out
          </button>
        </div>
      </aside>
    </>
  );
}
