use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

// ============================================================================
// Plugin types
// ============================================================================

/// Plugin category for grouping and namespace routing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginCategory {
    Workflow,
    Quality,
    Context,
    Manage,
    Ideate,
    Custom(String),
}

impl PluginCategory {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Workflow => "workflow",
            Self::Quality => "quality",
            Self::Context => "context",
            Self::Manage => "manage",
            Self::Ideate => "ideate",
            Self::Custom(s) => s.as_str(),
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s {
            "workflow" => Self::Workflow,
            "quality" => Self::Quality,
            "context" => Self::Context,
            "manage" => Self::Manage,
            "ideate" => Self::Ideate,
            other => Self::Custom(other.to_string()),
        }
    }
}

/// Metadata about an installed plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool { true }

/// A loaded skill definition (from SKILL.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDef {
    pub name: String,
    pub description: String,
    pub category: String,
    /// The prompt template body (everything after YAML frontmatter).
    pub prompt_template: String,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
}

// ============================================================================
// Plugin errors
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    NotFound { name: String },
    AlreadyExists { name: String },
    InvalidManifest { name: String, reason: String },
    SkillNotFound { plugin: String, skill: String },
    IoError(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { name } => write!(f, "plugin not found: {name}"),
            Self::AlreadyExists { name } => write!(f, "plugin already exists: {name}"),
            Self::InvalidManifest { name, reason } => write!(f, "invalid manifest for {name}: {reason}"),
            Self::SkillNotFound { plugin, skill } => write!(f, "skill {skill} not found in plugin {plugin}"),
            Self::IoError(msg) => write!(f, "I/O error: {msg}"),
        }
    }
}

impl std::error::Error for PluginError {}

// ============================================================================
// Skill loader
// ============================================================================

/// Parses SKILL.md files with YAML frontmatter.
pub struct SkillLoader;

impl SkillLoader {
    /// Load a skill definition from a SKILL.md file.
    ///
    /// Format:
    /// ```text
    /// ---
    /// name: skill-name
    /// description: What this skill does
    /// category: workflow
    /// allowed_tools: read_file,write_file
    /// ---
    /// The prompt template body goes here.
    /// Multiple lines are supported.
    /// ```
    pub fn load(path: &Path) -> Result<SkillDef> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read skill file: {}", path.display()))?;

        Self::parse(&content)
            .with_context(|| format!("failed to parse skill: {}", path.display()))
    }

    /// Parse skill definition from string content.
    pub fn parse(content: &str) -> Result<SkillDef> {
        // Split on "---" markers
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            bail!("invalid SKILL.md: missing frontmatter delimiters (expected --- ... ---)");
        }

        let frontmatter = parts[1].trim();
        let body = parts[2].trim().to_string();

        let mut name = String::new();
        let mut description = String::new();
        let mut category = "workflow".to_string();
        let mut allowed_tools = Vec::new();

        for line in frontmatter.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "name" => name = value.to_string(),
                    "description" => description = value.to_string(),
                    "category" => category = value.to_string(),
                    "allowed_tools" => {
                        allowed_tools = value.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    _ => {}
                }
            }
        }

        if name.is_empty() {
            bail!("skill missing required 'name' field in frontmatter");
        }

        Ok(SkillDef {
            name,
            description,
            category,
            prompt_template: body,
            allowed_tools,
        })
    }
}

// ============================================================================
// Plugin registry
// ============================================================================

/// Manages installed plugins in a plugins directory.
///
/// Directory structure:
/// ```text
/// plugins_dir/
///   {plugin_name}/
///     manifest.json        — PluginManifest
///     skills/
///       {skill_name}.md    — SKILL.md with YAML frontmatter
/// ```
pub struct PluginRegistry {
    plugins_dir: PathBuf,
    /// In-memory cache of discovered plugins.
    manifests: HashMap<String, PluginManifest>,
}

impl PluginRegistry {
    /// Create a new registry rooted at `plugins_dir`.
    #[must_use]
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            manifests: HashMap::new(),
        }
    }

    /// Discover all installed plugins by scanning the plugins directory.
    pub fn discover(&mut self) -> Result<Vec<PluginManifest>> {
        if !self.plugins_dir.exists() {
            std::fs::create_dir_all(&self.plugins_dir)?;
            return Ok(Vec::new());
        }

        self.manifests.clear();

        for entry in std::fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let manifest_path = entry.path().join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            let json = std::fs::read_to_string(&manifest_path)?;
            if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&json) {
                self.manifests.insert(manifest.name.clone(), manifest);
            }
        }

        let mut manifests: Vec<_> = self.manifests.values().cloned().collect();
        manifests.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(manifests)
    }

    /// Install a plugin.
    pub fn install_plugin(&mut self, manifest: PluginManifest) -> Result<()> {
        if self.manifests.contains_key(&manifest.name) {
            return Err(PluginError::AlreadyExists { name: manifest.name }.into());
        }

        let dir = self.plugins_dir.join(&manifest.name);
        std::fs::create_dir_all(dir.join("skills"))?;

        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(dir.join("manifest.json"), manifest_json)?;

        self.manifests.insert(manifest.name.clone(), manifest);
        Ok(())
    }

    /// Uninstall a plugin.
    pub fn uninstall_plugin(&mut self, name: &str) -> Result<()> {
        let dir = self.plugins_dir.join(name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        self.manifests.remove(name);
        Ok(())
    }

    /// Enable a plugin.
    pub fn enable_plugin(&mut self, name: &str) -> Result<()> {
        {
            let manifest = self.manifests.get_mut(name)
                .ok_or_else(|| PluginError::NotFound { name: name.to_string() })?;
            manifest.enabled = true;
        }
        let manifest = self.manifests.get(name)
            .ok_or_else(|| PluginError::NotFound { name: name.to_string() })?;
        let json = serde_json::to_string_pretty(manifest)?;
        std::fs::write(self.plugins_dir.join(name).join("manifest.json"), json)?;
        Ok(())
    }

    /// Disable a plugin.
    pub fn disable_plugin(&mut self, name: &str) -> Result<()> {
        {
            let manifest = self.manifests.get_mut(name)
                .ok_or_else(|| PluginError::NotFound { name: name.to_string() })?;
            manifest.enabled = false;
        }
        let manifest = self.manifests.get(name)
            .ok_or_else(|| PluginError::NotFound { name: name.to_string() })?;
        let json = serde_json::to_string_pretty(manifest)?;
        std::fs::write(self.plugins_dir.join(name).join("manifest.json"), json)?;
        Ok(())
    }

    /// Load a specific skill from a plugin.
    pub fn load_skill(&self, plugin_name: &str, skill_name: &str) -> Result<SkillDef> {
        let skill_path = self.plugins_dir
            .join(plugin_name)
            .join("skills")
            .join(format!("{skill_name}.md"));

        if !skill_path.exists() {
            return Err(PluginError::SkillNotFound {
                plugin: plugin_name.to_string(),
                skill: skill_name.to_string(),
            }.into());
        }

        SkillLoader::load(&skill_path)
    }

    /// List skill names for a plugin.
    pub fn list_skills(&self, plugin_name: &str) -> Result<Vec<String>> {
        let skills_dir = self.plugins_dir.join(plugin_name).join("skills");
        if !skills_dir.exists() {
            return Ok(Vec::new());
        }

        let mut skills = Vec::new();
        for entry in std::fs::read_dir(&skills_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") {
                skills.push(name.trim_end_matches(".md").to_string());
            }
        }
        skills.sort();
        Ok(skills)
    }

    /// List all skills from all plugins as (plugin_name, skill_name) pairs.
    pub fn list_all_skills(&self) -> Vec<(String, String)> {
        let mut all = Vec::new();
        for name in self.manifests.keys() {
            if let Ok(skills) = self.list_skills(name) {
                for skill in skills {
                    all.push((name.clone(), skill));
                }
            }
        }
        all.sort();
        all
    }

    /// Search skills by name and description.
    pub fn search_skills(&self, query: &str) -> Vec<SkillDef> {
        let lower = query.to_lowercase();
        let mut results = Vec::new();

        for (plugin_name, _manifest) in &self.manifests {
            if let Ok(skills) = self.list_skills(plugin_name) {
                for skill_name in skills {
                    if let Ok(skill) = self.load_skill(plugin_name, &skill_name) {
                        if skill.name.to_lowercase().contains(&lower)
                            || skill.description.to_lowercase().contains(&lower)
                            || skill.category.to_lowercase().contains(&lower)
                        {
                            results.push(skill);
                        }
                    }
                }
            }
        }

        results
    }

    /// Get a plugin manifest by name.
    #[must_use]
    pub fn get_plugin(&self, name: &str) -> Option<&PluginManifest> {
        self.manifests.get(name)
    }

    /// Get the plugins directory path.
    #[must_use]
    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    // -- Internals -----------------------------------------------------

    #[allow(dead_code)]
    fn save_manifest(&self, name: &str, manifest: &PluginManifest) -> Result<()> {
        let path = self.plugins_dir.join(name).join("manifest.json");
        let json = serde_json::to_string_pretty(manifest)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_plugin_dir(plugins_dir: &Path, name: &str, skills: &[(&str, &str, &str, &str)]) {
        let dir = plugins_dir.join(name);
        std::fs::create_dir_all(dir.join("skills")).unwrap();

        let manifest = PluginManifest {
            name: name.to_string(),
            version: "1.0.0".into(),
            description: format!("{name} plugin"),
            author: "test".into(),
            skills: skills.iter().map(|(s, _, _, _)| s.to_string()).collect(),
            dependencies: vec![],
            enabled: true,
        };
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        ).unwrap();

        for (skill_name, desc, cat, tools) in skills {
            let content = format!(
                "---\nname: {skill_name}\ndescription: {desc}\ncategory: {cat}\nallowed_tools: {tools}\n---\nPrompt body for {skill_name}\n"
            );
            std::fs::write(dir.join("skills").join(format!("{skill_name}.md")), content).unwrap();
        }
    }

    #[test]
    fn discover_plugins_finds_installed() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugins_dir = dir.path().join("plugins");
        make_plugin_dir(&plugins_dir, "test-plugin", &[
            ("explore", "Explore codebase", "workflow", "read_file,grep_files"),
        ]);

        let mut registry = PluginRegistry::new(plugins_dir);
        let plugins = registry.discover().unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "test-plugin");
    }

    #[test]
    fn load_skill_parses_frontmatter() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugins_dir = dir.path().join("plugins");
        make_plugin_dir(&plugins_dir, "core", &[
            ("plan-phase", "Plan a development phase", "workflow", "read_file,write_file"),
        ]);

        let mut registry = PluginRegistry::new(plugins_dir);
        registry.discover().unwrap();

        let skill = registry.load_skill("core", "plan-phase").unwrap();
        assert_eq!(skill.name, "plan-phase");
        assert_eq!(skill.description, "Plan a development phase");
        assert_eq!(skill.category, "workflow");
        assert_eq!(skill.allowed_tools, vec!["read_file", "write_file"]);
        assert!(skill.prompt_template.contains("Prompt body"));
    }

    #[test]
    fn enable_disable_plugin() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugins_dir = dir.path().join("plugins");
        make_plugin_dir(&plugins_dir, "toggle-test", &[
            ("test", "Test skill", "quality", ""),
        ]);

        let mut registry = PluginRegistry::new(plugins_dir);
        registry.discover().unwrap();

        assert!(registry.get_plugin("toggle-test").unwrap().enabled);

        registry.disable_plugin("toggle-test").unwrap();
        assert!(!registry.get_plugin("toggle-test").unwrap().enabled);

        registry.enable_plugin("toggle-test").unwrap();
        assert!(registry.get_plugin("toggle-test").unwrap().enabled);
    }

    #[test]
    fn search_skills_finds_match() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugins_dir = dir.path().join("plugins");
        make_plugin_dir(&plugins_dir, "workflow-tools", &[
            ("discuss-phase", "Gather phase context", "workflow", ""),
            ("plan-phase", "Create implementation plan", "workflow", ""),
        ]);
        make_plugin_dir(&plugins_dir, "quality-tools", &[
            ("code-review", "Review code for bugs", "quality", ""),
        ]);

        let mut registry = PluginRegistry::new(plugins_dir);
        registry.discover().unwrap();

        let results = registry.search_skills("phase");
        assert_eq!(results.len(), 2); // discuss-phase and plan-phase

        let results = registry.search_skills("review");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "code-review");
    }

    #[test]
    fn install_uninstall_plugin() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugins_dir = dir.path().join("plugins");
        let mut registry = PluginRegistry::new(plugins_dir.clone());

        let manifest = PluginManifest {
            name: "new-plugin".into(),
            version: "0.1.0".into(),
            description: "A new plugin".into(),
            author: "test".into(),
            skills: vec!["hello".into()],
            dependencies: vec![],
            enabled: true,
        };

        registry.install_plugin(manifest).unwrap();
        assert!(registry.get_plugin("new-plugin").is_some());

        registry.uninstall_plugin("new-plugin").unwrap();
        assert!(registry.get_plugin("new-plugin").is_none());
    }

    #[test]
    fn skill_loader_rejects_missing_frontmatter() {
        let result = SkillLoader::parse("just a body with no frontmatter");
        assert!(result.is_err());
    }
}
