use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
#[allow(unused_imports)]
use chrono::Utc;
use serde::{Deserialize, Serialize};

// ============================================================================
// Data types
// ============================================================================

/// PROJECT.md equivalent — project vision and constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub framework: Option<String>,
    #[serde(default)]
    pub decisions: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A single requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,       // e.g. "REQ-001"
    pub description: String,
    pub status: ReqStatus,
    pub priority: u32,   // 1 (highest) - 5 (lowest)
    #[serde(default)]
    pub phase: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReqStatus {
    Proposed,
    Accepted,
    Implemented,
    Deferred,
}

/// REQUIREMENTS.md equivalent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementsDoc {
    pub project_name: String,
    #[serde(default)]
    pub requirements: Vec<Requirement>,
}

/// ROADMAP.md phase entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub number: u32,
    pub name: String,
    pub description: String,
    pub status: PhaseStatus,
    #[serde(default)]
    pub dependencies: Vec<u32>,
    #[serde(default)]
    pub estimated_plans: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseStatus {
    Pending,
    InProgress,
    Completed,
}

/// ROADMAP.md equivalent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Roadmap {
    pub project_name: String,
    #[serde(default)]
    pub phases: Vec<Phase>,
    #[serde(default)]
    pub current_phase: Option<u32>,
}

/// STATE.md equivalent — living project memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub phase_status: String,
    pub current_work: String,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub decisions: Vec<String>,
    #[serde(default)]
    pub metrics: HashMap<String, String>,
    pub last_updated: String,
}

/// Per-plan PLAN.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanFile {
    pub plan_id: String,       // e.g. "01-02"
    pub phase_number: u32,
    pub title: String,
    pub description: String,
    pub status: PlanStatus,
    #[serde(default)]
    pub tasks: Vec<String>,
    #[serde(default)]
    pub estimated_effort: String,
    #[serde(default)]
    pub assigned_agent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Planned,
    InProgress,
    Completed,
    Failed,
}

/// CONTEXT.md — per-phase discussion context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionContext {
    pub phase_number: u32,
    #[serde(default)]
    pub decisions: Vec<(String, String)>, // (question, answer)
    #[serde(default)]
    pub assumptions: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

// ============================================================================
// Phase pipeline
// ============================================================================

/// What action to take next in the workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseAction {
    Discuss(u32),
    Plan(u32),
    Execute(u32),
    Verify(u32),
    Ship(u32),
    Complete,
    Idle,
}

/// Workflow state machine: determines the next action.
pub struct PhasePipeline;

impl PhasePipeline {
    /// Determine the next logical action.
    #[must_use]
    pub fn next_action(state: &ProjectState, roadmap: &Roadmap) -> PhaseAction {
        // Find the current phase
        let current = roadmap
            .current_phase
            .and_then(|n| roadmap.phases.iter().find(|p| p.number == n));

        // Find the first non-completed phase
        let next = roadmap
            .phases
            .iter()
            .find(|p| p.status != PhaseStatus::Completed);

        // Check blockers
        let has_blockers = !state.blockers.is_empty();

        match (current, next) {
            (None, None) => PhaseAction::Idle,
            (None, Some(next_phase)) => {
                if has_blockers {
                    PhaseAction::Idle
                } else {
                    PhaseAction::Discuss(next_phase.number)
                }
            }
            (Some(curr), _) if curr.status == PhaseStatus::Completed => {
                // Move to next phase
                next.map_or(PhaseAction::Complete, |n| {
                    if has_blockers {
                        PhaseAction::Idle
                    } else {
                        PhaseAction::Discuss(n.number)
                    }
                })
            }
            (Some(curr), _) => match curr.status {
                PhaseStatus::Pending => {
                    if has_blockers {
                        PhaseAction::Idle
                    } else {
                        PhaseAction::Discuss(curr.number)
                    }
                }
                PhaseStatus::InProgress => {
                    // Check if plans exist and their statuses
                    // For now, suggest execute
                    PhaseAction::Execute(curr.number)
                }
                PhaseStatus::Completed => PhaseAction::Verify(curr.number),
            },
        }
    }
}

// ============================================================================
// Planning directory manager
// ============================================================================

/// Manages the `.deepseek/planning/` directory.
pub struct PlanningDir {
    base_dir: PathBuf,
}

impl PlanningDir {
    /// Initialize the planning directory structure.
    pub fn init(base_dir: PathBuf) -> Result<Self> {
        let pd = Self { base_dir };

        // Create directory structure
        std::fs::create_dir_all(&pd.base_dir)?;
        std::fs::create_dir_all(pd.base_dir.join("phases"))?;
        std::fs::create_dir_all(pd.base_dir.join("research"))?;
        std::fs::create_dir_all(pd.base_dir.join("learnings"))?;
        std::fs::create_dir_all(pd.base_dir.join("spikes"))?;

        Ok(pd)
    }

    /// Return the base directory.
    #[must_use]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Path to a phase directory.
    #[must_use]
    pub fn phase_dir(&self, phase: u32) -> PathBuf {
        self.base_dir.join("phases").join(format!("{:02}", phase))
    }

    // -- Project manifest -----------------------------------------------

    fn project_path(&self) -> PathBuf {
        self.base_dir.join("project.json")
    }

    pub fn write_project(&self, manifest: &ProjectManifest) -> Result<()> {
        let json = serde_json::to_string_pretty(manifest)?;
        std::fs::write(self.project_path(), json)?;
        Ok(())
    }

    pub fn read_project(&self) -> Result<ProjectManifest> {
        let json = std::fs::read_to_string(self.project_path())
            .context("project.json not found — run init first")?;
        Ok(serde_json::from_str(&json)?)
    }

    // -- Requirements ---------------------------------------------------

    fn requirements_path(&self) -> PathBuf {
        self.base_dir.join("requirements.json")
    }

    pub fn write_requirements(&self, doc: &RequirementsDoc) -> Result<()> {
        let json = serde_json::to_string_pretty(doc)?;
        std::fs::write(self.requirements_path(), json)?;
        Ok(())
    }

    pub fn read_requirements(&self) -> Result<RequirementsDoc> {
        let json = std::fs::read_to_string(self.requirements_path())
            .unwrap_or_else(|_| "{\"project_name\":\"\",\"requirements\":[]}".into());
        Ok(serde_json::from_str(&json)?)
    }

    pub fn add_requirement(&self, req: Requirement) -> Result<()> {
        let mut doc = self.read_requirements()?;
        doc.requirements.push(req);
        self.write_requirements(&doc)
    }

    // -- Roadmap --------------------------------------------------------

    fn roadmap_path(&self) -> PathBuf {
        self.base_dir.join("roadmap.json")
    }

    pub fn write_roadmap(&self, roadmap: &Roadmap) -> Result<()> {
        let json = serde_json::to_string_pretty(roadmap)?;
        std::fs::write(self.roadmap_path(), json)?;
        Ok(())
    }

    pub fn read_roadmap(&self) -> Result<Roadmap> {
        let json = std::fs::read_to_string(self.roadmap_path())
            .unwrap_or_else(|_| "{\"project_name\":\"\",\"phases\":[]}".into());
        Ok(serde_json::from_str(&json)?)
    }

    // -- State ----------------------------------------------------------

    fn state_path(&self) -> PathBuf {
        self.base_dir.join("state.json")
    }

    pub fn write_state(&self, state: &ProjectState) -> Result<()> {
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write(self.state_path(), json)?;
        Ok(())
    }

    pub fn read_state(&self) -> Result<ProjectState> {
        let json = std::fs::read_to_string(self.state_path())
            .context("state.json not found")?;
        Ok(serde_json::from_str(&json)?)
    }

    // -- Plans ----------------------------------------------------------

    pub fn write_plan(&self, plan: &PlanFile) -> Result<()> {
        let dir = self.phase_dir(plan.phase_number);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}-PLAN.json", plan.plan_id));
        let json = serde_json::to_string_pretty(plan)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn read_plan(&self, phase: u32, plan_id: &str) -> Result<PlanFile> {
        let path = self.phase_dir(phase).join(format!("{plan_id}-PLAN.json"));
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("plan not found: {}", path.display()))?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn list_plans(&self, phase: u32) -> Result<Vec<PlanFile>> {
        let dir = self.phase_dir(phase);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut plans = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with("-PLAN.json") {
                let json = std::fs::read_to_string(entry.path())?;
                if let Ok(plan) = serde_json::from_str::<PlanFile>(&json) {
                    plans.push(plan);
                }
            }
        }
        plans.sort_by_key(|p| p.plan_id.clone());
        Ok(plans)
    }

    // -- Discussion context ---------------------------------------------

    pub fn write_context(&self, ctx: &DiscussionContext) -> Result<()> {
        let dir = self.phase_dir(ctx.phase_number);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("CONTEXT.json");
        let json = serde_json::to_string_pretty(ctx)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn read_context(&self, phase: u32) -> Result<DiscussionContext> {
        let path = self.phase_dir(phase).join("CONTEXT.json");
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| format!("{{\"phase_number\":{phase},\"decisions\":[],\"assumptions\":[],\"notes\":[]}}"));
        Ok(serde_json::from_str(&json)?)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_planning() -> (tempfile::TempDir, PlanningDir) {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let pd = PlanningDir::init(dir.path().join(".deepseek/planning")).expect("init");
        (dir, pd)
    }

    fn sample_project() -> ProjectManifest {
        ProjectManifest {
            name: "test-project".into(),
            description: "A test project".into(),
            version: "0.1.0".into(),
            repository: Some("https://github.com/test/project".into()),
            language: Some("Rust".into()),
            framework: None,
            decisions: vec!["Use SQLite".into()],
            constraints: vec!["Must support WASM".into()],
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    // ------------------------------------------------------------

    #[test]
    fn init_creates_directory_structure() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let base = dir.path().join(".deepseek/planning");
        let pd = PlanningDir::init(base.clone()).expect("init");

        assert!(base.exists());
        assert!(base.join("phases").exists());
        assert!(base.join("research").exists());
        assert!(base.join("learnings").exists());
        assert!(base.join("spikes").exists());
    }

    #[test]
    fn write_and_read_project_manifest() {
        let (_dir, pd) = temp_planning();
        let manifest = sample_project();
        pd.write_project(&manifest).expect("write");

        let read = pd.read_project().expect("read");
        assert_eq!(read.name, "test-project");
        assert_eq!(read.language, Some("Rust".into()));
        assert_eq!(read.decisions, vec!["Use SQLite"]);
    }

    #[test]
    fn add_and_list_requirements() {
        let (_dir, pd) = temp_planning();

        pd.add_requirement(Requirement {
            id: "REQ-001".into(),
            description: "User authentication".into(),
            status: ReqStatus::Proposed,
            priority: 1,
            phase: Some(1),
        }).expect("add");

        pd.add_requirement(Requirement {
            id: "REQ-002".into(),
            description: "Dark mode".into(),
            status: ReqStatus::Accepted,
            priority: 3,
            phase: None,
        }).expect("add");

        let doc = pd.read_requirements().expect("read");
        assert_eq!(doc.requirements.len(), 2);
        assert_eq!(doc.requirements[0].id, "REQ-001");
        assert_eq!(doc.requirements[1].status, ReqStatus::Accepted);
    }

    #[test]
    fn write_and_read_roadmap() {
        let (_dir, pd) = temp_planning();

        let roadmap = Roadmap {
            project_name: "test".into(),
            phases: vec![
                Phase {
                    number: 1,
                    name: "Core Setup".into(),
                    description: "Set up project structure".into(),
                    status: PhaseStatus::Completed,
                    dependencies: vec![],
                    estimated_plans: 2,
                },
                Phase {
                    number: 2,
                    name: "Auth System".into(),
                    description: "Implement authentication".into(),
                    status: PhaseStatus::InProgress,
                    dependencies: vec![1],
                    estimated_plans: 3,
                },
            ],
            current_phase: Some(2),
        };
        pd.write_roadmap(&roadmap).expect("write");

        let read = pd.read_roadmap().expect("read");
        assert_eq!(read.phases.len(), 2);
        assert_eq!(read.current_phase, Some(2));
        assert_eq!(read.phases[1].status, PhaseStatus::InProgress);
    }

    #[test]
    fn write_and_read_state() {
        let (_dir, pd) = temp_planning();
        let mut metrics = HashMap::new();
        metrics.insert("tests_passing".into(), "42".into());

        let state = ProjectState {
            phase_status: "executing phase 1".into(),
            current_work: "implementing auth handler".into(),
            blockers: vec!["Waiting for API key".into()],
            decisions: vec!["Use JWT".into()],
            metrics,
            last_updated: Utc::now().to_rfc3339(),
        };
        pd.write_state(&state).expect("write");

        let read = pd.read_state().expect("read");
        assert_eq!(read.phase_status, "executing phase 1");
        assert_eq!(read.blockers.len(), 1);
        assert_eq!(read.metrics.get("tests_passing").unwrap(), "42");
    }

    #[test]
    fn write_and_read_plan() {
        let (_dir, pd) = temp_planning();

        let plan = PlanFile {
            plan_id: "01-02".into(),
            phase_number: 1,
            title: "Database schema".into(),
            description: "Create the initial database schema".into(),
            status: PlanStatus::Planned,
            tasks: vec!["Create migrations".into(), "Add models".into()],
            estimated_effort: "2h".into(),
            assigned_agent: None,
        };
        pd.write_plan(&plan).expect("write");

        let read = pd.read_plan(1, "01-02").expect("read");
        assert_eq!(read.title, "Database schema");
        assert_eq!(read.tasks.len(), 2);

        let plans = pd.list_plans(1).expect("list");
        assert_eq!(plans.len(), 1);
    }

    #[test]
    fn phase_pipeline_routing() {
        let state = ProjectState {
            phase_status: "ready".into(),
            current_work: "".into(),
            blockers: vec![],
            decisions: vec![],
            metrics: HashMap::new(),
            last_updated: Utc::now().to_rfc3339(),
        };

        let roadmap = Roadmap {
            project_name: "test".into(),
            phases: vec![
                Phase {
                    number: 1,
                    name: "Phase 1".into(),
                    description: "First phase".into(),
                    status: PhaseStatus::Pending,
                    dependencies: vec![],
                    estimated_plans: 2,
                },
            ],
            current_phase: None,
        };

        // First phase is pending — should discuss
        let action = PhasePipeline::next_action(&state, &roadmap);
        assert_eq!(action, PhaseAction::Discuss(1));

        // With blockers — should idle
        let blocked_state = ProjectState {
            blockers: vec!["blocked".into()],
            ..state.clone()
        };
        let action = PhasePipeline::next_action(&blocked_state, &roadmap);
        assert_eq!(action, PhaseAction::Idle);
    }
}
