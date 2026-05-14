# 🧠 HIVE-MIND COORDINATION — READ FIRST

You are part of a swarm. Other agents are working on the same codebase right now.

## Coordination Rules

### 1. Check State Before Acting
Read `.deepseek/state/AGENT-STATE.json` at startup to see what other agents are doing.

### 2. Build Orchestrator (REQUIRED)
**NEVER run `npm run build` directly.** Use the build orchestrator:
```bash
AGENT_ID=<your-agent-name> bash scripts/build-orchestrator.sh --wait
```
This acquires a mutex so only ONE agent builds at a time.

### 3. Edit Coordination
- After editing files, append to `.deepseek/state/TURN-LOG.md`:
  ```
  | HH:MM:SS | <agent-name> | edit | <file-path> | <brief description> |
  ```
- Before editing a file another agent might be editing, check TURN-LOG.md

### 4. Never Do These Concurrently
| Operation | What to use |
|-----------|------------|
| `npm run build` | `bash scripts/build-orchestrator.sh --wait` |
| `rm -rf .next` | Check BUILD-LOCK first; only if no active build |
| `prisma migrate` | Check DB-LOCK first |
| `kubectl apply` | Check K8S-LOCK first |

### 5. Error Handling
- If build fails, log to TURN-LOG.md with the error
- Don't try to fix another agent's build failure — report it

### 6. On Completion
Write your final status to `.deepseek/state/AGENT-STATE.json`.

## Current Context
Other agents may be working on: layout, accessibility, performance, UX, testing.
You are one agent in the swarm. Act accordingly.
