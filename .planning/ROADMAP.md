# ROADMAP.md

## Phase 1: PuTTY/basic SSH compatibility & paste fixes
**Status:** Completed ✅ | **Dependencies:** None | **Plans:** 3

- ✅ Plan 01-01: CTRL+V paste detection and handling
- ✅ Plan 01-02: PuTTY terminal capability detection fixes
- ✅ Plan 01-03: Burst paste improvements for remote sessions

## Phase 2: GSD orchestration integration
**Status:** Completed ✅ | **Dependencies:** Phase 1 | **Plans:** 2

- ✅ Plan 02-01: PlanningDirector struct + PhasePipeline → SwarmOrchestrator bridge
- ✅ Plan 02-02: Hive mind context sharing with planning state

## Phase 3: Optimizations & hardening
**Status:** Pending | **Dependencies:** Phase 2 | **Plans:** 2

- Plan 03-01: Input latency profiling and optimization
- Plan 03-02: Comprehensive terminal test matrix
