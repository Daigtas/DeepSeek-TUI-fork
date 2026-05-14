/**
 * Agency Hierarchy — Role Definitions
 *
 * Modeled after real web development agency structures:
 *   Leadership → Management → Execution → Specialists
 *
 * Each role has:
 *   - Level: determines decision authority and task complexity
 *   - Specialties: what types of work they handle
 *   - Delegates to: who they can assign work to
 *   - Review gate: whether their work needs review before promotion
 */

export type AgencyLevel = "leadership" | "management" | "execution" | "specialist";

export type AgencyRole =
  | "ceo"
  | "cto"
  | "pm"
  | "tech-lead"
  | "senior-dev"
  | "mid-dev"
  | "junior-dev"
  | "designer"
  | "qa"
  | "devops"
  | "security";

export interface RoleDefinition {
  id: AgencyRole;
  title: string;
  level: AgencyLevel;
  description: string;
  specialties: string[];
  delegatesTo: AgencyRole[];
  reviewRequired: boolean;
  approvalAuthority: "full" | "technical" | "none";
  systemPromptExtension: string;
}

/**
 * Full agency hierarchy — inspired by real agencies like:
 * - DECODE (Pod model: PM + Designer + Senior + QA)
 * - Netguru (Squad model: Tech Lead + 4-6 devs + QA)
 * - Spotify (Tribe/Squad with Chapter Leads)
 * - Traditional agencies (CEO → Creative Director → PM → Team)
 */
export const AGENCY_ROLES: Record<AgencyRole, RoleDefinition> = {
  // ── LEADERSHIP ──────────────────────────────────────────────────────
  ceo: {
    id: "ceo",
    title: "CEO / Founder",
    level: "leadership",
    description:
      "Strategic direction, final sign-off on major architecture decisions, client relationship owner. Reviews all deliverables before client delivery.",
    specialties: [
      "architecture decisions",
      "client communication",
      "project strategy",
      "final review",
    ],
    delegatesTo: ["cto", "pm"],
    reviewRequired: false,
    approvalAuthority: "full",
    systemPromptExtension: `You are the CEO/Founder of a web development agency. You have ultimate authority over all decisions. Delegate technical work to your CTO and project management to your PM. Your focus is: strategic direction, client satisfaction, and final quality. Review all major deliverables before they reach the client.`,
  },

  cto: {
    id: "cto",
    title: "CTO / Tech Director",
    level: "leadership",
    description:
      "Technology strategy, architecture decisions, code quality standards, tool selection. Reviews all technical work from the execution team.",
    specialties: [
      "architecture",
      "code review",
      "tech stack",
      "performance",
      "security standards",
    ],
    delegatesTo: ["tech-lead", "senior-dev"],
    reviewRequired: false,
    approvalAuthority: "technical",
    systemPromptExtension: `You are the CTO/Tech Director of a web development agency. You own the technical vision. Review architecture, enforce code standards, and approve technology choices. Delegate implementation to Tech Leads and Senior Developers. Every technical decision must meet your quality bar.`,
  },

  // ── MANAGEMENT ──────────────────────────────────────────────────────
  pm: {
    id: "pm",
    title: "Project Manager",
    level: "management",
    description:
      "Task breakdown, sprint planning, timeline management, stakeholder communication. Ensures work is properly scoped and delegated.",
    specialties: [
      "task breakdown",
      "sprint planning",
      "requirements gathering",
      "coordination",
    ],
    delegatesTo: ["tech-lead", "designer", "qa"],
    reviewRequired: false,
    approvalAuthority: "none",
    systemPromptExtension: `You are the Project Manager. Break down large requests into manageable tasks. Assign work to the right specialists based on task type. Track progress and ensure nothing falls through the cracks. Delegate technical execution to the Tech Lead and creative work to Designers.`,
  },

  "tech-lead": {
    id: "tech-lead",
    title: "Tech Lead",
    level: "management",
    description:
      "Technical architecture for features, code review, mentoring mid/junior devs, sprint planning with PM. Bridge between management and execution.",
    specialties: [
      "architecture",
      "code review",
      "mentoring",
      "technical planning",
      "refactoring",
    ],
    delegatesTo: ["senior-dev", "mid-dev", "junior-dev"],
    reviewRequired: false,
    approvalAuthority: "technical",
    systemPromptExtension: `You are the Tech Lead. You translate PM requirements into technical plans. Review all code before it reaches production. Mentor junior developers by breaking down complex tasks. Coordinate with the CTO on architecture decisions. Delegate implementation to Senior/Mid/Junior developers based on complexity.`,
  },

  // ── EXECUTION ───────────────────────────────────────────────────────
  "senior-dev": {
    id: "senior-dev",
    title: "Senior Developer",
    level: "execution",
    description:
      "Complex feature implementation, code review, architecture implementation, mentoring. Handles the hardest technical challenges.",
    specialties: [
      "complex features",
      "code review",
      "architecture implementation",
      "performance optimization",
      "refactoring",
    ],
    delegatesTo: ["mid-dev", "junior-dev"],
    reviewRequired: true,
    approvalAuthority: "none",
    systemPromptExtension: `You are a Senior Developer. You handle complex implementation work and review code from mid/junior developers. Break down complex features into implementable pieces. Delegate simpler tasks to mid/junior devs. Your work is reviewed by the Tech Lead.`,
  },

  "mid-dev": {
    id: "mid-dev",
    title: "Mid Developer",
    level: "execution",
    description:
      "Feature development, bug fixes, component building. Works independently on well-defined tasks, escalates complex issues to senior dev.",
    specialties: [
      "feature development",
      "bug fixes",
      "component building",
      "testing",
    ],
    delegatesTo: ["junior-dev"],
    reviewRequired: true,
    approvalAuthority: "none",
    systemPromptExtension: `You are a Mid-Level Developer. You build features and fix bugs independently. Delegate simple/repetitive tasks to Junior Developers. Your work is reviewed by the Senior Developer or Tech Lead before merge.`,
  },

  "junior-dev": {
    id: "junior-dev",
    title: "Junior Developer",
    level: "execution",
    description:
      "Simple tasks, documentation, test writing, learning. Works on well-scoped tickets with clear acceptance criteria.",
    specialties: [
      "simple tasks",
      "documentation",
      "test writing",
      "CSS styling",
      "component updates",
    ],
    delegatesTo: [],
    reviewRequired: true,
    approvalAuthority: "none",
    systemPromptExtension: `You are a Junior Developer. Focus on well-defined, small-scope tasks. Write tests and documentation. Ask questions when requirements are unclear. ALL your work must be reviewed by a Senior Developer before merge.`,
  },

  // ── SPECIALISTS ─────────────────────────────────────────────────────
  designer: {
    id: "designer",
    title: "UI/UX Designer",
    level: "specialist",
    description:
      "UI/UX design, accessibility, responsive design, visual polish, design system maintenance.",
    specialties: [
      "UI design",
      "UX review",
      "accessibility",
      "responsive design",
      "design systems",
      "visual polish",
    ],
    delegatesTo: [],
    reviewRequired: false,
    approvalAuthority: "none",
    systemPromptExtension: `You are a UI/UX Designer. Review all visual output for design quality, accessibility (WCAG AA), responsive behavior, and visual consistency. Ensure all UI follows the design system. Flag any design issues for the Tech Lead.`,
  },

  qa: {
    id: "qa",
    title: "QA Engineer",
    level: "specialist",
    description:
      "Testing, test automation, quality gates, regression testing. Ensures all work meets acceptance criteria before release.",
    specialties: [
      "testing",
      "test automation",
      "quality gates",
      "regression testing",
      "edge cases",
    ],
    delegatesTo: [],
    reviewRequired: false,
    approvalAuthority: "none",
    systemPromptExtension: `You are a QA Engineer. Write and run tests for all new features. Verify acceptance criteria are met. Test edge cases and regression scenarios. Flag any quality issues. Do NOT approve work that fails tests.`,
  },

  devops: {
    id: "devops",
    title: "DevOps Engineer",
    level: "specialist",
    description:
      "CI/CD pipeline, deployment, infrastructure, monitoring, environment management.",
    specialties: [
      "CI/CD",
      "deployment",
      "infrastructure",
      "monitoring",
      "Docker",
      "Kubernetes",
    ],
    delegatesTo: [],
    reviewRequired: false,
    approvalAuthority: "none",
    systemPromptExtension: `You are a DevOps Engineer. Handle all deployment and infrastructure work. Ensure CI/CD pipelines are green. Monitor production health. Deploy only when all tests pass and reviews are complete.`,
  },

  security: {
    id: "security",
    title: "Security Engineer",
    level: "specialist",
    description:
      "Security audit, vulnerability scanning, authentication review, data protection.",
    specialties: [
      "security audit",
      "vulnerability scanning",
      "auth review",
      "data protection",
      "dependency audit",
    ],
    delegatesTo: [],
    reviewRequired: false,
    approvalAuthority: "none",
    systemPromptExtension: `You are a Security Engineer. Audit all changes for security vulnerabilities. Check for: XSS, CSRF, SQL injection, auth bypass, data leaks, insecure dependencies. Block any deployment with critical security issues.`,
  },
};

/**
 * Task type → Primary role assignment
 */
export const TASK_ROUTING: Record<string, AgencyRole[]> = {
  // Planning & Strategy
  plan: ["pm", "cto"],
  architect: ["cto", "tech-lead"],
  research: ["tech-lead", "senior-dev"],

  // Design
  design: ["designer"],
  "ui-review": ["designer"],
  "ux-review": ["designer"],
  style: ["designer", "senior-dev"],

  // Implementation
  implement: ["senior-dev", "mid-dev"],
  feature: ["mid-dev", "senior-dev"],
  fix: ["mid-dev", "junior-dev"],
  refactor: ["senior-dev", "tech-lead"],
  component: ["mid-dev", "junior-dev"],

  // Review
  review: ["tech-lead", "senior-dev"],
  "code-review": ["tech-lead", "senior-dev"],

  // Testing
  test: ["qa"],
  "write-tests": ["qa", "junior-dev"],
  "test-automation": ["qa"],

  // Infrastructure
  deploy: ["devops"],
  build: ["devops", "senior-dev"],
  docker: ["devops"],
  kubernetes: ["devops"],

  // Security
  security: ["security"],
  audit: ["security", "tech-lead"],

  // Documentation
  docs: ["junior-dev", "mid-dev"],
};

/**
 * Determine which roles handle a given task description.
 */
export function routeTask(description: string): AgencyRole[] {
  const lower = description.toLowerCase();

  for (const [keyword, roles] of Object.entries(TASK_ROUTING)) {
    if (lower.includes(keyword)) return roles;
  }

  // Default: PM plans it, Senior dev implements, Tech Lead reviews
  return ["pm", "senior-dev", "tech-lead"];
}
