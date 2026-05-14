import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db";
import { requireAuth, checkCSRF, checkRateLimit, settingsSchema, logApiError } from "@/lib/api-utils";

// GET /api/settings
export async function GET() {
  try {
    const { error: authErr, userId } = await requireAuth();
    if (authErr) return authErr;

    if (!checkRateLimit(userId!)) {
      return NextResponse.json({ error: "Too many requests" }, { status: 429 });
    }

    const prefs = await db.userPreferences.findUnique({
      where: { userId: userId! },
    });

    return NextResponse.json({
      preferences: prefs ? {
        agentMode: prefs.agentMode,
        model: prefs.model,
        contextLimit: prefs.contextLimit,
        theme: prefs.theme,
        temperature: prefs.temperature,
        maxTokens: prefs.maxTokens,
        autoApprove: prefs.autoApprove,
        markdownRender: prefs.markdownRender,
        syntaxHighlight: prefs.syntaxHighlight,
        hooks: prefs.hooks || null,
      } : null,
    });
  } catch (err) {
    logApiError("settings", err);
    return NextResponse.json({ error: "Internal error" }, { status: 500 });
  }
}

// PUT /api/settings
export async function PUT(req: NextRequest) {
  try {
    // CSRF protection
    if (!checkCSRF(req)) {
      return NextResponse.json({ error: "Invalid origin" }, { status: 403 });
    }

    const { error: authErr, userId } = await requireAuth();
    if (authErr) return authErr;

    if (!checkRateLimit(userId!, 30)) {
      return NextResponse.json({ error: "Too many requests" }, { status: 429 });
    }

    const body = await req.json();

    // Validate with Zod
    const parsed = settingsSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({
        error: "Invalid input",
        details: parsed.error.flatten().fieldErrors,
      }, { status: 400 });
    }

    const { preferences } = parsed.data;

    const prefs = await db.userPreferences.upsert({
      where: { userId: userId! },
      create: {
        userId: userId!,
        agentMode: preferences.agentMode,
        model: preferences.model,
        contextLimit: preferences.contextLimit ?? 128000,
        theme: preferences.theme,
        temperature: preferences.temperature ?? 0.7,
        maxTokens: preferences.maxTokens ?? 32768,
        autoApprove: preferences.autoApprove ?? true,
        markdownRender: preferences.markdownRender ?? true,
        syntaxHighlight: preferences.syntaxHighlight ?? true,
        hooks: preferences.hooks || undefined,
      },
      update: {
        agentMode: preferences.agentMode,
        model: preferences.model,
        contextLimit: preferences.contextLimit ?? 128000,
        theme: preferences.theme,
        temperature: preferences.temperature ?? 0.7,
        maxTokens: preferences.maxTokens ?? 32768,
        autoApprove: preferences.autoApprove ?? true,
        markdownRender: preferences.markdownRender ?? true,
        syntaxHighlight: preferences.syntaxHighlight ?? true,
        hooks: preferences.hooks || undefined,
      },
    });

    return NextResponse.json({
      preferences: {
        agentMode: prefs.agentMode,
        model: prefs.model,
        contextLimit: prefs.contextLimit,
        theme: prefs.theme,
        temperature: prefs.temperature,
        maxTokens: prefs.maxTokens,
        autoApprove: prefs.autoApprove,
        markdownRender: prefs.markdownRender,
        syntaxHighlight: prefs.syntaxHighlight,
      },
    });
  } catch (err) {
    logApiError("settings:PUT", err);
    return NextResponse.json({ error: "Internal error" }, { status: 500 });
  }
}
