export interface Command {
  id: string;
  label: string;
  desc: string;
}

// Aligned with desktop DeepSeek TUI SlashCommand::completions()
// Web-only additions (gsd-*) are kept for backward compat
export const COMMANDS: Command[] = [
  { id: "help", label: "/help", desc: "Show available commands and usage" },
  { id: "compact", label: "/compact", desc: "Free context token budget" },
  { id: "clear", label: "/clear", desc: "Clear current conversation" },
  { id: "model", label: "/model <name>", desc: "Switch AI model" },
  { id: "agents", label: "/agents", desc: "Show agent swarm status" },
  { id: "diff", label: "/diff", desc: "Show current git diff" },
  { id: "file", label: "/file <path>", desc: "Attach a file to conversation" },
  { id: "status", label: "/status", desc: "Show daemon status" },
  { id: "resume", label: "/resume", desc: "Resume previous session" },
  { id: "save", label: "/save", desc: "Save current session" },
  // Web-only extras
  { id: "gsd-plan", label: "/gsd-plan", desc: "Create GSD phase plan" },
  { id: "gsd-execute", label: "/gsd-execute", desc: "Execute GSD phase" },
  { id: "gsd-discuss", label: "/gsd-discuss", desc: "Discuss GSD phase" },
  { id: "gsd-review", label: "/gsd-review", desc: "Review changes" },
  { id: "swarm", label: "/swarm", desc: "Swarm agents" },
  { id: "dashboard", label: "/dashboard", desc: "Metrics dashboard" },
  { id: "memory", label: "/memory", desc: "Agent memory" },
  { id: "session", label: "/session", desc: "Session management" },
];

/** Flat list of command ids for InputBar autocomplete */
export const COMMAND_IDS = COMMANDS.map((c) => c.id);
