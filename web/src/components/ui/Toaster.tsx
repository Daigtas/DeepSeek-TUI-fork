"use client";

import { ToastProvider, useToast } from "@/components/Toast";

// Re-export the Toast system from the components directory
export { ToastProvider, useToast };

// Default export for backward compatibility (used by marketplace layout)
export default function Toaster() {
  return <ToastProvider>{null}</ToastProvider>;
}
