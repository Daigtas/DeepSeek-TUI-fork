/**
 * Shared API utilities — rate limiting, input validation, CSRF, logging.
 * All state-changing API routes should use these guards.
 */
import { NextRequest, NextResponse } from "next/server";
import { z } from "zod";

// ── Rate Limiting (in-memory, per-deployment) ─────────────────────────

interface RateLimitEntry {
  count: number;
  resetAt: number;
}

const rateLimitStore = new Map<string, RateLimitEntry>();

/**
 * Checks rate limit for a given key (userId or IP).
 * Returns true if the request should be allowed.
 */
export function checkRateLimit(
  key: string,
  limit: number = 60,
  windowMs: number = 60_000,
): boolean {
  const now = Date.now();
  const entry = rateLimitStore.get(key);

  if (entry && now < entry.resetAt) {
    entry.count += 1;
    return entry.count <= limit;
  }

  // New window
  rateLimitStore.set(key, { count: 1, resetAt: now + windowMs });
  return true;
}

// Periodically clean up expired entries (every 5 minutes)
if (typeof setInterval !== "undefined") {
  setInterval(() => {
    const now = Date.now();
    for (const [key, entry] of rateLimitStore) {
      if (now >= entry.resetAt) rateLimitStore.delete(key);
    }
  }, 5 * 60 * 1000);
}

// ── Authentication Helper ──────────────────────────────────────────────

import { auth } from "@/lib/auth";
import { headers } from "next/headers";

export async function requireAuth() {
  const hdrs = await headers();
  const session = await auth.api.getSession({ headers: hdrs });
  if (!session) {
    return { error: NextResponse.json({ error: "Unauthorized" }, { status: 401 }), userId: null };
  }
  return { error: null, userId: session.user.id };
}

// ── CSRF Protection ───────────────────────────────────────────────────

/**
 * Validates the Origin/Referer header for state-changing requests.
 * In production, the APP_URL env var should match the expected origin.
 */
export function checkCSRF(req: NextRequest): boolean {
  // Skip CSRF check for GET/HEAD/OPTIONS
  if (["GET", "HEAD", "OPTIONS"].includes(req.method)) return true;

  const origin = req.headers.get("origin");
  const referer = req.headers.get("referer");

  // Allow same-origin requests (no Origin header for same-origin POSTs in some browsers)
  if (!origin && !referer) return true;

  const appUrl = process.env.NEXT_PUBLIC_APP_URL || "http://localhost:3100";
  const allowedOrigins = [
    appUrl,
    appUrl.replace("https://", "http://"),
    appUrl.replace("http://", "https://"),
  ];

  const check = (header: string | null) => {
    if (!header) return false;
    return allowedOrigins.some((o) => header.startsWith(o));
  };

  return check(origin) || check(referer);
}

// ── Input Validation Schemas ──────────────────────────────────────────

export const settingsSchema = z.object({
  preferences: z.object({
    agentMode: z.enum(["agent", "plan", "yolo"]),
    model: z.string().min(1).max(100),
    contextLimit: z.number().int().min(1000).max(2_000_000).optional(),
    theme: z.enum(["dark", "light", "system"]),
    temperature: z.number().min(0).max(2).optional(),
    maxTokens: z.number().int().min(1).max(131_072).optional(),
    autoApprove: z.boolean().optional(),
    markdownRender: z.boolean().optional(),
    syntaxHighlight: z.boolean().optional(),
    hooks: z.any().optional(),
  }),
});

export const profileSchema = z.object({
  name: z.string().min(1).max(100),
  bio: z.string().max(500).optional(),
});

export const createSessionSchema = z.object({
  title: z.string().min(1).max(200).optional(),
});

export const messageSchema = z.object({
  role: z.enum(["user", "assistant", "tool", "error"]),
  content: z.string().min(1).max(500_000),
  toolCalls: z.any().optional(),
});

// ── Logging ───────────────────────────────────────────────────────────

export function logApiError(route: string, err: unknown, userId?: string) {
  const msg = err instanceof Error ? err.message : String(err);
  const userLabel = userId ? ` user=${userId}` : "";
  console.error(`[api:${route}] error${userLabel}: ${msg}`);
}
