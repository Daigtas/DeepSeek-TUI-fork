## What's New

### Rich Ratatui TUI
Multi-pane terminal UI with Chat, Diff, Tasks, Agents, and Jobs panes.
Switch with keys 1-5. Scroll with arrow keys.

### Mid-Execution Pushback
Type corrections while the agent works — auto-sends on period (.) or Enter.
The agent rethinks and continues without restarting from scratch.

### Real-Time Streaming
Tool calls and text deltas stream to the terminal in real-time via the hooks system.

### Session Persistence
Auto-save on Ctrl+C or /exit. Auto-resume on next startup.

### Slash Commands
/diff — git diff view
/tasks — GSD task list
/swarm — agent swarm status
/save — manual session save

### Full Changelog
- Cancellable prompt execution with oneshot channel bridge
- ChannelHookSink for real-time hook event streaming
- Agents pane with 3s swarm status poll
- Tasks pane reading .planning/ ROADMAP.md and STATE.md
- Diff pane with git diff syntax coloring
- Agency mode now visible in Settings UI
