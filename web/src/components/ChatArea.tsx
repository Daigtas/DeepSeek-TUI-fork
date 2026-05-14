"use client";

import React, { useRef, useState, useEffect, useCallback } from "react";
import { ChatBubble } from "./ChatBubble";
import { StreamingText } from "./StreamingText";
import { ToolCard } from "./ToolCard";
import { InputBar } from "./InputBar";
import { AttachmentBar } from "./AttachmentBar";
import ChatSkeleton from "./ChatSkeleton";
import { ChevronDown, Terminal, Command, FileText } from "lucide-react";
import type { ChatMessage, ToolCallEntry, FileAttachment, UserPreferences } from "@/lib/types";
import { DEFAULT_PREFERENCES } from "@/lib/types";

interface ChatAreaProps {
  messages: ChatMessage[];
  streamingText: string;
  currentTools: ToolCallEntry[];
  isStreaming: boolean;
  onSubmit: (text: string) => void;
  onOpenPalette: () => void;
  messagesEndRef: React.Ref<HTMLDivElement>;
  onAbort?: () => void;
  onSlashCommand?: (command: string, args?: string) => void;
  preferences?: UserPreferences;
  attachments?: FileAttachment[];
  onAddAttachments?: (files: FileAttachment[]) => void;
  onRemoveAttachment?: (id: string) => void;
  midTaskMode?: boolean;
  isLoading?: boolean;
  /** Called when the user clicks a suggestion chip in the empty state */
  onFillInput?: (text: string) => void;
}

export function ChatArea({
  messages, streamingText, currentTools, isStreaming,
  onSubmit, onOpenPalette, messagesEndRef, onAbort, onSlashCommand,
  preferences = DEFAULT_PREFERENCES,
  attachments = [],
  onAddAttachments,
  onRemoveAttachment,
  midTaskMode = false,
  isLoading = false,
  onFillInput,
}: ChatAreaProps) {
  const noop = React.useCallback(() => {}, []);

  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const [showScrollButton, setShowScrollButton] = useState(false);
  const [newMessagesBelow, setNewMessagesBelow] = useState(false);

  // Track whether user has scrolled up
  const handleScroll = useCallback(() => {
    const el = scrollContainerRef.current;
    if (!el) return;
    const { scrollTop, scrollHeight, clientHeight } = el;
    const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
    const isNearBottom = distanceFromBottom < 80;
    setShowScrollButton(!isNearBottom);
    if (isNearBottom) setNewMessagesBelow(false);
  }, []);

  // When new messages arrive while scrolled up, show notification
  useEffect(() => {
    if (showScrollButton && (messages.length > 0 || streamingText)) {
      setNewMessagesBelow(true);
    }
  }, [messages, streamingText]);

  // Auto-scroll to bottom when streaming (unless user scrolled up)
  useEffect(() => {
    if (isStreaming && !showScrollButton) {
      const ref = messagesEndRef;
      if (ref && typeof ref === "object" && "current" in ref) {
        ref.current?.scrollIntoView({ behavior: "smooth" });
      }
    }
  }, [streamingText, isStreaming, showScrollButton]);

  const scrollToBottom = useCallback(() => {
    const ref = messagesEndRef;
    if (ref && typeof ref === "object" && "current" in ref) {
      ref.current?.scrollIntoView({ behavior: "smooth" });
    }
    setShowScrollButton(false);
    setNewMessagesBelow(false);
  }, [messagesEndRef]);

  // ── Suggested actions for empty state ──────────────────────────────
  const suggestions = [
    { icon: Command, label: "Explain code", query: "Explain this code" },
    { icon: Terminal, label: "Debug this", query: "Debug this issue" },
    { icon: FileText, label: "Write a function", query: "Write a function that" },
  ];

  return (
    <div className="flex flex-1 flex-col min-h-0" aria-label="Chat area">
      {/* Messages — scrollable region */}
      <div
        ref={scrollContainerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto px-3 sm:px-4 py-3 sm:py-4 relative"
        role="log"
        aria-live="polite"
      >
        {/* Loading skeleton */}
        {isLoading && <ChatSkeleton />}

        {/* Empty state */}
        {!isLoading && !isStreaming && messages.length === 0 && (
          <div className="flex h-full items-center justify-center">
            <div className="text-center px-4 max-w-md animate-fade-in">
              <div className="text-3xl mb-4">
                <span className="text-amber font-mono">❯_</span>
              </div>
              <h2 className="text-lg font-bold text-fg mb-2">Ask anything</h2>
              <p className="text-sm text-fg-dim mb-5 leading-relaxed">
                Your AI coding assistant. Ask questions, run commands, or analyze code.
                Press <kbd className="text-amber font-mono text-xs">/</kbd> for the full command palette.
              </p>
              {/* Suggestion chips */}
              <div className="flex flex-wrap justify-center gap-2">
                {suggestions.map((s) => (
                  <button
                    key={s.label}
                    onClick={() => {
                      if (onFillInput) onFillInput(s.query);
                    }}
                    className="inline-flex items-center gap-1.5 rounded-full border border-border bg-card px-4 py-2 text-xs text-fg-dim hover:text-fg hover:border-amber/30 hover:bg-amber/5 transition-all active:scale-95"
                  >
                    <s.icon className="h-3.5 w-3.5 text-amber-dim" />
                    {s.label}
                  </button>
                ))}
              </div>
              <p className="mt-5 text-[10px] text-fg-faint">
                Drag & drop files, paste images, or type <kbd className="text-amber font-mono">/help</kbd> to get started.
              </p>
            </div>
          </div>
        )}

        {/* Messages */}
        {messages.map((msg, i) => (
          <ChatBubble key={`msg-${i}`} message={msg} preferences={preferences} />
        ))}

        {/* Mid-task correction banner */}
        {midTaskMode && isStreaming && (
          <div className="mb-3 rounded border border-green/20 bg-green/5 px-3 py-2 text-xs text-green animate-fade-in" role="status">
            <span className="font-semibold">Correction noted.</span> Agent is rethinking and continuing the task with your feedback.
          </div>
        )}

        {/* Streaming area */}
        {isStreaming && (
          <div className="mb-4 animate-fade-in">
            {currentTools.map((tc) => (
              <ToolCard key={tc.id} tool={tc} />
            ))}
            {streamingText ? (
              <StreamingText text={streamingText} markdownRender={preferences.markdownRender} isStreaming={isStreaming} />
            ) : currentTools.length > 0 ? (
              <div className="flex items-center gap-2 py-2 text-fg-faint text-xs sm:text-sm" aria-busy="true">
                <span className="inline-block h-1.5 w-1.5 animate-pulse bg-amber" />
                Working… {currentTools.filter(t => !t.result).length} tool{currentTools.filter(t => !t.result).length !== 1 ? 's' : ''} running
              </div>
            ) : (
              <div className="flex items-center gap-2 py-2 text-fg-faint text-xs sm:text-sm" aria-busy="true">
                <span className="flex items-center gap-0.5">
                  <span className="inline-block h-1.5 w-1.5 animate-dot-pulse bg-amber" style={{ animationDelay: "0ms" }} />
                  <span className="inline-block h-1.5 w-1.5 animate-dot-pulse bg-amber" style={{ animationDelay: "200ms" }} />
                  <span className="inline-block h-1.5 w-1.5 animate-dot-pulse bg-amber" style={{ animationDelay: "400ms" }} />
                </span>
                Thinking…
              </div>
            )}
          </div>
        )}

        <div ref={messagesEndRef} />

        {/* Scroll-to-bottom button */}
        {showScrollButton && (
          <button
            onClick={scrollToBottom}
            className="sticky bottom-0 left-1/2 -translate-x-1/2 mb-1 flex items-center justify-center w-8 h-8 rounded-full border border-amber/30 bg-amber/10 text-amber hover:bg-amber/20 transition-all animate-bounce-in shadow-lg"
            aria-label={newMessagesBelow ? "New messages — scroll to bottom" : "Scroll to bottom"}
          >
            <ChevronDown className="h-4 w-4" />
            {newMessagesBelow && (
              <span className="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full bg-amber animate-pulse" />
            )}
          </button>
        )}
      </div>

      {/* Attachments bar */}
      {!isStreaming && (
        <AttachmentBar
          attachments={attachments}
          onAttach={onAddAttachments || noop}
          onRemove={onRemoveAttachment || (() => {})}
        />
      )}

      {/* Input */}
      <div
        className={`border-t px-3 sm:px-4 py-2.5 sm:py-3 transition-colors duration-200 ${
          midTaskMode ? "border-green/20 bg-green/5" : "border-border"
        }`}
        style={{ paddingBottom: "calc(0.75rem + env(safe-area-inset-bottom))" }}
      >
        <InputBar
          onSubmit={onSubmit}
          disabled={isStreaming && !midTaskMode}
          placeholder={
            midTaskMode
              ? "Add correction or feedback to the running agent…"
              : isStreaming
              ? "Agent is working…"
              : "Ask anything… / for commands"
          }
          onOpenPalette={onOpenPalette}
          onAbort={onAbort}
          midTaskMode={midTaskMode}
          onSlashCommand={onSlashCommand}
        />
      </div>
    </div>
  );
}
