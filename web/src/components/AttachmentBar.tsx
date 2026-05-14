"use client";

import { useCallback, useState, useRef, type DragEvent, type ClipboardEvent } from "react";
import { Paperclip, X, FileText, Image, File, Plus, Code, Link } from "lucide-react";
import type { FileAttachment } from "@/lib/types";

interface AttachmentBarProps {
  attachments: FileAttachment[];
  onAttach: (files: FileAttachment[]) => void;
  onRemove: (id: string) => void;
  disabled?: boolean;
}

const MAX_FILE_SIZE = 10 * 1024 * 1024; // 10MB
const MAX_TOTAL_SIZE = 50 * 1024 * 1024; // 50MB total

function fileToAttachment(file: File): FileAttachment | null {
  if (file.size > MAX_FILE_SIZE) return null;
  return {
    id: crypto.randomUUID(),
    fileName: file.name,
    fileSize: file.size,
    mimeType: file.type || "application/octet-stream",
  };
}

function formatSize(bytes: number): string {
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  if (bytes >= 1_000) return `${(bytes / 1_000).toFixed(0)} KB`;
  return `${bytes} B`;
}

function fileTypeColor(mimeType: string): string {
  if (mimeType.startsWith("image/")) return "text-cyan border-cyan/20";
  if (mimeType.startsWith("text/") || mimeType.includes("json") || mimeType.includes("javascript") || mimeType.includes("xml") || mimeType.includes("typescript"))
    return "text-green border-green/20";
  if (mimeType.includes("pdf") || mimeType.includes("document") || mimeType.includes("spreadsheet"))
    return "text-amber border-amber/20";
  return "text-fg-faint border-fg-faint/20";
}

function FileIcon({ mimeType, className }: { mimeType: string; className?: string }) {
  const cls = className || "h-4 w-4";
  if (mimeType.startsWith("image/")) return <Image className={cls} />;
  if (mimeType.startsWith("text/") || mimeType.includes("json") || mimeType.includes("javascript") || mimeType.includes("xml"))
    return <FileText className={cls} />;
  if (mimeType.includes("pdf")) return <FileText className={cls} />;
  return <File className={cls} />;
}

// ── Paste content type detection (matching desktop TUI) ──────────────

type DetectedType = "text" | "code" | "link" | "image" | "mixed";

interface DetectionResult {
  type: DetectedType;
  language?: string;
  url?: string;
  mimeType?: string;
}

const SHEBANG_RE = /^#!\s*(?:\/usr\/bin\/env\s+)?(\S+)/;
const DATA_URI_RE = /^data:(image\/\w+);base64,/;
const CODE_INDICATORS: [RegExp, string][] = [
  [/^(package\s+\w+|import\s+["\w]|func\s+\w+\s*\(|func\s+main)/m, "go"],
  [/^(interface\s+\w+\s*\{|type\s+\w+\s*=|function\s+\w+\s*\(|const\s+\w+\s*[:=]|import\s+.*from)/m, "typescript"],
  [/^\{[ \t\r\n]*"[^"]+"\s*:/m, "json"],
  [/^---\s+a\/.*\n\+\+\+\s+b\//m, "diff"],
  [/^(def\s+\w+|class\s+\w+:|import\s+\w+|from\s+\w+\s+import)/m, "python"],
  [/^(public\s+(class|interface|enum|record)|package\s+\w+;)/m, "java"],
  [/^(use\s+\w+::|fn\s+\w+\s*\(|pub\s+(fn|struct|enum|trait|mod))/m, "rust"],
  [/^(<\?xml|<html|<\!DOCTYPE\s+html)/im, "html"],
  [/^(SELECT|INSERT|UPDATE|DELETE|CREATE\s+TABLE)\s/im, "sql"],
  [/^(\.\w+\s*\{|@import|@media|@keyframes)/m, "css"],
  [/^(require\s*\(|module\.exports|import\s+.*from)/m, "javascript"],
  [/^#!\/bin\/(bash|sh)/, "bash"],
  [/^#!\/usr\/bin\/env\s+(python|ruby|perl|node)/, "python"],
];

const URL_RE = /^https?:\/\/[^\s]+$/;

function detectContentType(text: string): DetectionResult {
  // Check for data URIs (images)
  const dataMatch = text.match(DATA_URI_RE);
  if (dataMatch) return { type: "image", mimeType: dataMatch[1] };

  // Check for base64-encoded images
  if (/^[A-Za-z0-9+/]{100,}={0,2}$/.test(text.trim())) {
    // Decode a few bytes to check magic numbers
    try {
      const bytes = Uint8Array.from(atob(text.trim().slice(0, 12)), c => c.charCodeAt(0));
      if (bytes[0] === 0x89 && bytes[1] === 0x50) return { type: "image", mimeType: "image/png" };
      if (bytes[0] === 0xFF && bytes[1] === 0xD8) return { type: "image", mimeType: "image/jpeg" };
      if (bytes[0] === 0x47 && bytes[1] === 0x49) return { type: "image", mimeType: "image/gif" };
    } catch {}
  }

  // Check for URLs
  if (URL_RE.test(text.trim())) return { type: "link", url: text.trim() };

  // Check for shebang
  const shebangMatch = text.match(SHEBANG_RE);
  if (shebangMatch) {
    const lang = shebangMatch[1].split("/").pop() || "shell";
    return { type: "code", language: lang === "node" ? "javascript" : lang };
  }

  // Check for code indicators
  for (const [re, lang] of CODE_INDICATORS) {
    if (re.test(text)) return { type: "code", language: lang };
  }

  return { type: "text" };
}

function DetectionBadge({ detection }: { detection?: DetectionResult }) {
  if (!detection || detection.type === "text") return null;

  if (detection.type === "code") {
    return (
      <span className="inline-flex items-center gap-0.5 text-[9px] text-cyan ml-1">
        <Code className="h-2.5 w-2.5" />
        {detection.language}
      </span>
    );
  }
  if (detection.type === "link") {
    return (
      <span className="inline-flex items-center gap-0.5 text-[9px] text-cyan ml-1">
        <Link className="h-2.5 w-2.5" />
        URL
      </span>
    );
  }
  if (detection.type === "image") {
    return (
      <span className="inline-flex items-center gap-0.5 text-[9px] text-cyan ml-1">
        <Image className="h-2.5 w-2.5" />
        {detection.mimeType?.replace("image/", "")}
      </span>
    );
  }
  return null;
}

export function AttachmentBar({ attachments, onAttach, onRemove, disabled }: AttachmentBarProps) {
  const [dragOver, setDragOver] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const dragCounter = useRef(0);

  const processFiles = useCallback(
    (files: FileList | File[]) => {
      const totalSize = attachments.reduce((s, a) => s + a.fileSize, 0);
      const newAttachments: FileAttachment[] = [];
      let newTotal = totalSize;

      for (const file of files) {
        const att = fileToAttachment(file);
        if (!att) {
          setError(`${file.name} exceeds 10MB limit`);
          continue;
        }
        newTotal += att.fileSize;
        if (newTotal > MAX_TOTAL_SIZE) {
          setError("Total upload size exceeds 50MB limit");
          break;
        }
        newAttachments.push(att);
      }

      if (newAttachments.length > 0) {
        onAttach(newAttachments);
        setError(null);
      }
    },
    [attachments, onAttach],
  );

  // Drag & drop handlers
  const handleDragEnter = useCallback((e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current++;
    setDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounter.current--;
    if (dragCounter.current === 0) setDragOver(false);
  }, []);

  const handleDragOver = useCallback((e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  }, []);

  const handleDrop = useCallback(
    (e: DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setDragOver(false);
      dragCounter.current = 0;
      if (disabled) return;
      if (e.dataTransfer.files?.length) {
        processFiles(e.dataTransfer.files);
      }
    },
    [disabled, processFiles],
  );

  // Click to browse
  const handleBrowse = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleFileInput = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      if (e.target.files?.length) {
        processFiles(e.target.files);
        e.target.value = "";
      }
    },
    [processFiles],
  );

  return (
    <div
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
      className={`relative ${dragOver ? "bg-amber/5" : ""}`}
    >
      {/* Attached files */}
      {attachments.length > 0 && (
        <div className="flex flex-wrap gap-1.5 px-3 pt-2">
          {attachments.map(att => {
            const colorClass = fileTypeColor(att.mimeType);
            return (
            <div
              key={att.id}
              className={`flex items-center gap-2 rounded border bg-card px-2.5 py-1.5 text-xs group transition-colors ${colorClass}`}
            >
              <FileIcon mimeType={att.mimeType} className={`h-4 w-4 shrink-0 ${colorClass.split(" ")[0]}`} />
              <span className="max-w-[120px] sm:max-w-[200px] truncate text-fg">{att.fileName}</span>
              <span className="text-[10px] text-fg-faint tabular-nums shrink-0">{formatSize(att.fileSize)}</span>
              {!disabled && (
                <button
                  onClick={() => onRemove(att.id)}
                  className="ml-0.5 text-fg-faint hover:text-rose transition-colors shrink-0"
                  aria-label={`Remove ${att.fileName}`}
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              )}
            </div>
          )})}
        </div>
      )}

      {/* Paste indicator */}
      <input
        ref={fileInputRef}
        type="file"
        multiple
        onChange={handleFileInput}
        className="hidden"
      />

      {error && (
        <div className="px-3 pt-1 text-[11px] text-rose">{error}</div>
      )}

      {/* Drag overlay */}
      {dragOver && (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center border-2 border-dashed border-amber/40 bg-amber/5">
          <div className="flex items-center gap-2 text-xs text-amber">
            <Paperclip className="h-4 w-4" />
            Drop files to attach
          </div>
        </div>
      )}
    </div>
  );
}

// ── Hook: usePasteHandler ────────────────────────────────────────────────

/** @deprecated This hook is exported but not used outside this file. */
export function usePasteHandler(
  onAttach: (files: FileAttachment[]) => void,
  onPasteText: (text: string) => void,
  enabled: boolean = true,
) {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  const handlePaste = useCallback(
    (e: ClipboardEvent<HTMLTextAreaElement>) => {
      if (!enabled) return;

      const items = e.clipboardData?.items;
      if (!items) return;

      // Check for files first (images, etc.)
      const files: File[] = [];
      for (let i = 0; i < items.length; i++) {
        const item = items[i];
        if (item.kind === "file") {
          const file = item.getAsFile();
          if (file) files.push(file);
        }
      }

      if (files.length > 0) {
        e.preventDefault();
        const attachments = files.map(f => fileToAttachment(f)).filter(Boolean) as FileAttachment[];
        if (attachments.length > 0) onAttach(attachments);
        return;
      }

      // If no files, let the default paste text behavior happen (handled by InputBar)
      const text = e.clipboardData.getData("text/plain");
      if (text && text.length > 0) {
        // For very large text pastes (>100KB), treat as file attachment
        if (text.length > 100_000) {
          e.preventDefault();
          // File constructor is browser-only; cast through Blob for TS compat
          const file = new (globalThis as any).File([text], "pasted-text.txt", { type: "text/plain" }) as File;
          const att = fileToAttachment(file);
          if (att) onAttach([att]);
          return;
        }
        // Otherwise let normal paste processing happen in InputBar
      }
    },
    [enabled, onAttach],
  );

  return { handlePaste, textareaRef };
}
