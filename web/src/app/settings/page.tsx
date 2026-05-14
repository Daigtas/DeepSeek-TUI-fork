"use client";

import dynamic from "next/dynamic";

const SettingsPageContent = dynamic(() => import("./SettingsPageContent"), {
  ssr: false,
  loading: () => (
    <div className="flex h-screen items-center justify-center bg-bg">
      <div className="animate-pulse text-fg-faint text-sm">Loading…</div>
    </div>
  ),
});

export default function SettingsPage() {
  return <SettingsPageContent />;
}
