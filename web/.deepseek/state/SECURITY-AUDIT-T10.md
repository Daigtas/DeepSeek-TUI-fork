# 🔐 T-10 Security Audit — Auto-Login Endpoint

**Auditor:** Victor Ndlovu (sec-1)
**Date:** 2026-05-14
**File:** `src/app/api/auth/auto-login/route.ts`
**Proxy:** `proxy.ts` (12-step pipeline)

---

## Audit Checklist

### 1. 🔴 Token Leakage — VULNERABILITY FOUND

**Finding:** The auto-login token is transmitted as a URL query parameter (`?token=...`).

**Evidence:** `route.ts:20-21`
```ts
const url = new URL(req.url);
const token = url.searchParams.get("token");
```

**Impact:**
- Token is logged in **server access logs** (NGINX, load balancer, Cloudflare).
- Token persists in **browser history** after navigation.
- Token may leak via **Referer headers** if the page loads external resources before redirect.
- After redirect, the token URL remains in history even though the token is consumed.

**Mitigation considered:** Token is single-use (consumed immediately), which limits the window of abuse. However, if an attacker gains access to server logs, they could replay tokens captured before legitimate use. The redirect (302) happens after consumption, so the window between token exposure and consumption is brief but non-zero.

**Severity:** HIGH — token in URL is a well-known anti-pattern for sensitive credentials. Consider switching to a POST with token in body, or using a cryptographic nonce-exchange pattern.

---

### 2. 🔴 Replay Attacks — VULNERABILITY FOUND (TOCTOU Race)

**Finding:** Token consumption is not atomic with validation. A race window exists between token check and token nullification.

**Evidence:** `route.ts:38-89`
```ts
// Step A: Validate token (line 38-44)
if (!settings?.autoLoginToken || settings.autoLoginToken !== token) {
  return NextResponse.json({ error: "Invalid or expired..." }, { status: 401 });
}

// Step B: Expiry check (line 47-63)
// Step C: User lookup (line 72-76)
// Step D: Consume token (line 83-89)
await db.siteSettings.updateMany({
  data: { autoLoginToken: null, autoLoginEmail: null, autoLoginExpiresAt: null },
});
```

**Attack scenario:**
1. Attacker obtains a valid token (e.g., from server logs).
2. Attacker sends **two concurrent requests** with the same token.
3. Both requests pass Step A before either reaches Step D.
4. Both requests succeed — two sessions created from one single-use token.

**Why it's not atomic:** The code uses a read-then-write pattern (`findFirst` → validate → `updateMany`). Prisma's `updateMany` with a `where` clause on the token value would make this atomic. Alternatively, a database transaction with row-level locking would close the window.

**Severity:** HIGH — violates the "single-use" security guarantee stated in the route's own doc comment.

**Recommended fix:**
```ts
// Atomic consume-via-update with WHERE clause
const consumed = await db.siteSettings.updateMany({
  where: { autoLoginToken: token },
  data: { autoLoginToken: null, autoLoginEmail: null, autoLoginExpiresAt: null },
});
if (consumed.count === 0) {
  return NextResponse.json({ error: "Invalid or expired..." }, { status: 401 });
}
// Token is now consumed — safe to proceed with user lookup
```

---

### 3. 🔴 Timing Attacks — VULNERABILITY FOUND

**Finding:** Token comparison is NOT timing-safe.

**Evidence:** `route.ts:39`
```ts
settings.autoLoginToken !== token
```

JavaScript's `!==` operator for strings short-circuits on the first byte that differs. An attacker can measure response times to leak the token character-by-character (standard timing oracle attack).

**Contrast:** The codebase's CSRF module (`src/lib/proxy/csrf.ts`) correctly uses `crypto.timingSafeEqual`:
```ts
return timingSafeEqual(ab, bb)
```

**Real-world feasibility:** Timing attacks over a network are noisy and typically require thousands of samples. However, with the auth rate limit of 10 req/15min, practical exploitation is severely constrained. This is a defense-in-depth concern.

**Severity:** MEDIUM — rate limiting severely constrains practical exploitation, but the fix is trivial and the codebase already has the right pattern elsewhere.

**Recommended fix:**
```ts
import { timingSafeEqual } from 'crypto';
// ...
const tokenBuf = Buffer.from(token, 'utf8');
const storedBuf = Buffer.from(settings.autoLoginToken, 'utf8');
if (tokenBuf.length !== storedBuf.length || !timingSafeEqual(tokenBuf, storedBuf)) {
  return NextResponse.json({ error: "Invalid or expired..." }, { status: 401 });
}
```

---

### 4. 🟡 Brute Force — MITIGATED

**Finding:** The token is presumed to be UUID v4 (122 bits entropy per doc comment). Rate limiting provides defense-in-depth.

**Evidence:**
- `proxy.ts:137-141`: `checkAuthRateLimit` applies to `/api/auth/` routes.
- Config: 10 requests per 15-minute window per IP (`AUTH_RATE_LIMIT_MAX = 10`, `AUTH_RATE_LIMIT_WINDOW_MS = 15 * 60 * 1000`).
- Additional general anonymous rate limit in proxy step 2.

**Note:** Token generation code was not found in the repository. The token is stored in `siteSettings.autoLoginToken` (Prisma schema: `String?`). If generation uses a weaker PRNG than `crypto.randomUUID()`, entropy could be lower than assumed. **Recommend verifying the provisioner script** that creates these tokens.

**Severity:** LOW — rate limiting renders brute force infeasible regardless of token entropy.

---

### 5. 🟡 Email Enumeration — MINOR INFORMATION LEAK

**Finding:** Different error responses reveal different failure states.

**Evidence:** Error matrix:
| Condition | Status | Message |
|---|---|---|
| No token param | 400 | "Missing auto-login token" |
| Wrong token | 401 | "Invalid or expired auto-login token" |
| Token expired | 410 | "Auto-login link has expired. Please use regular login." |
| Token valid, no email set | 400 | "No email associated with auto-login" |
| Token valid, email set, user not found | 404 | "User not found" |

**Analysis:**
- The 410 response **confirms a token was valid** (an attacker with a leaked-but-expired token can confirm it was real).
- The 404 "User not found" could confirm that an email exists in `site_settings` but has no corresponding user (edge case after user deletion).
- However, to reach either the 410 or 404 paths, the attacker must first possess a valid token. Without a valid token, they only see 400/401.

**Severity:** LOW — requires possession of a valid token to extract any meaningful information beyond "token is wrong."

**Recommendation:** Normalize error messages for invalid/expired tokens to a single generic response:
```
"Invalid or expired auto-login token" (401)
```

---

### 6. 🟡 Redirect Validation — ACCEPTABLE (with minor concern)

**Finding:** Redirect URL is constructed from environment variable, not user input. No open redirect.

**Evidence:** `route.ts:94-96`
```ts
const tenantUrl = process.env.NEXT_PUBLIC_APP_URL || "http://localhost:3000";
return NextResponse.redirect(
  `${tenantUrl}/sign-in?email=${encodeURIComponent(settings.autoLoginEmail)}&auto=1`,
);
```

**Analysis:**
- Destination is hardcoded to `/sign-in` on the tenant's own domain.
- `email` parameter comes from database (`settings.autoLoginEmail`), not user input — safe.
- `auto=1` is a static flag.
- **Minor concern:** The user's email is placed in the redirect URL query string, exposing it in browser history and potentially to analytics scripts on the sign-in page.

**Severity:** LOW — no open redirect vector. Email in URL is a minor privacy leak.

---

### 7. 🟢 Rate Limiting — ADEQUATE

**Finding:** The endpoint is rate-limited by the proxy pipeline.

**Evidence:**
- `proxy.ts:137-141`: Explicit auth rate limit on `/api/auth/` routes (10 req / 15 min / IP).
- `proxy.ts:85-92` (Step 2): General anonymous rate limit via `checkRateLimit` (60 req / min for anonymous tier).
- `proxy-rules-loader.ts:136`: `/api/auth/` is in `STATIC_PUBLIC_PREFIXES`, placing it in the public path bypass at step 5 — but the auth rate limit check runs BEFORE the public path bypass.

**Severity:** NONE — rate limiting is properly implemented.

---

### 8. 🟢 HTTPS — ENFORCED

**Finding:** HTTPS is enforced in production by proxy step 1.

**Evidence:** `proxy.ts:68-74`
```ts
if (process.env.NODE_ENV !== 'development' && request.nextUrl.protocol === 'http:') {
  const url = request.nextUrl.clone();
  url.protocol = 'https:';
  return NextResponse.redirect(url, 308);
}
```

Permanent redirect (308) from HTTP to HTTPS in all non-development environments.

**Severity:** NONE — properly enforced.

---

## Summary of Findings

| # | Item | Severity | Status |
|---|------|----------|--------|
| 1 | Token in URL query param | 🔴 HIGH | Vulnerability |
| 2 | TOCTOU race on token consumption | 🔴 HIGH | Vulnerability |
| 3 | Non-timing-safe comparison | 🟡 MEDIUM | Vulnerability (constrained) |
| 4 | Brute force resistance | 🟢 LOW | Mitigated by rate limiting |
| 5 | Email enumeration via errors | 🟢 LOW | Minor leak |
| 6 | Redirect validation | 🟢 LOW | Acceptable |
| 7 | Rate limiting | 🟢 NONE | Adequate |
| 8 | HTTPS enforcement | 🟢 NONE | Enforced |

### Critical Fixes Required (before production)

1. **Atomic token consumption** — Use `updateMany` with a `where: { autoLoginToken: token }` clause to make read-and-consume atomic.
2. **Timing-safe comparison** — Replace `!==` with `crypto.timingSafeEqual`.

### Recommended Improvements

3. **Move token out of URL** — Consider a two-step flow: POST the token, get a short-lived session cookie, then redirect. This keeps the token out of server logs and browser history.
4. **Unify error responses** — Return identical 401 responses for invalid, expired, and consumed tokens to eliminate the timing/error oracles.
