#!/usr/bin/env bash
# Build Orchestrator — coordinates parallel agent builds
# 
# Usage: ./scripts/build-orchestrator.sh [--wait]
#   --wait: block until build mutex is acquired
#
# Agents call this instead of `npm run build` directly.
# Ensures only ONE build runs at a time across all sub-agents.

set -euo pipefail

STATE_DIR="/var/www/tui/.deepseek/state"
LOCK_FILE="$STATE_DIR/BUILD-LOCK"
STATE_FILE="$STATE_DIR/AGENT-STATE.json"
LOG_FILE="$STATE_DIR/TURN-LOG.md"
AGENT_ID="${AGENT_ID:-unknown}"

# ── Acquire build lock ──────────────────────────────

acquire_lock() {
    local waited=0
    while true; do
        if [ ! -f "$LOCK_FILE" ]; then
            echo "$AGENT_ID:$(date +%s)" > "$LOCK_FILE"
            echo "[build-orch] Lock acquired by $AGENT_ID"
            return 0
        fi
        
        local holder
        holder=$(head -1 "$LOCK_FILE" 2>/dev/null | cut -d: -f1)
        echo "[build-orch] Build locked by $holder — waiting... (${waited}s)"
        
        if [ "${1:-}" != "--wait" ] && [ "$waited" -gt 10 ]; then
            echo "[build-orch] Timeout waiting for build lock"
            return 1
        fi
        
        sleep 2
        waited=$((waited + 2))
    done
}

# ── Release build lock ──────────────────────────────

release_lock() {
    if [ -f "$LOCK_FILE" ]; then
        local holder
        holder=$(head -1 "$LOCK_FILE" 2>/dev/null | cut -d: -f1)
        if [ "$holder" = "$AGENT_ID" ]; then
            rm -f "$LOCK_FILE"
            echo "[build-orch] Lock released by $AGENT_ID"
        fi
    fi
}

# ── Log build result ────────────────────────────────

log_build() {
    local status="$1"
    local duration="$2"
    echo "| $(date -u +%H:%M:%S) | $AGENT_ID | build | $status | ${duration}s |" >> "$LOG_FILE"
}

# ── Main ────────────────────────────────────────────

acquire_lock "${1:-}"

START_TIME=$(date +%s)

echo "[build-orch] Starting build ($AGENT_ID)..."
cd /var/www/tui

# Run the actual build
if npm run build; then
    DURATION=$(( $(date +%s) - START_TIME ))
    echo "[build-orch] Build succeeded in ${DURATION}s"
    log_build "success" "$DURATION"
    release_lock
    exit 0
else
    DURATION=$(( $(date +%s) - START_TIME ))
    echo "[build-orch] Build FAILED in ${DURATION}s"
    log_build "FAILED" "$DURATION"
    release_lock
    exit 1
fi
