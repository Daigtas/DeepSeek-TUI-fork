import { NextRequest, NextResponse } from "next/server";
import { db } from "@/lib/db";
import { requireAuth, checkCSRF, checkRateLimit, profileSchema, logApiError } from "@/lib/api-utils";

// GET /api/profile
export async function GET() {
  try {
    const { error: authErr, userId } = await requireAuth();
    if (authErr) return authErr;

    if (!checkRateLimit(userId!)) {
      return NextResponse.json({ error: "Too many requests" }, { status: 429 });
    }

    const user = await db.user.findUnique({
      where: { id: userId! },
      select: {
        id: true, name: true, email: true, image: true, bio: true,
        emailVerified: true, createdAt: true,
      },
    });

    if (!user) return NextResponse.json({ error: "Not found" }, { status: 404 });

    return NextResponse.json({
      profile: {
        id: user.id, name: user.name, email: user.email,
        image: user.image, bio: user.bio,
        emailVerified: user.emailVerified,
        createdAt: user.createdAt.toISOString(),
      },
    });
  } catch (err) {
    logApiError("profile", err);
    return NextResponse.json({ error: "Internal error" }, { status: 500 });
  }
}

// PUT /api/profile
export async function PUT(req: NextRequest) {
  try {
    // CSRF protection
    if (!checkCSRF(req)) {
      return NextResponse.json({ error: "Invalid origin" }, { status: 403 });
    }

    const { error: authErr, userId } = await requireAuth();
    if (authErr) return authErr;

    if (!checkRateLimit(userId!, 20)) {
      return NextResponse.json({ error: "Too many requests" }, { status: 429 });
    }

    const body = await req.json();

    // Validate with Zod
    const parsed = profileSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({
        error: "Invalid input",
        details: parsed.error.flatten().fieldErrors,
      }, { status: 400 });
    }

    const { name, bio } = parsed.data;

    const user = await db.user.update({
      where: { id: userId! },
      data: { name: name.trim(), bio: bio?.trim() || null },
      select: {
        id: true, name: true, email: true, image: true, bio: true,
        emailVerified: true, createdAt: true,
      },
    });

    return NextResponse.json({
      profile: {
        id: user.id, name: user.name, email: user.email,
        image: user.image, bio: user.bio,
        emailVerified: user.emailVerified,
        createdAt: user.createdAt.toISOString(),
      },
    });
  } catch (err) {
    logApiError("profile:PUT", err);
    return NextResponse.json({ error: "Internal error" }, { status: 500 });
  }
}
