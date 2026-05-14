import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db";
import { requireAuth, checkCSRF, checkRateLimit, messageSchema, logApiError } from "@/lib/api-utils";

// POST /api/sessions/[id]/messages — save a message to a session
export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    // CSRF protection for state change
    if (!checkCSRF(req)) {
      return NextResponse.json({ error: "Invalid origin" }, { status: 403 });
    }

    const { error: authErr, userId } = await requireAuth();
    if (authErr) return authErr;

    if (!checkRateLimit(userId!, 60)) {
      return NextResponse.json({ error: "Too many requests" }, { status: 429 });
    }

    const { id: sessionId } = await params;
    const body = await req.json();

    // Validate with Zod
    const parsed = messageSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({
        error: "Invalid input",
        details: parsed.error.flatten().fieldErrors,
      }, { status: 400 });
    }

    const { role, content, toolCalls } = parsed.data;

    // Verify session belongs to user
    const chatSession = await db.chatSession.findFirst({
      where: { id: sessionId, userId: userId! },
    });
    if (!chatSession) {
      return NextResponse.json({ error: "Session not found" }, { status: 404 });
    }

    const message = await db.chatMessage.create({
      data: {
        userId: userId!,
        sessionId,
        role,
        content,
        toolCalls: toolCalls || undefined,
      },
    });

    // Update session timestamp
    await db.chatSession.update({
      where: { id: sessionId },
      data: { updatedAt: new Date() },
    });

    return NextResponse.json({ message });
  } catch (err) {
    logApiError("messages:POST", err);
    return NextResponse.json({ error: "Internal error" }, { status: 500 });
  }
}
