"use client";

import { StreamingResponseRenderer } from "./ResponseRenderer";

interface StreamingTextProps {
  text: string;
  markdownRender?: boolean;
  isStreaming?: boolean;
}

export function StreamingText({ text, markdownRender = true, isStreaming = true }: StreamingTextProps) {
  if (!markdownRender) {
    return (
      <div className="border border-border bg-card px-4 py-3">
        <div className="whitespace-pre-wrap break-words text-sm leading-relaxed text-fg">
          {text}
          <span className="ml-0.5 inline-block h-4 w-1.5 bg-amber animate-cursor-blink align-middle" />
        </div>
      </div>
    );
  }

  return (
    <div className="border border-border bg-card px-4 py-3">
      <StreamingResponseRenderer text={text} markdownRender={true} isStreaming={isStreaming} />
    </div>
  );
}
