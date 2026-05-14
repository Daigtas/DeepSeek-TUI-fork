"use client";

import { AlertTriangle, Check, X } from "lucide-react";
import type { PermissionRequest } from "@/lib/types";

interface PermissionPromptProps {
  request: PermissionRequest;
  onApprove: () => void;
  onDeny: () => void;
}

export function PermissionPrompt({ request, onApprove, onDeny }: PermissionPromptProps) {
  const isHigh = request.risk === "high";
  const borderColor = isHigh ? "border-rose/20" : "border-amber/20";
  const iconColor = isHigh ? "text-rose" : "text-amber";

  return (
    <div className={`flex items-start gap-3 border ${borderColor} bg-card p-3`}>
      <AlertTriangle className={`mt-0.5 h-4 w-4 shrink-0 ${iconColor}`} />
      <div className="flex-1 min-w-0">
        <p className="text-sm text-fg">
          Allow <span className="text-amber font-mono text-xs">{request.toolName}</span>?
        </p>
        <p className="mt-0.5 text-xs text-fg-dim">{request.description}</p>
        {request.details && (
          <p className="mt-1 truncate text-xs text-fg-faint font-mono">
            {request.details}
          </p>
        )}
      </div>
      <div className="flex gap-2 shrink-0">
        <button
          onClick={onDeny}
          className="flex items-center gap-1 border border-rose/20 px-3 py-1.5 text-xs text-rose hover:bg-rose/10 transition-colors"
        >
          <X className="h-3.5 w-3.5" />
          Deny
        </button>
        <button
          onClick={onApprove}
          className="flex items-center gap-1 bg-green/15 border border-green/30 px-3 py-1.5 text-xs text-green-light hover:bg-green/25 transition-colors"
        >
          <Check className="h-3.5 w-3.5" />
          Approve
        </button>
      </div>
    </div>
  );
}
