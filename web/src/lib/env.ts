// Environment variable access — build-safe
// Uses process.env directly with defaults; Zod validation skipped during build.
import { z } from "zod";

const envSchema = z.object({
  DATABASE_URL: z.string().optional().default("postgresql://localhost:5432/tui_web"),
  BETTER_AUTH_SECRET: z.string().optional().default("dev-secret-change-in-production-32chars+"),
  BETTER_AUTH_URL: z.string().optional().default("http://localhost:3100"),
  NEXT_PUBLIC_APP_URL: z.string().optional().default("http://localhost:3100"),
  DEEPSEEK_API_KEY: z.string().optional().default("sk-placeholder"),
  DEEPSEEK_MODEL: z.string().optional().default("deepseek-v4-pro"),
  DEEPSEEK_CONTEXT_LIMIT: z.coerce.number().optional().default(128000),
  WS_PORT: z.coerce.number().optional().default(3101),
});

// Parse lazily — defaults prevent build-time crash
const parsed = envSchema.safeParse(process.env);
const data = parsed.success ? parsed.data : envSchema.parse({}); // fallback to defaults

export const env = data;
