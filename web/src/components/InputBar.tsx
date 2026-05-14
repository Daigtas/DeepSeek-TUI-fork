"use client";

import { useState, useRef, useEffect, useCallback, KeyboardEvent, ClipboardEvent } from "react";
import { Send, Square, Edit3 } from "lucide-react";
import { COMMAND_IDS } from "@/lib/commands";

interface InputBarProps {
  onSubmit: (value: string) => void;
  disabled?: boolean;
  placeholder?: string;
  onOpenPalette?: () => void;
  onAbort?: () => void;
  midTaskMode?: boolean;
  onPasteFiles?: (files: File[]) => void;
  onSlashCommand?: (command: string, args: string) => void;
}

export function InputBar({ onSubmit, disabled, placeholder, onOpenPalette, onAbort, midTaskMode, onPasteFiles, onSlashCommand }: InputBarProps) {
  const [value, setValue] = useState("");
  const [activeMatchIndex, setActiveMatchIndex] = useState(0);
  const [sendPressed, setSendPressed] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const isSlash = value.trim().startsWith("/");
  const slashCommand = isSlash ? value.trim().match(/^\/([a-zA-Z_-]+)/)?.[1] : null;
  const matchingCommands = isSlash ? COMMAND_IDS.filter((c) => c.startsWith(value.slice(1))) : [];

  const triggerScalePress = () => {
    setSendPressed(true);
    setTimeout(() => setSendPressed(false), 200);
  };

  const handleSubmit = () => {
    const trimmed = value.trim();
    if (!trimmed || disabled) return;
    if (isSlash && slashCommand && onSlashCommand) {
      const args = value.trim().slice(slashCommand.length + 1).trim();
      onSlashCommand(slashCommand, args);
      setValue("");
      triggerScalePress();
      return;
    }
    onSubmit(trimmed);
    setValue("");
    triggerScalePress();
  };

  const handlePaste = (e: ClipboardEvent<HTMLTextAreaElement>) => {
    if (!disabled && onPasteFiles) {
      const items = e.clipboardData?.items;
      if (items) {
        const files: File[] = [];
        for (let i = 0; i < items.length; i++) {
          if (items[i].kind === "file") {
            const file = items[i].getAsFile();
            if (file) files.push(file);
          }
        }
        if (files.length > 0) {
          e.preventDefault();
          onPasteFiles(files);
          return;
        }
      }
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); handleSubmit(); return; }
    if (e.key === "Tab" && isSlash && matchingCommands.length > 0) {
      e.preventDefault();
      const cmd = matchingCommands[activeMatchIndex] ?? matchingCommands[0];
      setValue("/" + cmd + " ");
      return;
    }
    if (e.key === "ArrowDown" && isSlash && matchingCommands.length > 1) {
      e.preventDefault();
      setActiveMatchIndex((prev) => (prev + 1) % matchingCommands.length);
      return;
    }
    if (e.key === "ArrowUp" && isSlash && matchingCommands.length > 1) {
      e.preventDefault();
      setActiveMatchIndex((prev) => (prev - 1 + matchingCommands.length) % matchingCommands.length);
      return;
    }
    if (e.key === "Escape") {
      if (value !== "") { setValue(""); return; }
      onOpenPalette?.(); return;
    }
    if (e.key === "/" && value === "") { onOpenPalette?.(); return; }
  };

  const adjustHeight = () => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = "auto";
      // Auto-grow up to 8 rows
      const lineHeight = parseInt(getComputedStyle(textarea).lineHeight) || 22;
      const maxH = Math.min(textarea.scrollHeight, lineHeight * 8);
      textarea.style.height = maxH + "px";
    }
  };

  useEffect(() => {
    adjustHeight();
  }, [value]);

  return (
    <div>
      {slashCommand && (
        <div className="mb-2 flex items-center gap-2 text-xs animate-fade-in">
          <span className="bg-amber/10 text-amber font-mono px-2 py-0.5 border border-amber/15 animate-bounce-in">
            /{slashCommand}
          </span>
          <span className="text-fg-faint">
            Tab to autocomplete{matchingCommands.length > 1 && ` (${matchingCommands.length} matches)`}
          </span>
        </div>
      )}

      {/* Input wrapper — no orange outline, subtle border that dims on focus */}
      <div className={`flex items-start gap-2.5 rounded-md border bg-card px-3 py-2.5 transition-colors ${
        midTaskMode
          ? "border-green/40 bg-green/[0.02] ring-1 ring-green/20"
          : "border-border hover:border-border/80 focus-within:border-amber/40"
      }`}>
        {/* Prompt indicator */}
        {midTaskMode ? (
          <Edit3 className="mt-1 h-4 w-4 shrink-0 text-green" />
        ) : (
          <span className="mt-1 shrink-0 select-none text-amber font-mono font-bold text-sm">❯</span>
        )}

        {/* Resizable textarea */}
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => { setValue(e.target.value); adjustHeight(); }}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          disabled={disabled}
          placeholder={placeholder || (midTaskMode ? "Send correction..." : "Ask anything… / for commands")}
          rows={2}
          className="min-h-[2.5rem] flex-1 resize-none overflow-y-auto bg-transparent text-base sm:text-sm leading-relaxed text-fg placeholder:text-fg-faint/60 outline-none"
          style={{ minHeight: "2.5rem" }}
          aria-label="Message input"
          aria-describedby="input-help-text"
        />

        {/* Action button */}
        <div className="flex shrink-0 items-start pt-0.5">
          {disabled && onAbort && !midTaskMode ? (
            <button
              onClick={onAbort}
              className="flex h-11 w-11 items-center justify-center rounded text-rose/80 hover:text-rose hover:bg-rose/5 transition-colors"
              title="Stop generation"
              aria-label="Stop generation"
            >
              <Square className="h-5 w-5" />
            </button>
          ) : (
            <button
              onClick={handleSubmit}
              disabled={(!midTaskMode && disabled) || !value.trim()}
              className={`flex h-11 w-11 items-center justify-center rounded transition-colors ${
                midTaskMode
                  ? "text-green/80 hover:text-green hover:bg-green/5"
                  : "text-fg-faint/60 hover:text-amber hover:bg-amber/5"
              } disabled:opacity-20 disabled:cursor-not-allowed ${
                sendPressed ? "animate-scale-press" : ""
              }`}
              title={midTaskMode ? "Send correction" : "Send message"}
              aria-label={midTaskMode ? "Send correction" : "Send message"}
            >
              <Send className="h-5 w-5" />
            </button>
          )}
        </div>
      </div>

      {/* Help text */}
      <div id="input-help-text" className="mt-1.5 flex justify-between px-1 text-[10px] text-fg-faint/50">
        <span>Enter to send · Shift+Enter for newline · / for commands</span>
        <span>{value.length > 0 ? `${value.length} chars` : ""}</span>
      </div>
    </div>
  );
}
