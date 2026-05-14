"use client";

import dynamic from "next/dynamic";
import { ErrorBoundary } from "@/components/ErrorBoundary";

const HomePage = dynamic(() => import("./HomePage"), {
  ssr: false,
  loading: () => (
    <div className="flex h-screen items-center justify-center bg-[#0d0d0d]">
      <div className="animate-pulse text-[#888888]">Loading…</div>
    </div>
  ),
});

export default function Page() {
  return (
    <ErrorBoundary>
      <HomePage />
    </ErrorBoundary>
  );
}
