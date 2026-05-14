"use client";

import dynamic from "next/dynamic";
import ChatSkeleton from "./ChatSkeleton";

/**
 * ResponseRenderer — lazy-loaded to keep react-markdown out of the initial bundle.
 * Uses ChatSkeleton as the loading placeholder.
 */
export const ResponseRenderer = dynamic(
  () =>
    import("./ResponseRendererImpl").then((mod) => ({
      default: mod.ResponseRendererImpl,
    })),
  {
    ssr: false,
    loading: () => <ChatSkeleton />,
  }
);

/**
 * StreamingResponseRenderer — direct export for real-time streaming.
 * During streaming it renders plain text; react-markdown is only used
 * for the final render after streaming completes.
 */
export { StreamingResponseRenderer } from "./ResponseRendererImpl";
