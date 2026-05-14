"use client";

import { useMemo } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { Components } from "react-markdown";

interface ResponseRendererProps {
  text: string;
  markdownRender?: boolean;
  className?: string;
}

// ── Inline code highlighter (lightweight, no external dep needed) ────────

const KEYWORD_RE = /\b(function|const|let|var|return|if|else|for|while|class|import|export|from|async|await|try|catch|throw|new|typeof|instanceof|extends|default|switch|case|break|continue|yield|static|public|private|protected|interface|type|enum|namespace|declare|readonly|abstract|implements|void|never|any|boolean|number|string|symbol|object|true|false|null|undefined)\b/g;
const STRING_RE = /("[^"\\]*(\\.[^"\\]*)*"|'[^'\\]*(\\.[^'\\]*)*'|`[^`\\]*(\\.[^`\\]*)*`)/g;
const COMMENT_RE = /(\/\/[^\n]*|\/\*[\s\S]*?\*\/)/g;
const NUMBER_RE = /\b(\d+\.?\d*)\b/g;

function htmlEscape(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function highlightCode(code: string): string {
  // Escape HTML first so <tags> in code don't render as DOM
  let result = htmlEscape(code);

  // Comments (after escaping, // and /* */ are still intact)
  result = result.replace(COMMENT_RE, '<span class="text-fg-faint italic">$1</span>');
  // Strings
  result = result.replace(STRING_RE, '<span class="text-green">$1</span>');
  // Keywords
  result = result.replace(KEYWORD_RE, '<span class="text-cyan-light">$1</span>');
  // Numbers
  result = result.replace(NUMBER_RE, '<span class="text-amber-dim">$1</span>');

  return result;
}

// ── Markdown components ──────────────────────────────────────────────────

const markdownComponents: Components = {
  // Code blocks — syntax highlighting
  pre: ({ children, ...props }) => (
    <pre className="bg-alt border border-border p-3 overflow-x-auto text-xs leading-relaxed font-mono" {...props}>
      {children}
    </pre>
  ),
  code: ({ className, children, ...props }) => {
    const match = /language-(\w+)/.exec(className || "");
    const codeStr = String(children).replace(/\n$/, "");

    if (match) {
      // Fenced code block with language
      return (
        <code className={`block ${className || ""}`} {...props}>
          <div className="mb-1 text-[10px] uppercase tracking-wider text-fg-faint">{match[1]}</div>
          <div dangerouslySetInnerHTML={{ __html: highlightCode(codeStr) }} />
        </code>
      );
    }

    // Inline code
    return (
      <code className="bg-alt text-amber px-1 py-0.5 text-xs font-mono" {...props}>
        {children}
      </code>
    );
  },

  // Headings
  h1: ({ children, ...props }) => (
    <h1 className="mt-4 mb-2 text-base font-bold text-fg border-b border-border pb-1" {...props}>{children}</h1>
  ),
  h2: ({ children, ...props }) => (
    <h2 className="mt-3 mb-1.5 text-sm font-bold text-fg" {...props}>{children}</h2>
  ),
  h3: ({ children, ...props }) => (
    <h3 className="mt-2 mb-1 text-sm font-semibold text-fg-dim" {...props}>{children}</h3>
  ),

  // Text
  p: ({ children, ...props }) => (
    <p className="mb-2 leading-relaxed" {...props}>{children}</p>
  ),
  strong: ({ children, ...props }) => (
    <strong className="font-bold text-fg" {...props}>{children}</strong>
  ),
  em: ({ children, ...props }) => (
    <em className="italic text-fg-dim" {...props}>{children}</em>
  ),

  // Links
  a: ({ children, href, ...props }) => (
    <a
      href={href}
      className="text-cyan hover:text-cyan-light underline underline-offset-2"
      target="_blank"
      rel="noopener noreferrer"
      {...props}
    >
      {children}
    </a>
  ),

  // Lists
  ul: ({ children, ...props }) => (
    <ul className="mb-2 ml-4 space-y-0.5 list-disc marker:text-amber" {...props}>{children}</ul>
  ),
  ol: ({ children, ...props }) => (
    <ol className="mb-2 ml-4 space-y-0.5 list-decimal marker:text-fg-faint" {...props}>{children}</ol>
  ),
  li: ({ children, ...props }) => (
    <li className="text-sm" {...props}>{children}</li>
  ),

  // Blockquotes
  blockquote: ({ children, ...props }) => (
    <blockquote className="mb-2 border-l-2 border-amber/30 pl-3 italic text-fg-dim" {...props}>{children}</blockquote>
  ),

  // Horizontal rule
  hr: (props) => (
    <hr className="my-3 border-border" {...props} />
  ),

  // Tables
  table: ({ children, ...props }) => (
    <div className="mb-2 overflow-x-auto">
      <table className="w-full border-collapse text-xs" {...props}>{children}</table>
    </div>
  ),
  thead: ({ children, ...props }) => (
    <thead className="border-b border-border" {...props}>{children}</thead>
  ),
  th: ({ children, ...props }) => (
    <th className="px-2 py-1 text-left font-semibold text-fg bg-alt" {...props}>{children}</th>
  ),
  td: ({ children, ...props }) => (
    <td className="px-2 py-1 border-t border-border text-fg-dim" {...props}>{children}</td>
  ),
};

// ── Main component ───────────────────────────────────────────────────────

export function ResponseRendererImpl({
  text,
  markdownRender = true,
  className = "",
}: ResponseRendererProps) {
  const rendered = useMemo(() => {
    if (!markdownRender) {
      // Plain text with basic code detection
      return (
        <div className="whitespace-pre-wrap break-words text-sm leading-relaxed text-fg">
          {text}
        </div>
      );
    }

    return (
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={markdownComponents}
      >
        {text}
      </ReactMarkdown>
    );
  }, [text, markdownRender]);

  return (
    <div className={className}>
      {rendered}
    </div>
  );
}

// ── Streaming variant — lightweight, no full re-render on each chunk ─────

export function StreamingResponseRenderer({
  text,
  markdownRender = true,
  isStreaming = true,
}: {
  text: string;
  markdownRender?: boolean;
  isStreaming?: boolean;
}) {
  if (!markdownRender) {
    return (
      <div className="whitespace-pre-wrap break-words text-sm leading-relaxed text-fg">
        {text}
        <span className="ml-0.5 inline-block h-4 w-1.5 bg-amber animate-cursor-blink align-middle" />
      </div>
    );
  }

  // While streaming, render plain text — no markdown parse on every chunk
  if (isStreaming) {
    return (
      <div className="whitespace-pre-wrap break-words text-sm leading-relaxed text-fg">
        {text}
        <span className="ml-0.5 inline-block h-4 w-1.5 bg-amber animate-cursor-blink align-middle" />
      </div>
    );
  }

  // Streaming done — render full markdown once
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={markdownComponents}
    >
      {text}
    </ReactMarkdown>
  );
}
