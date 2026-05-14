# DeepSeek TUI (Fork)

> AI-powered terminal workspace with **daemon mode**, **swarm orchestration**, **session persistence**, and **hybrid context storage** — built on DeepSeek V4.

This is a fork of [DeepSeek TUI](https://github.com/DeepSeek-TUI/DeepSeek-TUI) that adds backend infrastructure for running the agent as a long-lived daemon with swarm coordination, persistent sessions, and a SQLite-backed context store. The TUI remains fully compatible with the upstream project.

---

## What's Different from Upstream

### 1. Daemon Mode (`deepseek serve --daemon`)

The original DeepSeek TUI runs as an interactive terminal application. This fork adds a production-grade daemon mode:

- **`deepseek serve --daemon`** — Forks to background, detaches from terminal, and serves an HTTP API
- **PID file support** — `--pid-file /var/run/deepseek.pid` for process supervision (systemd, launchd)
- **Auto-shutdown** — `--auto-shutdown-idle` with configurable `--idle-timeout-secs` (default 300s)
- **`deepseek status`** — Dashboard showing connected clients, active tasks, swarm agents, and uptime
- **`deepseek version`** — Prints version and exits
- **`deepseek stdio`** — JSON-RPC over stdin/stdout for IDE/editor integration
- **Daemon detection** — Running `deepseek serve` when a daemon is already active shows the dashboard instead of starting a duplicate
- **systemd service file** — Ships with `deepseek.service` for one-command deployment on Linux

### 2. Swarm Orchestration (`deepseek-swarm` crate)

Multi-agent coordination with a shared knowledge store:

- **Hive Mind** — Thread-safe, versioned key-value store shared across all agents. Supports pub/sub notifications, namespace isolation (`agent.*`, `finding.*`, `decision.*`, `task.*`), and full snapshots for new agent initialization
- **Swarm Orchestrator** — Spawns, assigns tasks to, and coordinates multiple specialized agents concurrently
- **Agent Roles** — Explorer (read-only), Implementer, Reviewer, Tester, Planner, Coordinator, General — each with appropriate tool restrictions
- **Task Graph** — DAG-based task decomposition with dependency resolution, parallel scheduling, and progress tracking
- **Objective Decomposition** — `SwarmOrchestrator::decompose_objective("add dark mode support")` auto-generates a 5-node task graph (explore → design → implement → review → test)
- **Hive Persistence** — `checkpoint_hive()` / `restore_from_store()` save and restore swarm state across daemon restarts

### 3. Hybrid Context Store (`deepseek-context` crate)

SQLite-backed persistent context with in-memory hot cache:

- **Conversation turns** — Full turn storage with reasoning blocks, model info, tool calls, and tags
- **Context entries** — Snippets, links, decisions, file references, workspace state, images
- **Full-text search** — `search_turns("SQLite", 5)` across all stored conversations
- **Hybrid context builder** — `build_hybrid_context()` assembles a token-budgeted context window from recent turns, decisions, workspace state, and search results
- **Batch operations** — `insert_turns_batch()` for efficient bulk storage
- **Tag-based search** — `search_by_tags(["rust", "example"], 10)`

### 4. Session Persistence (`deepseek-session` crate)

Full session lifecycle management:

- **Save/resume** — Sessions persist across daemon restarts with `SessionStore`
- **Export/import** — `.ds-session` archive format (tar.gz) with manifest + turns, validation on import
- **Cross-device resume** — Export on one machine, import on another
- **Session builder** — `SessionBuilder` fluent API for constructing sessions programmatically
- **Search and filter** — `search("rust")`, `list_by_workspace("/path")`, tag filtering
- **Archive validation** — `validate_archive()` checks header, manifest, turns, and structure integrity

### 5. Planning System (`deepseek-planning` crate)

Structured development workflow based on the GSD (Getting Stuff Done) methodology:

- **Project manifest** — PROJECT.md with vision, constraints, decisions
- **Requirements** — REQ-001 style tracking with status (Proposed/Accepted/Implemented/Deferred) and priorities
- **Roadmap** — Phases with dependencies, status tracking, and plan estimates
- **Phase pipeline** — State machine that routes: Discuss → Plan → Execute → Verify → Ship
- **Plan files** — Per-phase PLAN.md with tasks, effort estimates, and agent assignment
- **Project state** — STATE.md with blockers, decisions, and metrics

### 6. Plugin System (`deepseek-plugins` crate)

Composable skill and plugin management:

- **Skill loader** — Parses SKILL.md files with YAML frontmatter (name, description, category, allowed_tools)
- **Plugin registry** — Discovers plugins from disk, manages enable/disable state
- **Skill search** — `search_skills("phase")` finds relevant skills by name/description
- **Install/uninstall** — Plugin manifests with versioning, dependencies, and skill lists
- **Categories** — Workflow, Quality, Context, Manage, Ideate, Custom

### 7. Enhanced TUI Core (`deepseek-tui-core` crate)

Extended the original event system with production features:

- **Context budget tracking** — `ContextBudget` with warning/critical thresholds, ASCII gauge bar rendering (`[████▓▓░░░░] 65%`)
- **Bracketed paste support** — `BracketedPasteBuffer` handles terminal paste sequences (`\e[200~` … `\e[201~`) with multi-line paste detection
- **Paste content detection** — Auto-detects text, code (with language), URLs, images, or mixed content
- **Extended UI events** — 20+ new event types: `PasteStart`, `PasteEnd`, `PasteContent`, `ContextWarning`, `ContextCritical`, `TabPressed`, `SlashCommand`, `AgentSpawned`, `AgentCompleted`, `AgentErrored`, `AgentHeartbeat`, `CaptureCheckpoint`, `RestoreCheckpoint`, etc.
- **Slash command system** — `/help`, `/model`, `/compact`, `/restore`, `/checkpoint`, `/agents`, `/hive`, `/sessions`, `/progress`, `/resume` with completion and popup UI
- **Path autocompletion** — Tab-triggered file/directory completion in the composer
- **Checkpoint system** — Named snapshots of session state for save/restore
- **Stack-allocated effects** — `EffectVec` uses `SmallVec<[UiEffect; 4]>` to avoid heap allocation for common effect chains

### 8. Enhanced Tools (`deepseek-tools` crate)

- **Tool execution metrics** — Per-tool success/failure/timeout/retry tracking with average duration and success rate
- **Retry policy** — Configurable `RetryPolicy` with max retries, exponential backoff, and timeout handling
- **Parallel execution** — `ToolCallRuntime::with_max_parallel(n)` with semaphore-based concurrency control
- **Metrics snapshots** — `metrics_snapshot()` returns aggregated stats across all tools

### 9. Enhanced Agent Registry (`deepseek-agent` crate)

- **Model lifecycle** — `add_model()`, `remove_model()`, `deprecate_model()`, `undeprecate_model()`
- **Stable IDs** — Each model gets a persistent stable identifier for reliable references
- **Provider filtering** — `filter_by_provider(ProviderKind::DeepSeek)`
- **Capability filtering** — `filter_by_capabilities(Capabilities { reasoning: true, .. })`
- **Active models** — `active_models()` returns non-deprecated entries only

### 10. App Server Upgrades (`deepseek-app-server`)

- **Daemon state** — Tracks connected clients, detached mode, active task count, and uptime
- **Daemon API endpoints** — `/healthz`, `/daemon/status`, `/daemon/resume`, `/daemon/progress`
- **Swarm endpoints** — `/swarm/agents`, `/hive/summary`
- **Terminal input module** — Raw mode management with bracketed paste, `TerminalInput` wrapping stdin
- **Daemon supervisor** — Progress logging, agent lifecycle tracking, resume suggestions, hive checkpoint/restore
- **Session integration** — Session store wired into the HTTP API for list/resume/export

---

## Architecture

```
deepseek (CLI dispatcher)
  ├─ deepseek serve --daemon   → app-server (HTTP API)
  │   ├─ supervisor            → progress logging, resume suggestions
  │   ├─ terminal              → raw mode, bracketed paste
  │   ├─ swarm orchestrator    → multi-agent coordination
  │   │   ├─ hive mind         → shared knowledge store
  │   │   └─ task graph        → DAG-based decomposition
  │   ├─ context store         → SQLite + in-memory cache
  │   ├─ session store         → persistent session lifecycle
  │   └─ plugin registry       → skill discovery and loading
  ├─ deepseek stdio            → JSON-RPC for IDE integration
  ├─ deepseek status           → dashboard
  └─ deepseek version          → version info
```

---

## New Crate Map

| Crate | Purpose | LOC |
|---|---|---|
| `deepseek` | CLI dispatcher with daemon, status, version, stdio commands | 209 |
| `deepseek-app-server` | HTTP API with daemon mode, supervisor, terminal input | 1,118 |
| `deepseek-context` | SQLite-backed hybrid context store | 1,469 |
| `deepseek-planning` | GSD planning system (phases, plans, requirements) | 587 |
| `deepseek-plugins` | Plugin registry and skill loader | 533 |
| `deepseek-session` | Persistent sessions with export/import | 890 |
| `deepseek-swarm` | Swarm orchestration with hive mind and task graph | 1,521 |
| `deepseek-tui-core` | Extended TUI events, context budget, bracketed paste | 2,884 |
| `deepseek-tools` | Tool metrics, retry policy, parallel execution | 870 |
| `deepseek-agent` | Model registry with lifecycle management | 865 |

---

## Install

### From Source

```bash
git clone https://github.com/Daigtas/DeepSeek-TUI-fork.git
cd DeepSeek-TUI-fork
cargo build --release
```

The binary is at `target/release/deepseek`.

### Web UI Setup

```bash
cd web
npm install
npm run build
npm start        # or: sudo systemctl enable --now deepseek-tui-web
```

The web UI runs on port 3100 and connects to the daemon's WebSocket endpoint.

### Daemon Setup (systemd)

```bash
sudo cp deepseek.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now deepseek
```

### API Endpoints

| Endpoint | Description |
|---|---|
| `GET /healthz` | Health check |
| `GET /daemon/status` | Connected clients, active tasks, uptime |
| `GET /daemon/resume` | Resume suggestion with active agents and progress |
| `GET /daemon/progress` | Recent progress log entries |
| `GET /swarm/agents` | Active agent list |
| `GET /hive/summary` | Hive mind summary |

---

### 11. Web UI (`web/` directory)

A full Next.js 16 web application providing a browser-based interface to the TUI:

- **Multi-mode agent** — Agent, Plan, YOLO, and Agency modes with configurable system prompts
- **Real-time streaming** — Server-Sent Events streaming with tool call visualization and mid-task interruption
- **Session management** — Create, rename, delete, and search chat sessions with server-side persistence
- **WebSocket backend** — `ws-server.ts` handles chat streams, tool approvals, and disconnection recovery via daemonized tasks
- **Settings system** — Model selection, context limits, theme switching, and hook configuration (system prompt extensions, custom instructions, plugins)
- **Mobile-first design** — Safe area insets, 44px touch targets, responsive breakpoints, WCAG AA accessibility
- **ChatSkeleton** — Shimmer loading placeholders for chat session loading

### 12. Agency Engine (`web/src/lib/agency/`)

Complete web development agency simulation with hierarchical delegation:

- **11 roles across 4 levels** — Leadership (CEO, CTO), Management (PM, Tech Lead), Execution (Senior/Mid/Junior Dev), Specialists (Designer, QA, DevOps, Security)
- **72 team members** — Each with real names, personality traits, Belbin team roles, catchphrases, bios, and emoji avatars
- **Task routing** — Automatic role assignment based on task keywords (audit→Security, design→Designer, deploy→DevOps)
- **Sprint system** — 2-week sprints with backlog management, burndown charts, and Scrum ceremonies (standup, planning, review, retrospective)
- **Quality gates** — Role-based approval authority, review requirements, and delegation rules
- **Personality system** — 16 personality traits combined with 9 Belbin roles for realistic team dynamics

### 13. Hive-Mind Coordination (`web/.deepseek/`)

Sub-agent coordination system for parallel development workflows:

- **Build mutex** — File-based lock prevents concurrent builds from corrupting `.next/`
- **Shared state** — `AGENT-STATE.json` task board visible to all agents
- **Turn log** — `TURN-LOG.md` append-only action log for agent coordination
- **Chunking strategy** — 400-line file chunking for sub-agents to avoid API timeouts
- **Configuration** — `config.toml` with sub-agent timeout (600s), chunking, and hive-mind settings

---

## Upstream Compatibility

This fork is fully compatible with the upstream DeepSeek TUI. It adds backend infrastructure without removing or breaking any existing functionality. The TUI frontend, model integration, tool suite, and configuration system remain identical to upstream.

---

## License

[MIT](LICENSE) — same as upstream.
