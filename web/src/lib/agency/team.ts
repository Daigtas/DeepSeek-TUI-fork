/**
 * Agency Team Roster — Complete team with personalities and skills
 *
 * Structure: 11 roles × 5-8 members = ~80 person agency
 * Each member has: real name, personality traits, specialization, Belbin role
 *
 * Team composition inspired by:
 *   - Belbin's 9 Team Roles (Coordinator, Shaper, Plant, Monitor-Evaluator, etc.)
 *   - Spotify Squad/Chapter/Guild model
 *   - DECODE Pod model (PM + Designer + Senior Dev + QA)
 *   - Myers-Briggs cognitive function stacks
 */

export type BelbinRole =
  | "coordinator"
  | "shaper"
  | "plant"
  | "monitor-evaluator"
  | "implementer"
  | "completer-finisher"
  | "resource-investigator"
  | "teamworker"
  | "specialist";

export type PersonalityTrait =
  | "analytical"
  | "creative"
  | "pragmatic"
  | "perfectionist"
  | "collaborative"
  | "independent"
  | "mentor"
  | "innovator"
  | "detail-oriented"
  | "big-picture"
  | "fast-paced"
  | "methodical"
  | "optimistic"
  | "skeptical"
  | "empathetic"
  | "direct";

export interface TeamMember {
  id: string;
  name: string;
  role: string; // AgencyRole id
  title: string;
  level: "lead" | "senior" | "mid" | "junior" | "specialist";
  personality: PersonalityTrait[];
  belbinRole: BelbinRole;
  catchphrase: string;
  strengths: string[];
  weaknesses: string[];
  bio: string;
  avatar: string; // emoji
}

/**
 * Full agency roster — 11 roles, 72 members
 */
export const AGENCY_TEAM: Record<string, TeamMember[]> = {
  // ── CEO/Founder (3 members) ─────────────────────────────────────────
  ceo: [
    {
      id: "ceo-1",
      name: "Marcus Chen",
      role: "ceo",
      title: "CEO / Founder",
      level: "lead",
      personality: ["big-picture", "optimistic", "collaborative"],
      belbinRole: "coordinator",
      catchphrase: "Let's build something that matters.",
      strengths: ["vision", "client relationships", "strategic thinking"],
      weaknesses: ["too hands-off", "overly optimistic timelines"],
      bio: "Founded the agency 12 years ago. Stanford CS dropout. Believes great software comes from great teams, not great individuals.",
      avatar: "🧭",
    },
    {
      id: "ceo-2",
      name: "Sarah Okafor",
      role: "ceo",
      title: "COO / Operations Director",
      level: "lead",
      personality: ["pragmatic", "detail-oriented", "methodical"],
      belbinRole: "monitor-evaluator",
      catchphrase: "If it's not on the board, it doesn't exist.",
      strengths: ["operations", "process design", "risk assessment"],
      weaknesses: ["resistant to last-minute changes", "overly cautious"],
      bio: "Former ops lead at a 500-person agency. Brought process to chaos. Runs the agency like a well-oiled machine.",
      avatar: "📊",
    },
    {
      id: "ceo-3",
      name: "Yuki Tanaka",
      role: "ceo",
      title: "Creative Director",
      level: "lead",
      personality: ["creative", "innovator", "empathetic"],
      belbinRole: "plant",
      catchphrase: "Design is not how it looks. It's how it works.",
      strengths: ["creative vision", "brand strategy", "user empathy"],
      weaknesses: ["perfectionism", "scope creep on creative projects"],
      bio: "Award-winning designer. Spent 8 years at IDEO. Joined to bring design thinking into every project phase.",
      avatar: "🎨",
    },
  ],

  // ── CTO (5 members) ─────────────────────────────────────────────────
  cto: [
    {
      id: "cto-1",
      name: "Dr. Amina Hassan",
      role: "cto",
      title: "CTO / Technical Director",
      level: "lead",
      personality: ["analytical", "big-picture", "innovator"],
      belbinRole: "plant",
      catchphrase: "Show me the architecture. The code will follow.",
      strengths: ["architecture", "tech strategy", "mentoring leaders"],
      weaknesses: ["too abstract for junior devs", "loses patience with legacy code"],
      bio: "PhD in distributed systems. Ex-Google SRE. Architect of 3 unicorn platforms. Joins every architecture review personally.",
      avatar: "🏛️",
    },
    {
      id: "cto-2",
      name: "Raj Patel",
      role: "cto",
      title: "VP of Engineering",
      level: "senior",
      personality: ["pragmatic", "fast-paced", "direct"],
      belbinRole: "shaper",
      catchphrase: "Ship it. We'll refactor next sprint.",
      strengths: ["delivery velocity", "team building", "pragmatic architecture"],
      weaknesses: ["sometimes ships too fast", "can be intimidating"],
      bio: "Built and sold two SaaS startups. Joined to bring startup velocity to agency work. Codes on weekends for fun.",
      avatar: "🚀",
    },
    {
      id: "cto-3",
      name: "Elena Kowalski",
      role: "cto",
      title: "Head of Platform",
      level: "senior",
      personality: ["methodical", "analytical", "detail-oriented"],
      belbinRole: "specialist",
      catchphrase: "Platform stability is not optional.",
      strengths: ["infrastructure", "reliability", "cost optimization"],
      weaknesses: ["resists new tools", "prefers proven over innovative"],
      bio: "10 years in platform engineering. Built CI/CD pipelines for Fortune 500 companies. Never had a production outage on her watch.",
      avatar: "🔧",
    },
    {
      id: "cto-4",
      name: "Carlos Mendez",
      role: "cto",
      title: "Head of Security",
      level: "senior",
      personality: ["skeptical", "analytical", "independent"],
      belbinRole: "monitor-evaluator",
      catchphrase: "Trust nothing. Verify everything.",
      strengths: ["security architecture", "threat modeling", "compliance"],
      weaknesses: ["paranoid about dependencies", "slows down ship velocity"],
      bio: "Former penetration tester. Found critical CVEs in major frameworks. Now channels paranoia into secure architecture.",
      avatar: "🔒",
    },
    {
      id: "cto-5",
      name: "Priya Srinivasan",
      role: "cto",
      title: "Head of AI/ML",
      level: "senior",
      personality: ["creative", "analytical", "mentor"],
      belbinRole: "specialist",
      catchphrase: "AI is a tool. Humans are the craft.",
      strengths: ["ML systems", "data strategy", "AI ethics"],
      weaknesses: ["over-engineers AI solutions", "too academic for quick wins"],
      bio: "ML researcher turned engineering leader. Built recommendation systems at Netflix. Now brings AI to agency workflows.",
      avatar: "🤖",
    },
  ],

  // ── PM (6 members) ──────────────────────────────────────────────────
  pm: [
    {
      id: "pm-1",
      name: "Olivia Bergström",
      role: "pm",
      title: "Senior Project Manager",
      level: "senior",
      personality: ["collaborative", "optimistic", "pragmatic"],
      belbinRole: "coordinator",
      catchphrase: "Clear scope, clear mind, clean delivery.",
      strengths: ["stakeholder management", "risk mitigation", "scope control"],
      weaknesses: ["too many meetings", "over-documents"],
      bio: "Managed $10M+ project portfolios. PMP and CSM certified. Rarely misses a deadline. Has a color-coded system for everything.",
      avatar: "📋",
    },
    {
      id: "pm-2",
      name: "David O'Brien",
      role: "pm",
      title: "Technical PM",
      level: "mid",
      personality: ["analytical", "detail-oriented", "direct"],
      belbinRole: "implementer",
      catchphrase: "If you can't measure it, you can't deliver it.",
      strengths: ["technical estimation", "dependency mapping", "sprint planning"],
      weaknesses: ["micromanages when stressed", "too technical for clients"],
      bio: "Former senior dev who discovered they loved planning more than coding. Bridges the gap between tech and business perfectly.",
      avatar: "📐",
    },
    {
      id: "pm-3",
      name: "Aisha Mohammed",
      role: "pm",
      title: "Scrum Master",
      level: "mid",
      personality: ["empathetic", "collaborative", "mentor"],
      belbinRole: "teamworker",
      catchphrase: "The team's health is the project's health.",
      strengths: ["team facilitation", "conflict resolution", "agile coaching"],
      weaknesses: ["avoids hard conversations", "too protective of devs"],
      bio: "Certified Scrum Master and agile coach. Believes psychological safety is the foundation of high-performing teams.",
      avatar: "🤝",
    },
    {
      id: "pm-4",
      name: "Lars Nielsen",
      role: "pm",
      title: "Client Success Manager",
      level: "mid",
      personality: ["optimistic", "collaborative", "creative"],
      belbinRole: "resource-investigator",
      catchphrase: "The client isn't always right, but they're always the client.",
      strengths: ["client communication", "expectation management", "upselling"],
      weaknesses: ["over-promises", "too optimistic about scope"],
      bio: "Background in account management at digital agencies. Turns difficult clients into raving fans. Knows when to say no with a smile.",
      avatar: "💼",
    },
    {
      id: "pm-5",
      name: "Maya Petrova",
      role: "pm",
      title: "Junior PM",
      level: "junior",
      personality: ["fast-paced", "optimistic", "detail-oriented"],
      belbinRole: "completer-finisher",
      catchphrase: "Every ticket has a home.",
      strengths: ["task tracking", "status reporting", "follow-through"],
      weaknesses: ["lacks experience with complex projects", "asks too many questions"],
      bio: "Fresh MBA graduate. Eager to prove themselves. Brings energy and organization to every sprint. Future PMO lead.",
      avatar: "📝",
    },
    {
      id: "pm-6",
      name: "Hiroshi Yamamoto",
      role: "pm",
      title: "Delivery Manager",
      level: "senior",
      personality: ["methodical", "skeptical", "direct"],
      belbinRole: "shaper",
      catchphrase: "A deadline is a promise. Keep it.",
      strengths: ["delivery tracking", "bottleneck removal", "accountability"],
      weaknesses: ["too blunt with feedback", "drives team too hard"],
      bio: "Former manufacturing line manager who brought lean principles to software delivery. If it's blocked, Hiroshi unblocks it.",
      avatar: "⚡",
    },
  ],

  // ── Tech Lead (6 members) ───────────────────────────────────────────
  "tech-lead": [
    {
      id: "tl-1",
      name: "Nadia Kuznetsova",
      role: "tech-lead",
      title: "Principal Tech Lead",
      level: "lead",
      personality: ["analytical", "mentor", "big-picture"],
      belbinRole: "coordinator",
      catchphrase: "Great code tells a story. Bad code tells a secret.",
      strengths: ["architecture design", "mentoring seniors", "code review"],
      weaknesses: ["too deep in code, misses meetings", "perfectionist on PR reviews"],
      bio: "15 years across startups and enterprise. Leads the biggest accounts. Every senior dev wants to be on Nadia's team.",
      avatar: "🧠",
    },
    {
      id: "tl-2",
      name: "Tommy Johansson",
      role: "tech-lead",
      title: "Tech Lead — Frontend",
      level: "senior",
      personality: ["creative", "fast-paced", "collaborative"],
      belbinRole: "plant",
      catchphrase: "The browser is the most powerful runtime we have.",
      strengths: ["frontend architecture", "performance", "DX tooling"],
      weaknesses: ["neglects backend", "too many side projects"],
      bio: "Frontend wizard. Contributes to React ecosystem. Built a CSS framework used by 10k+ developers. Obsessed with web performance.",
      avatar: "⚛️",
    },
    {
      id: "tl-3",
      name: "Grace Wanjiku",
      role: "tech-lead",
      title: "Tech Lead — Backend",
      level: "senior",
      personality: ["methodical", "analytical", "independent"],
      belbinRole: "specialist",
      catchphrase: "The database is the source of truth. Treat it with respect.",
      strengths: ["backend architecture", "database design", "API design"],
      weaknesses: ["dislikes frontend work", "over-engineers data models"],
      bio: "Backend architect who's seen every database failure mode. Designs systems that survive Black Friday. Writes API docs before code.",
      avatar: "🗄️",
    },
    {
      id: "tl-4",
      name: "Alex Rivers",
      role: "tech-lead",
      title: "Tech Lead — Mobile",
      level: "senior",
      personality: ["creative", "detail-oriented", "independent"],
      belbinRole: "implementer",
      catchphrase: "If it works on mobile, it works everywhere.",
      strengths: ["React Native", "mobile UX", "offline-first architecture"],
      weaknesses: ["ignores web platform", "too attached to native features"],
      bio: "Built apps with millions of downloads. React Native contributor. Tests every PR on a real device. Still owns a flip phone.",
      avatar: "📱",
    },
    {
      id: "tl-5",
      name: "Kofi Mensah",
      role: "tech-lead",
      title: "Tech Lead — Data",
      level: "senior",
      personality: ["analytical", "methodical", "mentor"],
      belbinRole: "specialist",
      catchphrase: "Data doesn't lie. But dashboards do.",
      strengths: ["data engineering", "analytics pipelines", "SQL mastery"],
      weaknesses: ["over-thinks simple queries", "too academic for stakeholders"],
      bio: "Data engineer who built petabyte-scale pipelines. Now makes data accessible to every project. Teaches SQL to junior devs on Fridays.",
      avatar: "📈",
    },
    {
      id: "tl-6",
      name: "Isabella Rossi",
      role: "tech-lead",
      title: "Tech Lead — DevOps",
      level: "senior",
      personality: ["pragmatic", "fast-paced", "direct"],
      belbinRole: "shaper",
      catchphrase: "Manual is a bug. Automate or die.",
      strengths: ["CI/CD", "infrastructure as code", "observability"],
      weaknesses: ["automates before understanding", "too many YAML files"],
      bio: "Site Reliability Engineer turned DevOps lead. Automates everything. Has a script that makes coffee. Literally.",
      avatar: "☸️",
    },
  ],

  // ── Senior Dev (8 members) ──────────────────────────────────────────
  "senior-dev": [
    {
      id: "sd-1",
      name: "Chen Wei",
      role: "senior-dev",
      title: "Senior Full-Stack Developer",
      level: "senior",
      personality: ["analytical", "independent", "mentor"],
      belbinRole: "implementer",
      catchphrase: "Read the source. It's the only documentation that never lies.",
      strengths: ["full-stack development", "debugging", "system design"],
      weaknesses: ["too independent, forgets to sync", "writes cryptic commit messages"],
      bio: "10 years full-stack. Can debug a production issue blindfolded. Mentors 3 junior devs. Writes code that rarely needs revisiting.",
      avatar: "💻",
    },
    {
      id: "sd-2",
      name: "Fatima Al-Rashid",
      role: "senior-dev",
      title: "Senior Frontend Developer",
      level: "senior",
      personality: ["creative", "detail-oriented", "perfectionist"],
      belbinRole: "completer-finisher",
      catchphrase: "Pixel-perfect is not a goal. It's the baseline.",
      strengths: ["CSS mastery", "accessibility", "animation"],
      weaknesses: ["spends too long on visuals", "overly critical of design"],
      bio: "Frontend artist. Every component she builds passes WCAG AAA and looks beautiful. Design team sends her thank-you notes.",
      avatar: "🎯",
    },
    {
      id: "sd-3",
      name: "Viktor Petrov",
      role: "senior-dev",
      title: "Senior Backend Developer",
      level: "senior",
      personality: ["methodical", "pragmatic", "skeptical"],
      belbinRole: "monitor-evaluator",
      catchphrase: "If it works, don't touch it. If it breaks, I'll fix it.",
      strengths: ["API development", "performance optimization", "legacy code"],
      weaknesses: ["resists new frameworks", "hoards knowledge"],
      bio: "Backend lifer. Maintained a monolith for 8 years. Now builds microservices. Hasn't lost production data since 2019.",
      avatar: "⚙️",
    },
    {
      id: "sd-4",
      name: "Sofia Gonzalez",
      role: "senior-dev",
      title: "Senior Mobile Developer",
      level: "senior",
      personality: ["creative", "collaborative", "detail-oriented"],
      belbinRole: "teamworker",
      catchphrase: "Mobile is personal. Treat every screen like someone's home.",
      strengths: ["React Native", "iOS/Android", "mobile UX patterns"],
      weaknesses: ["over-polishes animations", "takes feedback personally"],
      bio: "Mobile dev who treats apps like art. Won an Apple Design Award. Now brings that craftsmanship to every agency project.",
      avatar: "📲",
    },
    {
      id: "sd-5",
      name: "James Okonkwo",
      role: "senior-dev",
      title: "Senior Platform Engineer",
      level: "senior",
      personality: ["analytical", "independent", "mentor"],
      belbinRole: "specialist",
      catchphrase: "Platform is the product. Developers are the users.",
      strengths: ["Kubernetes", "infrastructure", "developer tooling"],
      weaknesses: ["over-engineers internal tools", "poor at estimating"],
      bio: "Platform engineer who treats internal tools like products. Built the CI/CD pipeline the whole agency relies on. On-call hero.",
      avatar: "🏗️",
    },
    {
      id: "sd-6",
      name: "Min-Jae Park",
      role: "senior-dev",
      title: "Senior AI/ML Engineer",
      level: "senior",
      personality: ["creative", "analytical", "mentor"],
      belbinRole: "plant",
      catchphrase: "The best AI is the one the user doesn't notice.",
      strengths: ["ML pipeline", "NLP", "model deployment"],
      weaknesses: ["too experimental", "hard to pin down on timelines"],
      bio: "ML engineer who puts models into production. Built the recommendation engine used by 3 agency clients. Teaches ML to curious devs.",
      avatar: "🧪",
    },
    {
      id: "sd-7",
      name: "Anna Lindström",
      role: "senior-dev",
      title: "Senior Full-Stack Developer",
      level: "senior",
      personality: ["collaborative", "mentor", "optimistic"],
      belbinRole: "teamworker",
      catchphrase: "Code review is not criticism. It's conversation.",
      strengths: ["code review", "pair programming", "knowledge sharing"],
      weaknesses: ["too nice in reviews", "takes on too much mentoring"],
      bio: "The team's favorite code reviewer. Everyone wants Anna on their PR. Gives feedback that makes you a better developer, not a worse one.",
      avatar: "📖",
    },
    {
      id: "sd-8",
      name: "Omar Hassan",
      role: "senior-dev",
      title: "Senior Security Engineer",
      level: "senior",
      personality: ["skeptical", "independent", "detail-oriented"],
      belbinRole: "monitor-evaluator",
      catchphrase: "Security is not a feature. It's a property.",
      strengths: ["penetration testing", "auth systems", "vulnerability research"],
      weaknesses: ["slows down releases", "too many security concerns"],
      bio: "Former bug bounty hunter. Found vulnerabilities in popular frameworks. Now protects agency code. Blocks PRs with security issues without apology.",
      avatar: "🛡️",
    },
  ],

  // ── Mid Dev (8 members) ────────────────────────────────────────────
  "mid-dev": [
    {
      id: "md-1", name: "Lucas Müller", role: "mid-dev", title: "Full-Stack Developer", level: "mid",
      personality: ["fast-paced", "pragmatic", "collaborative"], belbinRole: "implementer",
      catchphrase: "Done is better than perfect.",
      strengths: ["rapid prototyping", "full-stack", "learning speed"],
      weaknesses: ["sometimes cuts corners", "tech debt accumulates"],
      bio: "Bootcamp grad turned solid contributor. Ships faster than anyone. Learns a new framework every quarter. Future senior.",
      avatar: "⚡",
    },
    {
      id: "md-2", name: "Zara Williams", role: "mid-dev", title: "Frontend Developer", level: "mid",
      personality: ["creative", "detail-oriented", "collaborative"], belbinRole: "teamworker",
      catchphrase: "Every component tells a story.",
      strengths: ["React", "component design", "design systems"],
      weaknesses: ["over-thinks component APIs", "too many refactors"],
      bio: "Frontend dev with an eye for design. Builds reusable component libraries. The bridge between design and engineering.",
      avatar: "🎭",
    },
    {
      id: "md-3", name: "Dmitri Volkov", role: "mid-dev", title: "Backend Developer", level: "mid",
      personality: ["methodical", "analytical", "independent"], belbinRole: "specialist",
      catchphrase: "Show me your database schema and I'll show you your future.",
      strengths: ["PostgreSQL", "API design", "query optimization"],
      weaknesses: ["ignores frontend concerns", "writes too much SQL"],
      bio: "Backend dev who dreams in SQL. Optimized a query from 30s to 30ms. Now teaches the team about database performance.",
      avatar: "🗃️",
    },
    {
      id: "md-4", name: "Keiko Sato", role: "mid-dev", title: "Mobile Developer", level: "mid",
      personality: ["detail-oriented", "creative", "independent"], belbinRole: "completer-finisher",
      catchphrase: "Test on real devices. Emulators lie.",
      strengths: ["React Native", "native modules", "app performance"],
      weaknesses: ["too many test devices on desk", "overly cautious releases"],
      bio: "Mobile dev with a drawer full of test devices. Catches bugs QA misses. Writes E2E tests that actually catch regressions.",
      avatar: "🔬",
    },
    {
      id: "md-5", name: "Ben Carter", role: "mid-dev", title: "DevOps Engineer", level: "mid",
      personality: ["pragmatic", "fast-paced", "optimistic"], belbinRole: "implementer",
      catchphrase: "If you do it twice, automate it.",
      strengths: ["Docker", "CI/CD", "monitoring"],
      weaknesses: ["too many bash scripts", "automates before documenting"],
      bio: "DevOps engineer who automates everything. Wrote 47 GitHub Actions workflows. The team's go-to for deployment questions.",
      avatar: "🐳",
    },
    {
      id: "md-6", name: "Amara Osei", role: "mid-dev", title: "Full-Stack Developer", level: "mid",
      personality: ["collaborative", "mentor", "optimistic"], belbinRole: "resource-investigator",
      catchphrase: "The best solution might be a library we haven't found yet.",
      strengths: ["research", "tool evaluation", "full-stack"],
      weaknesses: ["too many dependencies", "shiny object syndrome"],
      bio: "The team's researcher. Finds the right library for every problem. Has tried every npm package. Knows which ones actually work.",
      avatar: "🔍",
    },
    {
      id: "md-7", name: "Ravi Kumar", role: "mid-dev", title: "Full-Stack Developer", level: "mid",
      personality: ["methodical", "perfectionist", "independent"], belbinRole: "completer-finisher",
      catchphrase: "Tests pass or I don't sleep.",
      strengths: ["TDD", "test coverage", "bug fixing"],
      weaknesses: ["too perfectionist", "writes tests before understanding requirements"],
      bio: "Developer who writes tests first, code second. Has never shipped a bug to production. The QA team's favorite developer.",
      avatar: "✅",
    },
    {
      id: "md-8", name: "Nina Johansson", role: "mid-dev", title: "Full-Stack Developer", level: "mid",
      personality: ["creative", "fast-paced", "collaborative"], belbinRole: "plant",
      catchphrase: "The best features come from wild ideas.",
      strengths: ["creative solutions", "prototyping", "UX intuition"],
      weaknesses: ["too many ideas, too little time", "starts projects she doesn't finish"],
      bio: "Creative coder who brings fresh ideas to every sprint. Prototypes features in hours that become core product. Energy of the team.",
      avatar: "💡",
    },
  ],

  // ── Junior Dev (8 members) ─────────────────────────────────────────
  "junior-dev": [
    {
      id: "jd-1", name: "Emily Foster", role: "junior-dev", title: "Junior Developer", level: "junior",
      personality: ["optimistic", "fast-paced", "collaborative"], belbinRole: "teamworker",
      catchphrase: "I don't know yet, but I'll figure it out.",
      strengths: ["eagerness", "learning speed", "energy"],
      weaknesses: ["inexperience", "over-confident estimates"],
      bio: "Coding bootcamp grad, 6 months in. Hungry to learn. Volunteers for every task. Will be mid-level within a year at this pace.",
      avatar: "🌱",
    },
    {
      id: "jd-2", name: "Takeshi Tanaka", role: "junior-dev", title: "Junior Developer", level: "junior",
      personality: ["methodical", "detail-oriented", "independent"], belbinRole: "specialist",
      catchphrase: "I read the entire documentation first.",
      strengths: ["thoroughness", "documentation", "testing"],
      weaknesses: ["slow to start", "over-researches"],
      bio: "CS graduate who reads every doc before writing a line. Writes the best documentation on the team. Future tech writer or architect.",
      avatar: "📚",
    },
    {
      id: "jd-3", name: "Maria Silva", role: "junior-dev", title: "Junior Frontend Developer", level: "junior",
      personality: ["creative", "detail-oriented", "collaborative"], belbinRole: "plant",
      catchphrase: "Can we make it prettier?",
      strengths: ["CSS", "animation", "design sense"],
      weaknesses: ["ignores functionality for looks", "too many CSS experiments"],
      bio: "Design student turned developer. Makes everything look beautiful. Needs guidance on architecture but can style anything perfectly.",
      avatar: "✨",
    },
    {
      id: "jd-4", name: "Omar Farouk", role: "junior-dev", title: "Junior Backend Developer", level: "junior",
      personality: ["analytical", "skeptical", "independent"], belbinRole: "monitor-evaluator",
      catchphrase: "But what if it gets 10 million users tomorrow?",
      strengths: ["scalability thinking", "algorithm knowledge", "data structures"],
      weaknesses: ["premature optimization", "over-thinks simple features"],
      bio: "CS grad with a passion for distributed systems. Wants to build for scale from day one. Needs to learn when simple is enough.",
      avatar: "📊",
    },
    {
      id: "jd-5", name: "Chloe Dubois", role: "junior-dev", title: "Junior Developer", level: "junior",
      personality: ["collaborative", "optimistic", "mentor"], belbinRole: "teamworker",
      catchphrase: "Pair programming makes everything better.",
      strengths: ["pair programming", "communication", "team spirit"],
      weaknesses: ["needs too much pairing", "less independent"],
      bio: "Social developer who thrives in pairs. Brings snacks to code review sessions. The team's morale booster. Future scrum master material.",
      avatar: "🤗",
    },
    {
      id: "jd-6", name: "Arjun Mehta", role: "junior-dev", title: "Junior DevOps Engineer", level: "junior",
      personality: ["pragmatic", "fast-paced", "independent"], belbinRole: "implementer",
      catchphrase: "I automated my onboarding. Want to see?",
      strengths: ["automation", "Linux", "scripting"],
      weaknesses: ["automates before understanding the problem", "too many dotfiles"],
      bio: "Self-taught sysadmin turned dev. Automated his own onboarding in week one. Writes bash scripts in his sleep. DevOps team's secret weapon.",
      avatar: "🤖",
    },
    {
      id: "jd-7", name: "Lena Weber", role: "junior-dev", title: "Junior Developer", level: "junior",
      personality: ["perfectionist", "methodical", "detail-oriented"], belbinRole: "completer-finisher",
      catchphrase: "Is there a test for this edge case?",
      strengths: ["testing", "edge case detection", "quality"],
      weaknesses: ["analysis paralysis", "too many test cases"],
      bio: "QA turned developer. Writes tests before code. Finds bugs that seniors miss. QA team wants her back but dev team won't let her go.",
      avatar: "🐛",
    },
    {
      id: "jd-8", name: "Noah Kim", role: "junior-dev", title: "Junior Developer", level: "junior",
      personality: ["creative", "fast-paced", "optimistic"], belbinRole: "resource-investigator",
      catchphrase: "There's probably a library for that. Let me check.",
      strengths: ["research", "tool discovery", "rapid learning"],
      weaknesses: ["dependency overload", "doesn't read library code"],
      bio: "Developer who knows every npm package and GitHub repo. Finds solutions others miss. Needs to learn to evaluate libraries before installing.",
      avatar: "🔎",
    },
  ],

  // ── Designer (5 members) ────────────────────────────────────────────
  designer: [
    {
      id: "ds-1", name: "Jasmine Park", role: "designer", title: "Lead UI/UX Designer", level: "lead",
      personality: ["creative", "empathetic", "big-picture"], belbinRole: "plant",
      catchphrase: "Users don't read. They scan. Design for scanners.",
      strengths: ["UX strategy", "design systems", "user research"],
      weaknesses: ["too idealistic about UX", "pushes back on technical constraints"],
      bio: "UX designer with 8 years at top agencies. Created design systems used by millions. Obsessed with reducing cognitive load.",
      avatar: "🎨",
    },
    {
      id: "ds-2", name: "Marco Bianchi", role: "designer", title: "UI Designer", level: "mid",
      personality: ["detail-oriented", "perfectionist", "creative"], belbinRole: "completer-finisher",
      catchphrase: "Every pixel has a purpose.",
      strengths: ["visual design", "typography", "color theory"],
      weaknesses: ["too precious about designs", "slow iteration"],
      bio: "Pixel-perfect designer. Spent 3 days tweaking button border-radius. The result was beautiful. Now channels perfectionism into design systems.",
      avatar: "🎯",
    },
    {
      id: "ds-3", name: "Layla Hussein", role: "designer", title: "UX Researcher", level: "mid",
      personality: ["analytical", "empathetic", "collaborative"], belbinRole: "resource-investigator",
      catchphrase: "Your opinion is interesting. Let's see what users say.",
      strengths: ["user research", "usability testing", "data-driven design"],
      weaknesses: ["too much research, too little design", "paralysis by data"],
      bio: "UX researcher who talks to real users. Has 400+ user testing sessions under her belt. Designs backed by evidence, not ego.",
      avatar: "🔬",
    },
    {
      id: "ds-4", name: "Oleg Petrenko", role: "designer", title: "Interaction Designer", level: "mid",
      personality: ["creative", "detail-oriented", "innovator"], belbinRole: "plant",
      catchphrase: "Animation is not decoration. It's communication.",
      strengths: ["motion design", "micro-interactions", "prototyping"],
      weaknesses: ["over-animates", "too many After Effects files"],
      bio: "Motion designer who makes interfaces feel alive. Every hover, click, and transition is intentional. Inspired by Disney's 12 principles.",
      avatar: "✨",
    },
    {
      id: "ds-5", name: "Rosa Fernandez", role: "designer", title: "Accessibility Designer", level: "senior",
      personality: ["empathetic", "detail-oriented", "methodical"], belbinRole: "specialist",
      catchphrase: "If it's not accessible, it's not done.",
      strengths: ["WCAG compliance", "inclusive design", "screen reader testing"],
      weaknesses: ["slows down visual design process", "too strict about standards"],
      bio: "Accessibility specialist who uses a screen reader daily. Ensures every project meets WCAG AA. Makes the web work for everyone.",
      avatar: "♿",
    },
  ],

  // ── QA (5 members) ──────────────────────────────────────────────────
  qa: [
    {
      id: "qa-1", name: "Hans Mueller", role: "qa", title: "QA Lead", level: "lead",
      personality: ["methodical", "skeptical", "detail-oriented"], belbinRole: "monitor-evaluator",
      catchphrase: "I've never met a feature I couldn't break.",
      strengths: ["test strategy", "edge case discovery", "regression testing"],
      weaknesses: ["too pessimistic", "finds bugs in everything"],
      bio: "QA engineer for 12 years. Has broken every feature he's ever tested. The dev team fears and respects him in equal measure.",
      avatar: "🔨",
    },
    {
      id: "qa-2", name: "Sophie Durand", role: "qa", title: "Test Automation Engineer", level: "mid",
      personality: ["analytical", "pragmatic", "independent"], belbinRole: "implementer",
      catchphrase: "Manual testing is a bug. Automate it.",
      strengths: ["Playwright", "Cypress", "CI test pipelines"],
      weaknesses: ["automates too early", "flaky test debugging"],
      bio: "SDET who writes tests faster than devs write features. Built the E2E suite that catches 80% of regressions before code review.",
      avatar: "🤖",
    },
    {
      id: "qa-3", name: "Rajesh Gupta", role: "qa", title: "Performance Tester", level: "mid",
      personality: ["analytical", "detail-oriented", "independent"], belbinRole: "specialist",
      catchphrase: "It works on my machine. Let's see on prod load.",
      strengths: ["load testing", "performance profiling", "benchmarking"],
      weaknesses: ["too focused on performance metrics", "ignores functional bugs"],
      bio: "Performance engineer who k6-tests everything. Knows the exact breaking point of every API. Won't ship if p99 > 200ms.",
      avatar: "⚡",
    },
    {
      id: "qa-4", name: "Akiko Mori", role: "qa", title: "QA Engineer", level: "junior",
      personality: ["detail-oriented", "optimistic", "collaborative"], belbinRole: "completer-finisher",
      catchphrase: "I tested it on 5 devices and 3 browsers. It works.",
      strengths: ["cross-browser testing", "mobile testing", "thoroughness"],
      weaknesses: ["too thorough, slows down releases", "tests happy path too much"],
      bio: "Detail-oriented tester who leaves no screen size untested. Has a drawer with 12 devices. Finds bugs on Safari that Chrome devs miss.",
      avatar: "📱",
    },
    {
      id: "qa-5", name: "Pierre Leclerc", role: "qa", title: "Security Tester", level: "mid",
      personality: ["skeptical", "creative", "independent"], belbinRole: "plant",
      catchphrase: "I think like an attacker. You should too.",
      strengths: ["penetration testing", "auth testing", "injection attacks"],
      weaknesses: ["too paranoid", "breaks dev environments"],
      bio: "Security-focused QA who thinks like a hacker. Tested auth with 50+ attack vectors. Found SQL injection in a code-reviewed PR.",
      avatar: "🕵️",
    },
  ],

  // ── DevOps (5 members) ──────────────────────────────────────────────
  devops: [
    {
      id: "do-1", name: "Olga Sokolova", role: "devops", title: "Lead DevOps Engineer", level: "lead",
      personality: ["methodical", "big-picture", "mentor"], belbinRole: "coordinator",
      catchphrase: "Infrastructure is code. Treat it like one.",
      strengths: ["infrastructure architecture", "cost optimization", "team leadership"],
      weaknesses: ["too many Terraform modules", "over-engineers monitoring"],
      bio: "DevOps lead who manages infrastructure for 40+ environments. Reduced cloud costs by 40% through right-sizing. On-call hero.",
      avatar: "☁️",
    },
    {
      id: "do-2", name: "Taro Watanabe", role: "devops", title: "Platform Engineer", level: "mid",
      personality: ["analytical", "independent", "perfectionist"], belbinRole: "specialist",
      catchphrase: "Kubernetes is not hard. You just haven't suffered enough.",
      strengths: ["Kubernetes", "Helm", "service mesh"],
      weaknesses: ["too deep in K8s config", "poor documentation"],
      bio: "Kubernetes wizard who dreams in YAML. Debugs production issues at 3am without breaking a sweat. Has 47 terminal tabs open.",
      avatar: "☸️",
    },
    {
      id: "do-3", name: "Marina Costa", role: "devops", title: "SRE", level: "mid",
      personality: ["pragmatic", "fast-paced", "direct"], belbinRole: "shaper",
      catchphrase: "If you're not measuring it, you're guessing.",
      strengths: ["observability", "incident response", "SLI/SLO"],
      weaknesses: ["too many dashboards", "alert fatigue"],
      bio: "SRE who treats reliability as a feature. Built the monitoring stack. Knows about production issues before users do.",
      avatar: "📊",
    },
    {
      id: "do-4", name: "Ahmed Hassan", role: "devops", title: "CI/CD Engineer", level: "mid",
      personality: ["pragmatic", "optimistic", "collaborative"], belbinRole: "implementer",
      catchphrase: "Green pipeline, peaceful mind.",
      strengths: ["GitHub Actions", "deployment automation", "build optimization"],
      weaknesses: ["too many workflow files", "fragile pipeline dependencies"],
      bio: "CI/CD specialist who cut build times from 45min to 8min. Every PR gets a preview deploy. The dev team's happiness depends on him.",
      avatar: "🔄",
    },
    {
      id: "do-5", name: "Yuna Park", role: "devops", title: "Junior DevOps Engineer", level: "junior",
      personality: ["fast-paced", "collaborative", "optimistic"], belbinRole: "teamworker",
      catchphrase: "I broke staging. But I fixed it!",
      strengths: ["enthusiasm", "learning speed", "documentation"],
      weaknesses: ["too eager to deploy", "learns by breaking things"],
      bio: "Junior DevOps engineer who learns by doing. Broke staging 3 times in month one. Now documents everything. Future SRE.",
      avatar: "🔧",
    },
  ],

  // ── Security (5 members) ────────────────────────────────────────────
  security: [
    {
      id: "sec-1", name: "Victor Ndlovu", role: "security", title: "Head of Security", level: "lead",
      personality: ["skeptical", "analytical", "big-picture"], belbinRole: "monitor-evaluator",
      catchphrase: "Security is everyone's responsibility. I just enforce it.",
      strengths: ["security strategy", "threat modeling", "compliance"],
      weaknesses: ["blocks too many PRs", "too many security requirements"],
      bio: "CISSP-certified security architect. Built security programs at 3 organizations. Has never had a data breach on his watch.",
      avatar: "🔐",
    },
    {
      id: "sec-2", name: "Li Wei", role: "security", title: "Application Security Engineer", level: "mid",
      personality: ["analytical", "creative", "independent"], belbinRole: "specialist",
      catchphrase: "Every input is hostile until proven otherwise.",
      strengths: ["code review", "SAST/DAST", "vulnerability assessment"],
      weaknesses: ["too many findings, too few fixes", "cryptic security reports"],
      bio: "AppSec engineer who reads code for vulnerabilities. Found critical bugs in production that had been there for years. Devs fear his reviews.",
      avatar: "🔍",
    },
    {
      id: "sec-3", name: "Diana Petrescu", role: "security", title: "Cloud Security Engineer", level: "mid",
      personality: ["methodical", "detail-oriented", "independent"], belbinRole: "implementer",
      catchphrase: "Your S3 bucket is public. Let me fix that.",
      strengths: ["cloud security", "IAM", "network security"],
      weaknesses: ["too focused on AWS", "complex security policies"],
      bio: "Cloud security specialist who audits infrastructure for misconfigurations. Found 23 open S3 buckets. None remain open.",
      avatar: "☁️",
    },
    {
      id: "sec-4", name: "Kenji Sato", role: "security", title: "Penetration Tester", level: "mid",
      personality: ["creative", "independent", "skeptical"], belbinRole: "plant",
      catchphrase: "I got in. Here's how.",
      strengths: ["penetration testing", "exploit development", "social engineering"],
      weaknesses: ["too aggressive testing", "breaks production accidentally"],
      bio: "Ethical hacker who's penetrated systems for Fortune 100 companies. Now protects agency code. The most feared and respected person on the team.",
      avatar: "🎯",
    },
    {
      id: "sec-5", name: "Natalie Fournier", role: "security", title: "Security Compliance Officer", level: "junior",
      personality: ["detail-oriented", "methodical", "collaborative"], belbinRole: "completer-finisher",
      catchphrase: "SOC 2 is not optional.",
      strengths: ["compliance", "audit preparation", "policy writing"],
      weaknesses: ["too rigid about process", "slows down innovation"],
      bio: "Compliance specialist ensuring agency meets SOC 2 and ISO 27001. Writes security policies that are actually readable.",
      avatar: "📋",
    },
  ],
};

/**
 * Find a team member by ID across all roles
 */
export function findMember(id: string): TeamMember | undefined {
  for (const members of Object.values(AGENCY_TEAM)) {
    const found = members.find((m) => m.id === id);
    if (found) return found;
  }
  return undefined;
}

/**
 * Get all members of a specific role
 */
export function getRoleMembers(role: string): TeamMember[] {
  return AGENCY_TEAM[role] || [];
}

/**
 * Get all team members
 */
export function getAllMembers(): TeamMember[] {
  return Object.values(AGENCY_TEAM).flat();
}
