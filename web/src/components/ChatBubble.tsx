"use client";

import React, { memo, useState, useCallback } from "react";
import { Terminal, User, Copy, Check } from "lucide-react";
import { ToolCard } from "./ToolCard";
import { ResponseRenderer } from "./ResponseRenderer";
import type { ChatMessage, UserPreferences } from "@/lib/types";
import { DEFAULT_PREFERENCES } from "@/lib/types";

interface ChatBubbleProps {
  message: ChatMessage;
  preferences?: UserPreferences;
}

export const ChatBubble = memo(function ChatBubble({ message, preferences = DEFAULT_PREFERENCES }: ChatBubbleProps) {
  const isUser = message.role === "user";
  const isMidTask = message._midTask === true;
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    if (!message.content) return;
    try {
      await navigator.clipboard.writeText(message.content);
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    } catch {
      // Fallback for older browsers
      const ta = document.createElement("textarea");
      ta.value = message.content;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
      setCopied(true);
      setTimeout(() => setCopied(false), 1800);
    }
  }, [message.content]);

  return (
    <div className={`mb-3 sm:mb-4 animate-slide-up-fade ${isUser ? "ml-auto" : "mr-auto"}`} tabIndex={0}>
      <div className={`mb-1 flex items-center gap-2 ${isUser ? "justify-end" : ""}`}>
        {isUser ? (
          <>
            <User className="h-3.5 w-3.5 text-amber" />
            <span className="text-xs font-semibold text-amber">
              {isMidTask ? "Correction" : "You"}
            </span>
          </>
        ) : (
          <>
            <Terminal className="h-3.5 w-3.5 text-green" />
            <span className="text-xs font-semibold text-green">DeepSeek</span>
            {/* Copy button — only on assistant messages with content */}
            {message.content && (
              <button
                onClick={handleCopy}
                className={`ml-auto rounded p-1 transition-colors ${
                  copied
                    ? "text-green bg-green/10"
                    : "text-fg-faint/40 hover:text-fg-faint hover:bg-bg-hover"
                }`}
                title={copied ? "Copied!" : "Copy message"}
                aria-label={copied ? "Copied to clipboard" : "Copy message to clipboard"}
              >
                {copied ? (
                  <Check className="h-3 w-3" />
                ) : (
                  <Copy className="h-3 w-3" />
                )}
              </button>
            )}
          </>
        )}
      </div>
      <div className={`rounded px-3 py-2 sm:px-4 sm:py-3 text-[15px] sm:text-sm leading-relaxed border max-w-[85%] md:max-w-[70%] ${
        isUser
          ? isMidTask
            ? "bg-green/5 border-green/20 ml-auto"
            : "bg-card border-border ml-auto"
          : "bg-alt border-border mr-auto"
      }`}>
        {isUser ? (
          <div className="whitespace-pre-wrap break-words">{message.content}</div>
        ) : message.content ? (
          <ResponseRenderer
            text={message.content}
            markdownRender={preferences.markdownRender}
          />
        ) : message.toolCalls && message.toolCalls.length > 0 ? (
          <div className="text-fg-faint italic text-xs">
            Task completed — {message.toolCalls.length} tool call{message.toolCalls.length !== 1 ? 's' : ''} executed.
          </div>
        ) : (
          <div className="text-fg-faint italic text-xs">
            (No output — the agent may have encountered an issue.)
          </div>
        )}

        {/* Attachments in user messages */}
        {isUser && message.attachments && message.attachments.length > 0 && (
          <div className="mt-2 flex flex-wrap gap-1.5">
            {message.attachments.map(att => (
              <span key={att.id} className="inline-flex items-center gap-1 rounded border border-border bg-bg px-1.5 py-0.5 text-[10px] text-fg-faint">
                <Paperclip className="h-2.5 w-2.5" />
                {att.fileName}
              </span>
            ))}
          </div>
        )}
      </div>
      {message.toolCalls?.map((tc) => (
        <ToolCard key={tc.id} tool={tc} />
      ))}
    </div>
  );
});

// Small inline Paperclip icon (avoiding extra imports in the loop)
function Paperclip({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48" />
    </svg>
  );
}
