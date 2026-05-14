# PROJECT.md — DeepSeek TUI Fork

## Vision
AI-powered terminal workspace with daemon mode, swarm orchestration, session persistence, and hybrid context storage. Full PuTTY/basic SSH client compatibility for remote-first usage.

## Constraints
- Maintain upstream TUI compatibility
- Support terminals without bracketed paste (PuTTY, screen, tmux)
- Handle high-latency remote SSH connections
- GSD planning system must be primary orchestration layer

## Decisions
- Burst paste detection as fallback for terminals without bracketed paste
- CTRL+V paste trigger for Windows/PuTTY users
- OSC 52 clipboard only when terminal supports it
- Planning crate drives swarm orchestration via PhasePipeline

## Language
Rust (edition 2024)

## Repository
https://github.com/Daigtas/DeepSeek-TUI-fork

## Version
0.8.21
