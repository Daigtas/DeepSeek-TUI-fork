/**
 * Agency Engine — Builds agency-style system prompts with role delegation
 *
 * Integrates with the TUI's existing mode system.
 * New mode: "agency" — the AI acts as a full agency team with hierarchy.
 */
import type { AgencyRole } from "./roles";
import { AGENCY_ROLES, routeTask } from "./roles";

export interface AgencyConfig {
  /** The role the AI should play (default: "pm") */
  role: AgencyRole;
  /** Whether to include the full team hierarchy in the prompt */
  showTeam: boolean;
  /** Task description for routing */
  task?: string;
}

/**
 * Build an agency-style system prompt.
 *
 * The prompt includes:
 *   1. The AI's specific role and responsibilities
 *   2. The full team hierarchy (who to delegate to)
 *   3. Task routing (who should handle what)
 *   4. Review and approval gates
 */
export function buildAgencyPrompt(config: AgencyConfig): string {
  const role = AGENCY_ROLES[config.role];
  const team = config.showTeam ? buildTeamOverview(config.role) : "";

  let prompt = `## 🏢 AGENCY MODE — ${role.title}\n\n`;
  prompt += `${role.systemPromptExtension}\n\n`;

  if (config.task) {
    const routedRoles = routeTask(config.task);
    prompt += `## Task Analysis\n`;
    prompt += `Task: "${config.task}"\n`;
    prompt += `Primary roles for this task: ${routedRoles.map((r) => AGENCY_ROLES[r].title).join(", ")}\n\n`;
  }

  if (team) {
    prompt += team;
  }

  prompt += buildDelegationRules(role);
  prompt += buildQualityGates(role);

  return prompt;
}

/**
 * Build a team overview showing who reports to whom.
 */
function buildTeamOverview(currentRole: AgencyRole): string {
  let overview = `## Your Team\n\n`;
  overview += `\`\`\`\n`;
  overview += `CEO/Founder\n`;
  overview += `├── CTO/Tech Director                    ← Architecture, code standards\n`;
  overview += `└── Project Manager (PM)                 ← Task breakdown, coordination\n`;
  overview += `    ├── Tech Lead                        ← Technical planning, code review\n`;
  overview += `    │   ├── Senior Developer             ← Complex features, mentoring\n`;
  overview += `    │   │   ├── Mid Developer            ← Feature development, bug fixes\n`;
  overview += `    │   │   └── Junior Developer          ← Simple tasks, tests, docs\n`;
  overview += `    ├── UI/UX Designer                   ← Design, accessibility, polish\n`;
  overview += `    ├── QA Engineer                      ← Testing, quality gates\n`;
  overview += `    ├── DevOps Engineer                  ← CI/CD, deployment, infra\n`;
  overview += `    └── Security Engineer                ← Security audit, vulnerability\n`;
  overview += `\`\`\`\n\n`;
  overview += `Your current role: **${AGENCY_ROLES[currentRole].title}**\n\n`;
  return overview;
}

/**
 * Build delegation rules based on role.
 */
function buildDelegationRules(role: RoleDefinition): string {
  const def = AGENCY_ROLES[role.id];
  let rules = `## Delegation Rules\n\n`;

  if (def.delegatesTo.length > 0) {
    rules += `You can delegate work to:\n`;
    for (const delegateId of def.delegatesTo) {
      const delegate = AGENCY_ROLES[delegateId];
      rules += `- **${delegate.title}** — ${delegate.description}\n`;
    }
    rules += `\nWhen delegating, clearly specify:\n`;
    rules += `1. What exactly needs to be done\n`;
    rules += `2. Acceptance criteria\n`;
    rules += `3. Deadline or priority\n`;
    rules += `4. Who reviews the output\n\n`;
  } else {
    rules += `You are an individual contributor. Execute tasks directly.\n\n`;
  }

  return rules;
}

/**
 * Build quality gates based on role level.
 */
function buildQualityGates(role: RoleDefinition): string {
  let gates = `## Quality Gates\n\n`;

  if (role.reviewRequired) {
    gates += `⚠️ ALL your work MUST be reviewed before it reaches production.\n`;
    gates += `- Code review by: Senior Developer or Tech Lead\n`;
    gates += `- Tests must pass: QA verification required\n`;
    gates += `- Do NOT merge or deploy without approval\n\n`;
  }

  switch (role.approvalAuthority) {
    case "full":
      gates += `You have FULL approval authority. You can approve any decision.\n`;
      break;
    case "technical":
      gates += `You have TECHNICAL approval authority. Approve code/architecture, but not client deliverables.\n`;
      break;
    case "none":
      gates += `You do NOT have approval authority. Escalate decisions to your lead.\n`;
      break;
  }

  gates += `\n`;
  return gates;
}

// Re-export RoleDefinition for use elsewhere
import type { RoleDefinition } from "./roles";
