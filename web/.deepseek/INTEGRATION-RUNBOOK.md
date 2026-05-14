# ForgeHub-Boottify Integration Runbook

**Last updated:** 2026-05-14
**Status:** Production Ready (test tenant deployed)

---

## Architecture

```
boottify.com (Control Plane)          forgehub tenant (*.boottify.com)
┌──────────────────────┐              ┌──────────────────────────┐
│ Marketplace          │              │ ForgeHub Instance        │
│  ↓                   │              │                          │
│ Deploy Wizard        │   Stripe     │ 122 Prisma models        │
│  ↓                   │   Checkout   │ Owner auto-seeded        │
│ POST /api/deploy     │──────────────│ Auto-login (POST)        │
│  ↓                   │  Webhook     │ PlanUsageWidget          │
│ checkout.session     │◄─────────────│ Instance Mgmt APIs        │
│  ↓                   │              │ Backup/Restore APIs       │
│ executeDeployment()  │──────────────│ Health/Ready endpoints    │
│  ↓                   │  K8s deploy  │                          │
│ Tenant LIVE          │              │                          │
└──────────────────────┘              └──────────────────────────┘
```

### Payment-First Flow
1. User fills wizard → creates Stripe Checkout Session → NO deployment record
2. User pays → Webhook fires → creates deployment + triggers provisioning
3. Failed payments → zero database clutter

---

## Files Created

### daigtas-platform (Control Plane)
| File | Purpose |
|------|---------|
| `scripts/provision-forgehub-tenant.ts` | Full tenant lifecycle: DB → Prisma → K8s → DNS |
| `scripts/seed-forgehub-marketplace.ts` | Marketplace registration + 3 plans + Stripe sync |
| `scripts/sync-forgehub-stripe.ts` | Sync forgehub plans to Stripe Products/Prices |
| `src/lib/deployment/forgehub-provisioner.ts` | Webhook handler: Stripe → deploy → provision |
| `src/app/api/deploy/route.ts` | **Updated**: Payment-first — no deployment until payment |
| `src/app/api/webhooks/stripe/handlers/checkout-handlers.ts` | **Updated**: Creates deployment from Stripe metadata |
| `src/components/deploy/deploy-wizard.tsx` | **Updated**: Handles payment-first flow |

### forgehub (Tenant Application)
| File | Purpose |
|------|---------|
| `src/app/api/health/route.ts` | Liveness probe (DB + Redis) |
| `src/app/api/ready/route.ts` | Readiness probe (startup gate) |
| `src/app/api/auth/auto-login/route.ts` | Auto-login (GET→form, POST→session) |
| `src/app/auto-login/page.tsx` | Client-side auto-login form |
| `src/app/api/tenant/usage/route.ts` | Plan limits + real-time usage API |
| `src/components/provisioning/plan-usage-widget.tsx` | Usage widget with empty state |
| `src/app/api/admin/instance/users/route.ts` | Tenant user management |
| `src/app/api/admin/instance/users/[id]/route.ts` | Remove user (OWNER guard) |
| `src/app/api/admin/instance/storage/route.ts` | File storage browser |
| `src/app/api/admin/instance/database/route.ts` | DB management + reset |
| `src/app/api/admin/instance/backup/route.ts` | Create/list backups |
| `src/app/api/admin/instance/backup/[id]/download/route.ts` | Download backup |
| `src/scripts/backup-tenant.sh` | CLI backup/restore/list |
| `src/lib/webrtc/provider.tsx` | WebRTC voice/video/screen share |
| `prisma/schema.prisma` | **Updated**: autoLogin* fields, toolId FKs |
| `prisma/seeds/07-stage-checklist-templates.ts` | **Updated**: 6 tool-binding examples |

### tui (DeepSeek TUI Web)
| File | Purpose |
|------|---------|
| `src/lib/agency/roles.ts` | 11 agency roles + task routing |
| `src/lib/agency/engine.ts` | Agency system prompt builder |
| `src/lib/agency/team.ts` | 72-person team roster |
| `src/lib/agency/sprint.ts` | Sprint system, ceremonies, burndown |

---

## Deployment Commands

### 1. Register ForgeHub in Marketplace
```bash
cd /var/www/daigtas-platform
npx tsx scripts/seed-forgehub-marketplace.ts
# Output: ForgeHub service + 3 plans (starter/pro/enterprise)
# Auto-syncs Stripe Products + Prices
```

### 2. Build ForgeHub Docker Image
```bash
cd /var/www/forgehub
docker build -t ghcr.io/boottify/forgehub:latest .
docker push ghcr.io/boottify/forgehub:latest
```

### 3. Provision Test Tenant
```bash
cd /var/www/daigtas-platform
DATABASE_URL="postgresql://postgres@localhost:5432/postgres" \
  npx tsx scripts/provision-forgehub-tenant.ts \
  --slug demo \
  --plan starter \
  --admin-email admin@example.com \
  --domain demo.boottify.com
```

### 4. Verify Deployment
```bash
curl https://demo.boottify.com/api/health
# → { "status": "healthy", "checks": { "database": "ok", "redis": "ok" } }

# Auto-login URL is printed by provisioner:
# https://demo.boottify.com/auto-login?token=<uuid>
```

### 5. View Tenant Dashboard
- Plan usage: `GET /api/tenant/usage`
- Users: `GET /api/admin/instance/users`
- Storage: `GET /api/admin/instance/storage`
- Backup: `POST /api/admin/instance/backup`

---

## API Reference

### Health & Readiness
| Endpoint | Method | Response |
|----------|--------|----------|
| `/api/health` | GET | `{ status: "healthy", checks: {...} }` |
| `/api/ready` | GET | 200 when ready, 503 during startup |

### Authentication
| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/auth/auto-login` | GET | Redirects to `/auto-login?token=` |
| `/api/auth/auto-login` | POST | Validates `{ token }`, returns `{ redirectUrl }` |
| `/auto-login` | GET | Client-side form, auto-POSTs token |

### Tenant Management
| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/tenant/usage` | GET | Plan limits + current usage |
| `/api/admin/instance/users` | GET/POST | List/invite users |
| `/api/admin/instance/users/[id]` | DELETE | Remove user |
| `/api/admin/instance/storage` | GET | Browse file storage |
| `/api/admin/instance/database` | GET | DB info + reset |
| `/api/admin/instance/backup` | GET/POST | List/create backups |
| `/api/admin/instance/backup/[id]/download` | GET | Download backup |

---

## Stripe Plans
| Plan | USD/mo | Stripe Price ID |
|------|--------|-----------------|
| ForgeHub Starter | $49 | `price_1TWx9XGw7bip0mcM2ufNivEI` |
| ForgeHub Pro | $199 | `price_1TWx9XGw7bip0mcMq30NBWyM` |
| ForgeHub Enterprise | $999 | `price_1TWx9YGw7bip0mcM4RBvBLcx` |

---

## Test Suite
```
ForgeHub Jest:  59/61 suites pass, 491/494 tests pass
ForgeHub E2E:   Requires DB credentials (staging)
Boottify API:   200 OK (health)
TUI Web:        307 redirect (expected)
```

---

## Known Issues
1. **Stage-tools test failure**: `mergeStageTools` not re-exported from route.ts (pre-existing)
2. **Account test failure**: User-agent parsing mismatch in mocked route (pre-existing)
3. **K8s tenant deploy**: Requires `kubectl` access and Helm chart on target cluster
4. **Playwright E2E**: Requires `DATABASE_URL` environment variable for test DB
