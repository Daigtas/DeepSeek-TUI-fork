"use client";

export default function ChatSkeleton() {
  return (
    <div className="flex flex-col gap-4 p-4 animate-pulse" aria-busy="true" aria-label="Loading messages">
      {/* User message */}
      <div className="flex justify-end">
        <div className="w-2/3 max-w-[200px] h-8 rounded bg-bg-hover" />
      </div>
      {/* Assistant message - long */}
      <div className="flex flex-col gap-2">
        <div className="w-full max-w-[85%] h-4 rounded bg-bg-hover" />
        <div className="w-4/5 max-w-[85%] h-4 rounded bg-bg-hover" />
        <div className="w-3/5 max-w-[85%] h-4 rounded bg-bg-hover" />
      </div>
      {/* Assistant message - medium */}
      <div className="flex flex-col gap-2">
        <div className="w-5/6 max-w-[85%] h-4 rounded bg-bg-hover" />
        <div className="w-1/2 max-w-[85%] h-4 rounded bg-bg-hover" />
      </div>
    </div>
  );
}
