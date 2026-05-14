"use client";

import { useState } from "react";
import { Search, X, Slash } from "lucide-react";
import { COMMANDS } from "@/lib/commands";

interface CommandPaletteProps {
  onSelect: (command: string) => void;
  onClose: () => void;
}

export function CommandPalette({ onSelect, onClose }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);

  const filtered = COMMANDS.filter(
    (c) =>
      c.label.includes(query.toLowerCase()) ||
      c.desc.toLowerCase().includes(query.toLowerCase()),
  );

  // Reset active index when filter results change
  const safeActiveIndex = Math.min(activeIndex, Math.max(0, filtered.length - 1));
  const activeId = filtered.length > 0 ? `cmd-${filtered[safeActiveIndex]?.id}` : undefined;

  return (
    <div className="border border-amber/20 bg-card shadow-border">
      <div className="flex items-center gap-2 border-b border-border px-3 py-2">
        <Slash className="h-4 w-4 text-amber" />
        <input
          autoFocus
          value={query}
          onChange={(e) => { setQuery(e.target.value); setActiveIndex(0); }}
          placeholder="Search commands…"
          className="flex-1 bg-transparent text-sm text-fg outline-none placeholder:text-fg-faint"
          aria-label="Search commands"
          onKeyDown={(e) => {
            if (e.key === "Escape") { onClose(); return; }
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setActiveIndex((prev) => Math.min(prev + 1, filtered.length - 1));
              return;
            }
            if (e.key === "ArrowUp") {
              e.preventDefault();
              setActiveIndex((prev) => Math.max(prev - 1, 0));
              return;
            }
            if (e.key === "Enter" && filtered.length > 0) {
              const idx = Math.min(activeIndex, filtered.length - 1);
              onSelect(filtered[idx].id);
              onClose();
            }
          }}
        />
        <button onClick={onClose} className="text-fg-faint hover:text-fg" aria-label="Close command palette">
          <X className="h-3.5 w-3.5" />
        </button>
      </div>

      <div
        className="max-h-64 overflow-y-auto p-1"
        role="listbox"
        aria-label="Commands"
        aria-activedescendant={activeId ?? undefined}
      >
        {filtered.length === 0 ? (
          <p className="px-3 py-4 text-center text-xs text-fg-faint">No commands found</p>
        ) : (
          filtered.map((cmd, idx) => (
            <button
              key={cmd.id}
              id={`cmd-${cmd.id}`}
              role="option"
              aria-selected={idx === safeActiveIndex}
              onClick={() => { onSelect(cmd.id); onClose(); }}
              className={`flex w-full items-center gap-3 px-3 py-2 text-left transition-colors ${
                idx === safeActiveIndex ? "bg-amber/10" : "hover:bg-amber/5"
              }`}
            >
              <span className="text-sm font-mono text-amber">{cmd.label}</span>
              <span className="text-xs text-fg-faint">{cmd.desc}</span>
            </button>
          ))
        )}
      </div>
    </div>
  );
}
