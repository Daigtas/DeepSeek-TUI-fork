"use client";

import {
  createContext, useContext, useState, useCallback, useEffect, useRef,
  type ReactNode,
} from "react";
import { CheckCircle2, AlertCircle, Info, X } from "lucide-react";

// ── Types ────────────────────────────────────────────────────────────────

type ToastVariant = "success" | "error" | "info";

interface Toast {
  id: string;
  message: string;
  variant: ToastVariant;
  exiting?: boolean;
}

interface ToastContextValue {
  toast: (message: string, variant?: ToastVariant) => void;
  dismiss: (id: string) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

// ── Hook ──────────────────────────────────────────────────────────────────

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used within a ToastProvider");
  return ctx;
}

// ── Provider ──────────────────────────────────────────────────────────────

const TOAST_DURATION = 3500;
const EXIT_DURATION = 150;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const timersRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const dismiss = useCallback((id: string) => {
    // Start exit animation
    setToasts(prev => prev.map(t => t.id === id ? { ...t, exiting: true } : t));
    // Remove after animation
    const timer = setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id));
      timersRef.current.delete(id);
    }, EXIT_DURATION);
    timersRef.current.set(id + "-exit", timer);
    // Clear auto-dismiss timer
    const autoTimer = timersRef.current.get(id);
    if (autoTimer) {
      clearTimeout(autoTimer);
      timersRef.current.delete(id);
    }
  }, []);

  const toast = useCallback((message: string, variant: ToastVariant = "info") => {
    const id = crypto.randomUUID();
    setToasts(prev => [...prev, { id, message, variant }]);
    // Auto-dismiss
    const timer = setTimeout(() => dismiss(id), TOAST_DURATION);
    timersRef.current.set(id, timer);
  }, [dismiss]);

  // Cleanup timers on unmount
  useEffect(() => {
    return () => {
      timersRef.current.forEach(t => clearTimeout(t));
      timersRef.current.clear();
    };
  }, []);

  return (
    <ToastContext.Provider value={{ toast, dismiss }}>
      {children}
      {/* Toast container — fixed top-right */}
      <div
        className="fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none"
        aria-live="polite"
        aria-label="Notifications"
      >
        {toasts.map(t => (
          <ToastItem key={t.id} toast={t} onDismiss={() => dismiss(t.id)} />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

// ── Toast Item ────────────────────────────────────────────────────────────

const variantStyles: Record<ToastVariant, { border: string; bg: string; icon: typeof CheckCircle2; iconColor: string }> = {
  success: { border: "border-green/30", bg: "bg-green/10", icon: CheckCircle2, iconColor: "text-green" },
  error:   { border: "border-rose/30", bg: "bg-rose/10", icon: AlertCircle, iconColor: "text-rose" },
  info:    { border: "border-amber/30", bg: "bg-amber/10", icon: Info, iconColor: "text-amber" },
};

function ToastItem({ toast: t, onDismiss }: { toast: Toast; onDismiss: () => void }) {
  const { border, bg, icon: Icon, iconColor } = variantStyles[t.variant];

  return (
    <div
      className={`pointer-events-auto flex items-center gap-2.5 rounded border ${border} ${bg} px-3 py-2.5 text-xs shadow-lg backdrop-blur-sm max-w-xs ${
        t.exiting ? "animate-toast-out" : "animate-toast-in"
      }`}
      role="alert"
    >
      <Icon className={`h-4 w-4 shrink-0 ${iconColor}`} />
      <span className="flex-1 text-fg leading-snug">{t.message}</span>
      <button
        onClick={onDismiss}
        className="shrink-0 rounded p-0.5 text-fg-faint hover:text-fg hover:bg-bg-hover transition-colors"
        aria-label="Dismiss notification"
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}
