# REQUIREMENTS.md

## REQ-001: PuTTY/basic SSH terminal compatibility
**Status:** Accepted | **Priority:** 1 | **Phase:** 1
Full paste and keyboard input compatibility when connecting via PuTTY over SSH, including PuTTY "basic" terminal type.

## REQ-002: CTRL+V paste detection
**Status:** Accepted | **Priority:** 1 | **Phase:** 1
Detect CTRL+V (0x16 byte) as paste trigger for Windows/PuTTY users who rely on this shortcut.

## REQ-003: Burst paste for high-latency remote sessions
**Status:** Accepted | **Priority:** 2 | **Phase:** 1
Improve burst paste detection to handle SSH sessions where bytes arrive in chunks due to network latency.

## REQ-004: GSD planning as primary orchestration
**Status:** Accepted | **Priority:** 1 | **Phase:** 2
Wire the deepseek-planning crate (PhasePipeline, requirements, roadmap) as the primary coordination layer for swarm agents.

## REQ-005: PuTTY terminal capability detection
**Status:** Accepted | **Priority:** 2 | **Phase:** 1
Correctly detect PuTTY terminal capabilities including "basic" (vt100) mode vs "xterm" mode, without over-flagging all SSH connections.
