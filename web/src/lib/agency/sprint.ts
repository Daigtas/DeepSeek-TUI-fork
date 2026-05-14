/**
 * Sprint System — Agency sprint planning, backlogs, and ceremonies
 *
 * Modeled after Scrum + Spotify Squad model:
 *   - 2-week sprints
 *   - Daily standups
 *   - Sprint planning, review, retrospective
 *   - Backlog refinement
 *   - Chapter/Guild knowledge sharing
 */

export type SprintStatus = "planning" | "active" | "review" | "retrospective" | "closed";
export type TaskStatus = "backlog" | "todo" | "in_progress" | "review" | "done" | "blocked";
export type TaskPriority = "critical" | "high" | "medium" | "low";
export type CeremonyType = "standup" | "planning" | "review" | "retro" | "refinement" | "guild";

export interface SprintTask {
  id: string;
  title: string;
  description: string;
  status: TaskStatus;
  priority: TaskPriority;
  assignedTo: string[]; // team member IDs
  assignedRole: string; // primary role (agency role)
  storyPoints: number;
  acceptanceCriteria: string[];
  createdAt: string;
  startedAt?: string;
  completedAt?: string;
  blockedBy?: string[];
  dependencies?: string[];
}

export interface Sprint {
  id: string;
  number: number;
  name: string;
  goal: string;
  status: SprintStatus;
  startDate: string;
  endDate: string;
  tasks: SprintTask[];
  velocity: number; // story points completed
  capacity: number; // estimated available story points
}

export interface Ceremony {
  type: CeremonyType;
  date: string;
  duration: number; // minutes
  facilitator: string;
  participants: string[];
  notes: string;
  actionItems: string[];
}

export interface SprintBoard {
  currentSprint: Sprint;
  backlog: SprintTask[];
  nextSprint: SprintTask[]; // groomed backlog for next sprint
  ceremonies: Ceremony[];
  teamVelocity: number[]; // last 5 sprints' velocities
}

/**
 * Create a new sprint from a backlog
 */
export function createSprint(
  number: number,
  goal: string,
  backlogItems: SprintTask[],
  capacity: number,
): Sprint {
  // Pull top-priority items from backlog up to capacity
  const sorted = [...backlogItems].sort((a, b) => {
    const priorityOrder: Record<TaskPriority, number> = {
      critical: 0, high: 1, medium: 2, low: 3,
    };
    return priorityOrder[a.priority] - priorityOrder[b.priority];
  });

  let pointsUsed = 0;
  const sprintTasks: SprintTask[] = [];

  for (const task of sorted) {
    if (pointsUsed + task.storyPoints <= capacity) {
      sprintTasks.push({ ...task, status: "todo" });
      pointsUsed += task.storyPoints;
    }
  }

  const now = new Date();
  const endDate = new Date(now);
  endDate.setDate(endDate.getDate() + 14); // 2-week sprint

  return {
    id: `sprint-${number}`,
    number,
    name: `Sprint ${number}`,
    goal,
    status: "planning",
    startDate: now.toISOString(),
    endDate: endDate.toISOString(),
    tasks: sprintTasks,
    velocity: 0,
    capacity,
  };
}

/**
 * Start sprint — move from planning to active
 */
export function startSprint(sprint: Sprint): Sprint {
  return { ...sprint, status: "active" };
}

/**
 * Daily standup template
 */
export function dailyStandup(sprint: Sprint, date: string): Ceremony {
  return {
    type: "standup",
    date,
    duration: 15,
    facilitator: "pm-1", // Olivia — Senior PM
    participants: sprint.tasks.flatMap((t) => t.assignedTo),
    notes: "",
    actionItems: [
      "What did you do yesterday?",
      "What will you do today?",
      "Any blockers?",
    ],
  };
}

/**
 * Sprint review template
 */
export function sprintReview(sprint: Sprint): Ceremony {
  return {
    type: "review",
    date: sprint.endDate,
    duration: 60,
    facilitator: "pm-1",
    participants: sprint.tasks.flatMap((t) => t.assignedTo),
    notes: `Review of ${sprint.name}: "${sprint.goal}"`,
    actionItems: [
      "Demo completed work",
      "Review sprint goal achievement",
      "Collect stakeholder feedback",
      "Update product backlog",
    ],
  };
}

/**
 * Sprint retrospective template
 */
export function sprintRetrospective(sprint: Sprint): Ceremony {
  return {
    type: "retro",
    date: sprint.endDate,
    duration: 90,
    facilitator: "pm-3", // Aisha — Scrum Master
    participants: sprint.tasks.flatMap((t) => t.assignedTo),
    notes: `Retrospective for ${sprint.name}`,
    actionItems: [
      "What went well?",
      "What could be improved?",
      "What will we commit to improving next sprint?",
    ],
  };
}

/**
 * Create a new task for the backlog
 */
export function createTask(
  title: string,
  description: string,
  role: string,
  priority: TaskPriority,
  storyPoints: number,
  acceptanceCriteria: string[],
): SprintTask {
  return {
    id: `task-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    title,
    description,
    status: "backlog",
    priority,
    assignedTo: [],
    assignedRole: role,
    storyPoints,
    acceptanceCriteria,
    createdAt: new Date().toISOString(),
  };
}

/**
 * Assign a task to team members
 */
export function assignTask(task: SprintTask, memberIds: string[]): SprintTask {
  return { ...task, assignedTo: memberIds, status: "todo" };
}

/**
 * Move a task through the workflow
 */
export function transitionTask(
  task: SprintTask,
  newStatus: TaskStatus,
): SprintTask {
  const now = new Date().toISOString();
  return {
    ...task,
    status: newStatus,
    ...(newStatus === "in_progress" && !task.startedAt ? { startedAt: now } : {}),
    ...(newStatus === "done" ? { completedAt: now } : {}),
  };
}

/**
 * Get sprint progress as percentage
 */
export function sprintProgress(sprint: Sprint): number {
  if (sprint.tasks.length === 0) return 0;
  const done = sprint.tasks.filter((t) => t.status === "done").length;
  return Math.round((done / sprint.tasks.length) * 100);
}

/**
 * Get burndown data — ideal vs actual remaining story points
 */
export function burndownData(sprint: Sprint): { day: number; ideal: number; actual: number }[] {
  const totalDays = 10; // 2-week sprint = 10 working days
  const totalPoints = sprint.tasks.reduce((s, t) => s + t.storyPoints, 0);
  const completedPoints = sprint.tasks
    .filter((t) => t.status === "done")
    .reduce((s, t) => s + t.storyPoints, 0);

  const data: { day: number; ideal: number; actual: number }[] = [];
  for (let day = 0; day <= totalDays; day++) {
    const idealRemaining = totalPoints - (totalPoints / totalDays) * day;
    const actualRemaining = day === totalDays ? totalPoints - completedPoints : idealRemaining;
    data.push({ day, ideal: Math.round(idealRemaining), actual: Math.round(actualRemaining) });
  }
  return data;
}
