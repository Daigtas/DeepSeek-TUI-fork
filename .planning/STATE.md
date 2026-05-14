# STATE.md

## Phase Status
Phase 1-3 — Complete ✅ | **Phase 4: TUI paste wiring — Complete ✅**

## Current Work
- Release binary verified: `deepseek v0.8.21` (7.1 MB)
- `deepseek tui` command now enters raw mode with full paste support

## Root Cause of "paste not working"
`TerminalInput` and `BracketedPasteBuffer` were dead code — never constructed or wired into any I/O path. The binary only started an HTTP server via `deepseek serve`. No stdin raw mode = no bracketed paste, burst paste, or CTRL+V detection.

## Fix: `deepseek tui` command
Added `run_tui()` which:
1. Creates `TerminalInput` with `TerminalCaps::detect_from_env()`
2. Enables raw mode on stdin (calls `tcgetattr`/`cfmakeraw`)
3. Enables bracketed paste mode (`\e[?2004h`) when terminal supports it
4. Reads stdin bytes through `BracketedPasteBuffer::feed_bytes()`
5. **Paste detection**: bracketed paste (`\e[200~`…`\e[201~`), burst paste (fallback), CTRL+V (`0x16`)
6. Echoes input, builds input line, handles backspace
7. On Enter: submits `PromptRequest` to Runtime, prints response
8. On Ctrl+C/Ctrl+D: disables raw mode, exits cleanly

## Architecture — now three transport modes
```
deepseek tui      → raw terminal stdin (BracketedPasteBuffer → UiEvent → Runtime)
deepseek serve    → HTTP/WebSocket JSON-RPC (axum server)
deepseek stdio    → stdin/stdout JSON-RPC (IDE integration)
```

## Files changed (this session)
| File | Changes |
|---|---|
| `crates/app-server/src/lib.rs` | +160 lines: `run_tui()` with full event loop, paste handling, Runtime integration |
| `crates/deepseek/src/main.rs` | +9 lines: `Tui` command variant, import `run_tui` |
