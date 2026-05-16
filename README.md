# DeepSeek TUI (Fork)

> AI-powered development platform with **rich terminal UI**, **daemon mode**, **swarm orchestration**, **real-time streaming**, **mid-execution pushback**, **session persistence**, and **hybrid context storage** — built on DeepSeek V4.

This fork of [DeepSeek TUI](https://github.com/DeepSeek-TUI/DeepSeek-TUI) adds:
- **Rich TUI** — Ratatui-based multi-pane terminal: Chat, Diff, Tasks, Agents, Jobs with keyboard switching
- **Mid-Execution Pushback** — Type corrections while the agent works; auto-sends on period (.) or Enter
- **Real-Time Streaming** — Tool calls and text deltas stream via hooks as they happen
- **Session Persistence** — Auto-save on Ctrl+C, auto-resume on next `deepseek tui`
- **Web UI** — Next.js 16 browser interface with multi-mode agent chat, streaming, and session management
- **Daemon mode** — Long-lived background agent with HTTP API, swarm coordination, and persistent sessions
- **Swarm orchestration** — Multi-agent task decomposition with DAG-based scheduling
- **Hybrid context** — SQLite + in-memory cache with full-text search

All upstream functionality remains intact.

---

## What's Different from Upstream

### 1. Rich Ratatui TUI (`deepseek tui`)

A complete desktop-grade terminal interface replacing the upstream raw-echo TUI:

```
┌─ 1:Chat ─── 2:Diff ──── 3:Tasks ─── 4:Agents ─── 5:Jobs ──┐
│                                                           │
│  You add dark mode to settings page                       │
│  DS Here's the implementation plan...                     │
│  🔧 running read_file                                     │
│  ✅ completed write_file                                  │
│  DS ▌streaming text appears here...                       │
│                                                           │
│  ❯ type a message…                                       │
├───────────────────────────────────────────────────────────┤
│ ● STREAMING  🤖 Agent  deepseek-v4-pro  ctx 87% (12K) 5r │
└───────────────────────────────────────────────────────────┘
```

- **Chat pane** (key `1`) — Scrollable message history, role-colored messages (You/DS/Tool), input bar with streaming indicator
- **Diff pane** (key `2`) — Git diff with syntax coloring (yellow headers, green/red ± lines), file stats
- **Tasks pane** (key `3`) — GSD task list from `.planning/` ROADMAP.md and STATE.md, status icons
- **Agents pane** (key `4`) — Active swarm agents with status (idle/working/done/failed), periodic 3s poll
- **Jobs pane** (key `5`) — Reserved for background job tracking
- **PaneBar** — Tab bar at top with keyboard shortcut hints, active pane highlighted
- **Status bar** — Context budget gauge, agent mode, model name, round count, streaming indicator
- **Keyboard navigation** — `1`-`5` switch panes, `Up/Down` scroll, `PgUp/PgDown` fast scroll, `Enter` submit or switch to Chat
- Built on **ratatui** + **crossterm** with alternate screen and mouse support

### 2. Mid-Execution Pushback (Claude Code Parity)

The agent is never "unavailable" — type corrections anytime and the agent rethinks mid-stream:

- **Cancellable prompt execution** — `handle_prompt` wrapped in `tokio::spawn` with abort support via oneshot channel bridge. When the user sends a correction, the in-progress API call is cancelled and the prompt restarts with the correction merged.
- **Continuous auto-pushback** — During execution, typing a period (`.`) auto-sends the accumulated input as a correction. No Enter required. A subtle `↻` indicator appears in the terminal.
- **Persistent prompt state** — The background processor keeps the active prompt text across retries. Pushbacks merge cleanly into the original prompt with `[USER CORRECTION]` framing.
- **Race-based architecture** — `tokio::select!` races spawned `handle_prompt` against the pushback channel. On pushback arrival: abort, merge, re-spawn. This replaces the old design where pushbacks were only checked between prompts (never during execution).

```
User types "add dark mode"         → Agent starts working
User types "use system pref".      → Auto-sends correction
Agent aborts current work          → Merges correction
Agent restarts with merged prompt  → "add dark mode\n\n[USER CORRECTION]: use system pref"
```

### 3. Real-Time Streaming via Hooks

Text deltas and tool lifecycles stream to the terminal in real-time, not just after completion:

- **ChannelHookSink** — Implements the `deepseek-hooks` `HookSink` trait, piping `HookEvent` values through an unbounded mpsc channel to the TUI render loop
- **ResponseDelta** events append text character-by-character to the chat pane
- **ToolLifecycle** events show `🔧 running read_file` / `✅ completed write_file` as tools execute
- **ResponseEnd** flushes the streaming buffer to a permanent message
- Integrated into `build_state()` via optional `UnboundedSender<HookEvent>` parameter — only active in `run_tui_rich`, transparent to daemon and stdio modes

### 4. Session Persistence

Full session lifecycle with checkpoint/restore:

- **Auto-save on exit** — `Ctrl+C` and `/exit` capture a `Checkpoint` snapshot (input buffer, thread ID, pending tasks) to `~/.deepseek/tui_checkpoint.json`
- **Auto-resume on start** — `deepseek tui` detects the checkpoint file, displays a prompt ("Press 'r' to resume, any key to skip"), and restores the session
- **`/save` command** — Manual save at any time
- Checkpoint format includes `UiState` snapshot with timestamp, description, and tags

### 5. Slash Commands

Built-in slash commands accessible from the chat input:

| Command | Action |
|---------|--------|
| `/diff` | Load git diff output → switches to Diff pane |
| `/tasks` | Load GSD tasks from `.planning/` → switches to Tasks pane |
| `/swarm` | Refresh agent list from swarm orchestrator → switches to Agents pane |
| `/save` | Manual session save |
| `/clear` | Clear conversation and reset thread |
| `/exit` | Clean exit with auto-save |

---

## Architecture

```
deepseek (CLI dispatcher)
  ├─ deepseek tui              → ratatui TUI (rich multi-pane)
  │   ├─ TuiApp                → Chat, Diff, Tasks, Agents panes
  │   ├─ background processor  → cancellable prompt execution
  │   │   ├─ pushback channel  → mid-execution corrections
  │   │   └─ oneshot bridge   → abort + re-spawn on pushback
  │   ├─ stream_rx channel     → real-time hook events
  │   │   └─ ChannelHookSink   → pipes HookEvent to TUI
  │   ├─ agents_rx channel     → periodic swarm status poll
  │   └─ output_rx channel     → background task results
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

## Crate Map

| Crate | Purpose | LOC |
|---|---|---|
| `deepseek` | CLI dispatcher with daemon, status, version, stdio commands | 218 |
| `deepseek-app-server` | HTTP API + ratatui TUI + cancellable pushback + streaming hooks | 1,474 |
| `deepseek-context` | SQLite-backed hybrid context store | 1,469 |
| `deepseek-planning` | GSD planning system (phases, plans, requirements) | 587 |
| `deepseek-plugins` | Plugin registry and skill loader | 533 |
| `deepseek-session` | Persistent sessions with export/import | 890 |
| `deepseek-swarm` | Swarm orchestration with hive mind and task graph | 1,521 |
| `deepseek-tui-core` | Extended TUI events, context budget, bracketed paste, checkpoint | 2,884 |
| `deepseek-tools` | Tool metrics, retry policy, parallel execution | 870 |
| `deepseek-agent` | Model registry with lifecycle management | 865 |

**TUI widget module** (`crates/app-server/src/tui/`):

| Widget | Purpose | LOC |
|---|---|---|
| `app.rs` | TuiApp struct, pane switching, render orchestration | 116 |
| `widgets/chat.rs` | Chat pane with messages, input bar, streaming | 169 |
| `widgets/diff.rs` | Git diff parser and syntax-highlighted renderer | 89 |
| `widgets/tasks.rs` | GSD task list with status icons and stats | 114 |
| `widgets/agents.rs` | Swarm agent status display | 84 |
| `widgets/pane_bar.rs` | Tab bar with keyboard shortcuts | 45 |
| `widgets/status.rs` | Status bar with context gauge | 85 |

---

## Web UI

A full Next.js 15 web application providing a browser-based interface:

- **Multi-mode agent** — Agent, Plan, YOLO, and **Agency** modes with configurable system prompts
- **Agency Engine** — 11-role development agency hierarchy (CEO, CTO, PM, Tech Lead, Senior/Mid/Junior Dev, Designer, QA, DevOps, Security) with 72 team members, Belbin roles, and sprint system
- **Real-time streaming** — SSE streaming with tool call visualization and mid-task interruption
- **Session management** — Create, rename, delete, and search chat sessions with server-side persistence
- **Mobile-first design** — Responsive, WCAG AA accessible, 44px touch targets

---

## Install

### Download Binary (Linux x86_64)

```bash
curl -L https://github.com/Daigtas/DeepSeek-TUI-fork/releases/latest/download/deepseek -o ~/bin/deepseek
chmod +x ~/bin/deepseek
~/bin/deepseek version
```

### From Source

```bash
git clone https://github.com/Daigtas/DeepSeek-TUI-fork.git
cd DeepSeek-TUI-fork
cargo build --release
# Binary at target/release/deepseek
```

### Web UI Setup

```bash
cd web
npm install
npm run build
npm start        # or: sudo systemctl enable --now deepseek-tui-web
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

## Upstream Compatibility

This fork is fully compatible with the upstream DeepSeek TUI. It adds a rich terminal UI, mid-execution pushback, real-time streaming, and session persistence without removing or breaking any existing functionality. The daemon, stdio, model integration, tool suite, and configuration system remain identical to upstream.

---

## License

[MIT](LICENSE) — same as upstream.
