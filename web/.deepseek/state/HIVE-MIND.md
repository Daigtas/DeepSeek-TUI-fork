# Hive-Mind Coordination Rules

## Build Mutex
**ALWAYS read BUILD-LOCK before running `npm run build`.**
```bash
# Check if another agent is building
cat .deepseek/state/BUILD-LOCK 2>/dev/null
# If locked: wait, then check again
# If unlocked: acquire lock, build, release lock
```

## Shared State
**On start**: Read `.deepseek/state/AGENT-STATE.json` to see what other agents are doing.
**On every major action**: Write your current task to the state file.
**On completion**: Update state to "done" with summary.

## Commands That Require Coordination
| Command | Mutex | Reason |
|---------|-------|--------|
| `npm run build` | BUILD-LOCK | Builds must be serial (overwrites `.next/`) |
| `rm -rf .next` | BUILD-LOCK | Would break parallel builds |
| `prisma migrate` | DB-LOCK | Concurrent migrations corrupt schema |
| `docker build` | BUILD-LOCK | Shares Docker daemon |
| `kubectl apply` | K8S-LOCK | Prevents conflicting K8s changes |

## Communication Protocol
Agents communicate through:
1. `.deepseek/state/AGENT-STATE.json` — shared task board
2. `.deepseek/state/BUILD-LOCK` — build exclusion
3. `.deepseek/state/TURN-LOG.md` — what each agent did (append-only)

## Rules
1. NEVER `rm -rf .next` if BUILD-LOCK is held by another agent
2. After editing files, write your changes to TURN-LOG.md
3. If you need a build, acquire BUILD-LOCK first
4. Report errors to TURN-LOG.md so other agents can see them
5. The orchestrator (DeepSeek TUI) is the coordinator — it synchronizes agents
