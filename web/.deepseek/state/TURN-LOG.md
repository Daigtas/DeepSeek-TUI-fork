# ForgeHub Test Suite — QA Sprint S2-T05

**Date**: 2026-05-20
**Agent**: Hans Mueller (qa-1)

---

## Jest Unit/Integration Tests

**Command**: `npx jest --no-coverage --passWithNoTests`

### Summary

| Metric | Count |
|---|---|
| Test Suites | 61 total, 59 passed, **2 failed** |
| Tests | 494 total, 491 passed, **3 failed** |
| Time | 2.099 s |

### Failing Test Suites

#### 1. `tests/api/account.test.ts` — 1 failure

- **Test**: `GET /api/account/sessions › formats sessions with device info`
- **Error**: `expect(received).toBe(expected) // Object.is equality`
  - Expected: `"Chrome on macOS"`
  - Received: `"Chrome"`
- **Location**: `tests/api/account.test.ts:105`
- **Root Cause**: The route handler (`src/app/api/account/sessions/route.ts:31`) calls `parseUserAgent(s.userAgent)` which processes the raw UA string. The mock DB returns `userAgent: 'Chrome on macOS'`, but the `parseUserAgent` function apparently returns just `'Chrome'` instead of `'Chrome on macOS'`. This is likely a mock interaction issue with the dynamic `await import(...)` pattern used in the test — the mock value may not be reaching the handler, or the mock's `findMany` is returning a different value than expected.

#### 2. `tests/workflow/stage-tools.test.ts` — 2 failures

- **Test**: `mergeStageTools() — pure helper › merges configOverride on top of WorkflowStageTool config by key`
- **Test**: `mergeStageTools() — pure helper › ignores malformed override entries safely`
- **Error**: `TypeError: (0 , route_1.mergeStageTools) is not a function`
- **Location**: `tests/workflow/stage-tools.test.ts:177` and `:197`
- **Root Cause**: The test imports `mergeStageTools` from the route module:
  ```ts
  import { GET, mergeStageTools } from '@/app/api/projects/[id]/stages/[stageId]/route'
  ```
  However, `mergeStageTools` is defined in a separate file `merge-stage-tools.ts` and is imported (but NOT re-exported) by `route.ts`. The test must either:
  - Import directly from `./merge-stage-tools` (the source module), or
  - The route module needs `export { mergeStageTools }` added.

---

## Playwright E2E Tests

**Playwright version**: 1.59.1 (available)

**Command**: `npx playwright test --project=chromium`

### Result: FAILED (infrastructure)

```
Authentication failed against database server, the provided database credentials
for `forgehub_app` are not valid.

Please make sure to provide valid database credentials for the database server
at the configured address.

   at scripts/e2e-seed.ts:120
```

The E2E test suite cannot run because the database credentials in the test environment are not configured or valid. The `e2e-seed.ts` global setup script attempts to connect to the database to seed E2E test users and fails at the Prisma `db.user.upsert()` call.

---

## Passing Highlight

All 491 passing tests completed successfully, covering:
- API routes (zones, health, canvas, tools, permissions, n+1 queries)
- Composer (resolver, write-matrix, cache, canvas-doc)
- Security (CSP nonce, preview salt, CSS allowlist, SVG sanitizer, auth rate limiter, port membership, header strip, terminal allowlist, fs symlink, preview import)
- Workflow integration (settings tab)
- Modules (discovery, manifests, widget registry)
- Schema (workflow template tenancy, layout override, site settings boundary)
- Performance (preview host gate, db pool tuning, devserver port allocator, membership LRU, rbac raw SQL)
- Unit tests (theme, helpers, errors, env, email, logger, validators, api respond, toaster cap, zone shell)
- Auth (override cascade seed)
- Observability (metrics, alerts)
- Proxy (dynamic rules)
