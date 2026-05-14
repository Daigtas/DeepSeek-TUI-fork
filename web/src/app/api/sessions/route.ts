import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db";
import { requireAuth, checkCSRF, checkRateLimit, createSessionSchema, logApiError } from "@/lib/api-utils";

// GET /api/sessions — list user's sessions (summaries only, no message contents)
export async function GET() {
  try {
    const { error: authErr, userId } = await requireAuth();
    if (authErr) return authErr;

    if (!checkRateLimit(userId!)) {
      return NextResponse.json({ error: "Too many requests" }, { status: 429 });
    }

    const sessions = await db.chatSession.findMany({
      where: { userId: userId! },
      orderBy: { updatedAt: "desc" },
      include: {
        messages: {
          orderBy: { createdAt: "asc" },
          take: 50,
          select: {
            id: true, role: true, content: true,
            toolCalls: true, createdAt: true,
          },
        },
      },
      take: 10,
    });

    return NextResponse.json({ sessions });
  } catch (err) {
    logApiError("sessions", err);
    return NextResponse.json({ error: "Internal error" }, { status: 500 });
  }
}

// POST /api/sessions — create a new session
export async function POST(req: NextRequest) {
  try {
    // CSRF protection for state change
    if (!checkCSRF(req)) {
      return NextResponse.json({ error: "Invalid origin" }, { status: 403 });
    }

    const { error: authErr, userId } = await requireAuth();
    if (authErr) return authErr;

    if (!checkRateLimit(userId!, 20)) {
      return NextResponse.json({ error: "Too many requests" }, { status: 429 });
    }

    const body = await req.json().catch(() => ({}));

    // Validate with Zod
    const parsed = createSessionSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({
        error: "Invalid input",
        details: parsed.error.flatten().fieldErrors,
      }, { status: 400 });
    }

    const title = parsed.data.title || "New Chat";

    const chatSession = await db.chatSession.create({
      data: {
        userId: userId!,
        title,
      },
    });

    return NextResponse.json({
      session: {
        id: chatSession.id,
        title: chatSession.title,
        createdAt: chatSession.createdAt,
        updatedAt: chatSession.updatedAt,
        messages: [],
      },
    });
  } catch (err) {
    logApiError("sessions:POST", err);
    return NextResponse.json({ error: "Internal error" }, { status: 500 });
  }
}
