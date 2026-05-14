"use client";

import dynamic from "next/dynamic";

const ChatPageContent = dynamic(() => import("./ChatPageContent"), {
  ssr: false,
  loading: () => (
    <div className="flex h-screen items-center justify-center bg-[#0d0d0d]">
      <div className="animate-pulse text-[#888888]">Loading…</div>
    </div>
  ),
});

export default function ChatPage() {
  return <ChatPageContent />;
}
