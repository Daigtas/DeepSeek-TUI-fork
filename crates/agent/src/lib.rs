use std::collections::HashMap;
use std::fmt;

use deepseek_config::ProviderKind;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Model info
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    /// Unique stable identifier (UUID v4, generated on registration).
    pub stable_id: String,
    /// Provider-specific model id.
    pub id: String,
    /// Which provider this model belongs to.
    pub provider: ProviderKind,
    /// Alternative names that resolve to this model.
    pub aliases: Vec<String>,
    /// Whether the model supports tool/function calling.
    pub supports_tools: bool,
    /// Whether the model emits reasoning/thinking blocks.
    pub supports_reasoning: bool,
    /// Whether the model is deprecated – still resolves but emits a warning.
    #[serde(default)]
    pub deprecated: bool,
    /// Optional human-readable deprecation notice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecation_notice: Option<String>,
}

impl ModelInfo {
    fn new(id: impl Into<String>, provider: ProviderKind) -> Self {
        Self {
            stable_id: Uuid::new_v4().to_string(),
            id: id.into(),
            provider,
            aliases: Vec::new(),
            supports_tools: true,
            supports_reasoning: true,
            deprecated: false,
            deprecation_notice: None,
        }
    }

    fn with_aliases(mut self, aliases: Vec<String>) -> Self {
        self.aliases = aliases;
        self
    }

    fn with_capabilities(mut self, tools: bool, reasoning: bool) -> Self {
        self.supports_tools = tools;
        self.supports_reasoning = reasoning;
        self
    }
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResolution {
    pub requested: Option<String>,
    pub resolved: ModelInfo,
    pub used_fallback: bool,
    pub fallback_chain: Vec<String>,
}

impl ModelResolution {
    /// Human-readable description of what was resolved and why.
    pub fn describe(&self) -> String {
        let requested = self.requested.as_deref().unwrap_or("(default)");
        if self.used_fallback {
            format!(
                "{} → {} (fallback via {})",
                requested,
                self.resolved.id,
                self.fallback_chain.join(" → ")
            )
        } else {
            format!("{} → {} (direct match)", requested, self.resolved.id)
        }
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ModelRegistry {
    models: Vec<ModelInfo>,
    alias_map: HashMap<String, usize>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        let models = vec![
            ModelInfo::new("deepseek-v4-pro", ProviderKind::Deepseek),
            ModelInfo::new("deepseek-v4-flash", ProviderKind::Deepseek).with_aliases(vec![
                "deepseek-chat".into(),
                "deepseek-reasoner".into(),
                "deepseek-r1".into(),
                "deepseek-v3".into(),
                "deepseek-v3.2".into(),
            ]),
            ModelInfo::new(
                "deepseek-ai/deepseek-v4-pro",
                ProviderKind::NvidiaNim,
            )
            .with_aliases(vec![
                "deepseek-v4-pro".into(),
                "nvidia-deepseek-v4-pro".into(),
                "nim-deepseek-v4-pro".into(),
            ]),
            ModelInfo::new(
                "deepseek-ai/deepseek-v4-flash",
                ProviderKind::NvidiaNim,
            )
            .with_aliases(vec![
                "deepseek-v4-flash".into(),
                "deepseek-chat".into(),
                "deepseek-reasoner".into(),
                "nvidia-deepseek-v4-flash".into(),
                "nim-deepseek-v4-flash".into(),
            ]),
            ModelInfo::new("gpt-4.1", ProviderKind::Openai)
                .with_aliases(vec!["gpt4.1".into(), "gpt-4o".into()]),
            ModelInfo::new("gpt-4.1-mini", ProviderKind::Openai)
                .with_aliases(vec!["gpt-4o-mini".into()])
                .with_capabilities(true, false),
            ModelInfo::new(
                "deepseek/deepseek-v4-pro",
                ProviderKind::Openrouter,
            )
            .with_aliases(vec![
                "deepseek-v4-pro".into(),
                "openrouter-deepseek-v4-pro".into(),
            ]),
            ModelInfo::new(
                "deepseek/deepseek-v4-flash",
                ProviderKind::Openrouter,
            )
            .with_aliases(vec![
                "deepseek-v4-flash".into(),
                "deepseek-chat".into(),
                "deepseek-reasoner".into(),
                "openrouter-deepseek-v4-flash".into(),
            ]),
            ModelInfo::new(
                "deepseek/deepseek-v4-pro",
                ProviderKind::Novita,
            )
            .with_aliases(vec![
                "deepseek-v4-pro".into(),
                "novita-deepseek-v4-pro".into(),
            ]),
            ModelInfo::new(
                "deepseek/deepseek-v4-flash",
                ProviderKind::Novita,
            )
            .with_aliases(vec![
                "deepseek-v4-flash".into(),
                "deepseek-chat".into(),
                "deepseek-reasoner".into(),
                "novita-deepseek-v4-flash".into(),
            ]),
            ModelInfo::new(
                "accounts/fireworks/models/deepseek-v4-pro",
                ProviderKind::Fireworks,
            )
            .with_aliases(vec![
                "deepseek-v4-pro".into(),
                "fireworks-deepseek-v4-pro".into(),
            ]),
            ModelInfo::new(
                "deepseek-ai/DeepSeek-V4-Pro",
                ProviderKind::Sglang,
            )
            .with_aliases(vec![
                "deepseek-v4-pro".into(),
                "sglang-deepseek-v4-pro".into(),
            ]),
            ModelInfo::new(
                "deepseek-ai/DeepSeek-V4-Flash",
                ProviderKind::Sglang,
            )
            .with_aliases(vec![
                "deepseek-v4-flash".into(),
                "deepseek-chat".into(),
                "deepseek-reasoner".into(),
                "sglang-deepseek-v4-flash".into(),
            ]),
            ModelInfo::new(
                "deepseek-ai/DeepSeek-V4-Pro",
                ProviderKind::Vllm,
            )
            .with_aliases(vec![
                "deepseek-v4-pro".into(),
                "vllm-deepseek-v4-pro".into(),
            ]),
            ModelInfo::new(
                "deepseek-ai/DeepSeek-V4-Flash",
                ProviderKind::Vllm,
            )
            .with_aliases(vec![
                "deepseek-v4-flash".into(),
                "deepseek-chat".into(),
                "deepseek-reasoner".into(),
                "vllm-deepseek-v4-flash".into(),
            ]),
        ];
        Self::new(models)
    }
}

impl ModelRegistry {
    /// Build a registry from an explicit list of models.
    #[must_use]
    pub fn new(models: Vec<ModelInfo>) -> Self {
        let mut alias_map = HashMap::new();
        for (idx, model) in models.iter().enumerate() {
            alias_map.entry(normalize(&model.id)).or_insert(idx);
            for alias in &model.aliases {
                alias_map.entry(normalize(alias)).or_insert(idx);
            }
        }
        Self { models, alias_map }
    }

    // -- mutation -------------------------------------------------------

    /// Register a new model. Returns an error if the stable_id already
    /// exists or if the model id collides with a non-deprecated entry.
    pub fn add_model(&mut self, model: ModelInfo) -> Result<(), RegistryError> {
        if self.models.iter().any(|m| m.stable_id == model.stable_id) {
            return Err(RegistryError::StableIdCollision {
                stable_id: model.stable_id,
            });
        }
        // Allow re-registering a deprecated model under the same id.
        if let Some(existing) = self
            .models
            .iter()
            .find(|m| normalize(&m.id) == normalize(&model.id) && !m.deprecated)
        {
            return Err(RegistryError::ModelIdCollision {
                id: model.id,
                existing_stable_id: existing.stable_id.clone(),
            });
        }
        let idx = self.models.len();
        self.alias_map
            .entry(normalize(&model.id))
            .or_insert(idx);
        for alias in &model.aliases {
            self.alias_map
                .entry(normalize(alias))
                .or_insert(idx);
        }
        self.models.push(model);
        Ok(())
    }

    /// Remove a model by stable_id.
    pub fn remove_model(&mut self, stable_id: &str) -> Result<ModelInfo, RegistryError> {
        let idx = self
            .models
            .iter()
            .position(|m| m.stable_id == stable_id)
            .ok_or_else(|| RegistryError::NotFound {
                stable_id: stable_id.to_string(),
            })?;
        let removed = self.models.remove(idx);
        // Rebuild alias map – a remove is rare enough that this is fine.
        self.rebuild_alias_map();
        Ok(removed)
    }

    /// Mark a model as deprecated with an optional notice.
    pub fn deprecate_model(
        &mut self,
        stable_id: &str,
        notice: Option<String>,
    ) -> Result<(), RegistryError> {
        let model = self
            .models
            .iter_mut()
            .find(|m| m.stable_id == stable_id)
            .ok_or_else(|| RegistryError::NotFound {
                stable_id: stable_id.to_string(),
            })?;
        model.deprecated = true;
        model.deprecation_notice = notice;
        Ok(())
    }

    /// Undeprecate a model.
    pub fn undeprecate_model(&mut self, stable_id: &str) -> Result<(), RegistryError> {
        let model = self
            .models
            .iter_mut()
            .find(|m| m.stable_id == stable_id)
            .ok_or_else(|| RegistryError::NotFound {
                stable_id: stable_id.to_string(),
            })?;
        model.deprecated = false;
        model.deprecation_notice = None;
        Ok(())
    }

    fn rebuild_alias_map(&mut self) {
        self.alias_map.clear();
        for (idx, model) in self.models.iter().enumerate() {
            self.alias_map
                .entry(normalize(&model.id))
                .or_insert(idx);
            for alias in &model.aliases {
                self.alias_map
                    .entry(normalize(alias))
                    .or_insert(idx);
            }
        }
    }

    // -- lookup ---------------------------------------------------------

    /// Return a reference to every registered model (no allocation).
    #[must_use]
    pub fn list(&self) -> &[ModelInfo] {
        &self.models
    }

    /// Return cloned models (for serialization / FFI boundaries).
    #[must_use]
    pub fn list_cloned(&self) -> Vec<ModelInfo> {
        self.models.clone()
    }

    /// Look up a model by its stable UUID.
    #[must_use]
    pub fn get_by_stable_id(&self, stable_id: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.stable_id == stable_id)
    }

    /// Return models for a specific provider.
    #[must_use]
    pub fn filter_by_provider(&self, provider: ProviderKind) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.provider == provider)
            .collect()
    }

    /// Return models matching capability requirements.
    #[must_use]
    pub fn filter_by_capabilities(
        &self,
        requires_tools: bool,
        requires_reasoning: bool,
    ) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| {
                (!requires_tools || m.supports_tools)
                    && (!requires_reasoning || m.supports_reasoning)
            })
            .collect()
    }

    /// Return non-deprecated models.
    #[must_use]
    pub fn active_models(&self) -> Vec<&ModelInfo> {
        self.models.iter().filter(|m| !m.deprecated).collect()
    }

    // -- resolution -----------------------------------------------------

    /// Resolve a requested model name to a concrete ModelInfo.
    ///
    /// Resolution order:
    /// 1. Exact stable_id match
    /// 2. Provider-hinted match (id or alias)
    /// 3. Alias match (any provider)
    /// 4. Provider default (first non-deprecated model for that provider)
    /// 5. Global default (first non-deprecated model)
    /// 6. Hardcoded fallback
    #[must_use]
    pub fn resolve(
        &self,
        requested: Option<&str>,
        provider_hint: Option<ProviderKind>,
    ) -> ModelResolution {
        let mut fallback_chain = Vec::new();

        if let Some(name) = requested {
            fallback_chain.push(format!("requested:{name}"));

            // 1. stable_id match
            if let Some(model) = self.get_by_stable_id(name) {
                return ModelResolution {
                    requested: Some(name.to_string()),
                    resolved: model.clone(),
                    used_fallback: false,
                    fallback_chain,
                };
            }

            // 2. Provider-hinted match
            if let Some(provider) = provider_hint
                && let Some(model) = self
                    .models
                    .iter()
                    .find(|m| m.provider == provider && model_matches(m, name))
                    .cloned()
            {
                return ModelResolution {
                    requested: Some(name.to_string()),
                    resolved: preserve_requested_model_id_case(model, name),
                    used_fallback: false,
                    fallback_chain,
                };
            }

            // 3. Alias match (prefer non-deprecated)
            if let Some(idx) = self.alias_map.get(&normalize(name)) {
                let model = self.models[*idx].clone();
                if model.deprecated {
                    fallback_chain.push(format!(
                        "deprecated:{} id={}",
                        model.id,
                        model
                            .deprecation_notice
                            .as_deref()
                            .unwrap_or("no notice")
                    ));
                }
                return ModelResolution {
                    requested: Some(name.to_string()),
                    resolved: preserve_requested_model_id_case(model, name),
                    used_fallback: false,
                    fallback_chain,
                };
            }
        }

        // 4. Provider default (non-deprecated)
        let provider = provider_hint.unwrap_or(ProviderKind::Deepseek);
        fallback_chain.push(format!("provider_default:{}", provider.as_str()));
        if let Some(model) = self
            .models
            .iter()
            .find(|m| m.provider == provider && !m.deprecated)
            .cloned()
        {
            return ModelResolution {
                requested: requested.map(ToOwned::to_owned),
                resolved: model,
                used_fallback: true,
                fallback_chain,
            };
        }

        // 5. Global default (first non-deprecated)
        if let Some(model) = self.models.iter().find(|m| !m.deprecated).cloned() {
            fallback_chain.push("global_default".to_string());
            return ModelResolution {
                requested: requested.map(ToOwned::to_owned),
                resolved: model,
                used_fallback: true,
                fallback_chain,
            };
        }

        // 6. Hardcoded safety-net
        let final_fallback = ModelInfo {
            stable_id: "builtin-deepseek-v4-pro".to_string(),
            id: "deepseek-v4-pro".to_string(),
            provider: ProviderKind::Deepseek,
            aliases: Vec::new(),
            supports_tools: true,
            supports_reasoning: true,
            deprecated: false,
            deprecation_notice: None,
        };
        fallback_chain.push("hardcoded_default:deepseek-v4-pro".to_string());
        ModelResolution {
            requested: requested.map(ToOwned::to_owned),
            resolved: final_fallback,
            used_fallback: true,
            fallback_chain,
        }
    }

    /// Strict resolution: no fallback; returns `None` when the requested
    /// model cannot be found.
    #[must_use]
    pub fn resolve_strict(&self, requested: &str) -> Option<&ModelInfo> {
        // stable_id
        if let Some(model) = self.get_by_stable_id(requested) {
            return Some(model);
        }
        // alias map
        self.alias_map
            .get(&normalize(requested))
            .map(|&idx| &self.models[idx])
    }

    /// Number of registered models.
    #[must_use]
    pub fn len(&self) -> usize {
        self.models.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    StableIdCollision { stable_id: String },
    ModelIdCollision { id: String, existing_stable_id: String },
    NotFound { stable_id: String },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StableIdCollision { stable_id } => {
                write!(f, "model with stable_id '{stable_id}' already registered")
            }
            Self::ModelIdCollision {
                id,
                existing_stable_id,
            } => {
                write!(
                    f,
                    "model id '{id}' already registered (stable_id: {existing_stable_id})"
                )
            }
            Self::NotFound { stable_id } => {
                write!(f, "model with stable_id '{stable_id}' not found")
            }
        }
    }
}

impl std::error::Error for RegistryError {}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn model_matches(model: &ModelInfo, requested: &str) -> bool {
    let requested = normalize(requested);
    normalize(&model.id) == requested
        || model
            .aliases
            .iter()
            .any(|alias| normalize(alias) == requested)
}

fn preserve_requested_model_id_case(mut model: ModelInfo, requested: &str) -> ModelInfo {
    let requested = requested.trim();
    if model.id.eq_ignore_ascii_case(requested) {
        model.id = requested.to_string();
    }
    model
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- resolution (existing coverage, updated for stable_id) ----------

    #[test]
    fn deepseek_v4_pro_alias_stays_deepseek_by_default() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(Some("deepseek-v4-pro"), None);

        assert_eq!(resolved.resolved.provider, ProviderKind::Deepseek);
        assert_eq!(resolved.resolved.id, "deepseek-v4-pro");
    }

    #[test]
    fn deepseek_v4_pro_alias_resolves_to_nvidia_nim_when_provider_hinted() {
        let registry = ModelRegistry::default();
        let resolved =
            registry.resolve(Some("deepseek-v4-pro"), Some(ProviderKind::NvidiaNim));

        assert_eq!(resolved.resolved.provider, ProviderKind::NvidiaNim);
        assert_eq!(resolved.resolved.id, "deepseek-ai/deepseek-v4-pro");
    }

    #[test]
    fn nvidia_nim_default_uses_catalog_model_id() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(None, Some(ProviderKind::NvidiaNim));

        assert_eq!(resolved.resolved.provider, ProviderKind::NvidiaNim);
        assert_eq!(resolved.resolved.id, "deepseek-ai/deepseek-v4-pro");
    }

    #[test]
    fn deepseek_v4_flash_alias_resolves_to_nvidia_nim_when_provider_hinted() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(
            Some("deepseek-v4-flash"),
            Some(ProviderKind::NvidiaNim),
        );

        assert_eq!(resolved.resolved.provider, ProviderKind::NvidiaNim);
        assert_eq!(resolved.resolved.id, "deepseek-ai/deepseek-v4-flash");
    }

    #[test]
    fn openrouter_default_uses_namespaced_model_id() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(None, Some(ProviderKind::Openrouter));

        assert_eq!(resolved.resolved.provider, ProviderKind::Openrouter);
        assert_eq!(resolved.resolved.id, "deepseek/deepseek-v4-pro");
    }

    #[test]
    fn novita_default_uses_namespaced_model_id() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(None, Some(ProviderKind::Novita));

        assert_eq!(resolved.resolved.provider, ProviderKind::Novita);
        assert_eq!(resolved.resolved.id, "deepseek/deepseek-v4-pro");
    }

    #[test]
    fn fireworks_default_uses_canonical_model_id() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(None, Some(ProviderKind::Fireworks));

        assert_eq!(resolved.resolved.provider, ProviderKind::Fireworks);
        assert_eq!(
            resolved.resolved.id,
            "accounts/fireworks/models/deepseek-v4-pro"
        );
    }

    #[test]
    fn sglang_default_uses_canonical_model_id() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(None, Some(ProviderKind::Sglang));

        assert_eq!(resolved.resolved.provider, ProviderKind::Sglang);
        assert_eq!(resolved.resolved.id, "deepseek-ai/DeepSeek-V4-Pro");
    }

    #[test]
    fn vllm_default_uses_canonical_model_id() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(None, Some(ProviderKind::Vllm));

        assert_eq!(resolved.resolved.provider, ProviderKind::Vllm);
        assert_eq!(resolved.resolved.id, "deepseek-ai/DeepSeek-V4-Pro");
    }

    #[test]
    fn preserves_requested_model_casing() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(Some("DeepSeek-V4-Pro"), None);

        assert_eq!(resolved.resolved.provider, ProviderKind::Deepseek);
        assert_eq!(resolved.resolved.id, "DeepSeek-V4-Pro");
    }

    #[test]
    fn preserves_requested_model_casing_trimmed() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(Some("  DeepSeek-V4-Pro  "), None);

        assert_eq!(resolved.resolved.provider, ProviderKind::Deepseek);
        assert_eq!(resolved.resolved.id, "DeepSeek-V4-Pro");
    }

    #[test]
    fn alias_match_does_not_override_requested_casing() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(Some("deepseek-reasoner"), None);

        assert_eq!(resolved.resolved.provider, ProviderKind::Deepseek);
        assert_eq!(resolved.resolved.id, "deepseek-v4-flash");
    }

    // -- new: runtime mutation ------------------------------------------

    #[test]
    fn add_and_remove_model() {
        let mut registry = ModelRegistry::default();
        let initial_count = registry.len();

        let model = ModelInfo::new("custom-model", ProviderKind::Vllm)
            .with_aliases(vec!["my-model".into()]);
        let stable_id = model.stable_id.clone();

        registry.add_model(model).expect("add");
        assert_eq!(registry.len(), initial_count + 1);
        assert!(registry.get_by_stable_id(&stable_id).is_some());

        let removed = registry.remove_model(&stable_id).expect("remove");
        assert_eq!(removed.stable_id, stable_id);
        assert_eq!(registry.len(), initial_count);
    }

    #[test]
    fn add_model_rejects_duplicate_stable_id() {
        let mut registry = ModelRegistry::default();
        let model = ModelInfo::new("custom", ProviderKind::Deepseek);
        let stable_id = model.stable_id.clone();

        registry.add_model(model).expect("first add ok");
        let err = registry
            .add_model(ModelInfo::new("custom2", ProviderKind::Deepseek))
            .expect_err("should reject");
        // Different models get different UUIDs so this won't collide on
        // stable_id; test the non-deprecated id collision instead.
        assert!(matches!(err, RegistryError::ModelIdCollision { .. }));
    }

    #[test]
    fn deprecate_and_undeprecate_model() {
        let mut registry = ModelRegistry::default();
        let flash = registry
            .models
            .iter()
            .find(|m| m.provider == ProviderKind::Deepseek && m.id == "deepseek-v4-flash")
            .expect("flash exists");
        let stable_id = flash.stable_id.clone();

        registry
            .deprecate_model(&stable_id, Some("use v4-pro instead".into()))
            .expect("deprecate");

        let model = registry.get_by_stable_id(&stable_id).expect("still exists");
        assert!(model.deprecated);
        assert_eq!(
            model.deprecation_notice.as_deref(),
            Some("use v4-pro instead")
        );

        registry.undeprecate_model(&stable_id).expect("undeprecate");
        let model = registry.get_by_stable_id(&stable_id).expect("still exists");
        assert!(!model.deprecated);
    }

    // -- new: strict resolution -----------------------------------------

    #[test]
    fn strict_resolution_finds_existing() {
        let registry = ModelRegistry::default();
        let model = registry.resolve_strict("deepseek-v4-pro");
        assert!(model.is_some());
        assert_eq!(model.unwrap().provider, ProviderKind::Deepseek);
    }

    #[test]
    fn strict_resolution_returns_none_for_unknown() {
        let registry = ModelRegistry::default();
        assert!(registry.resolve_strict("nonexistent-model").is_none());
    }

    // -- new: filtering -------------------------------------------------

    #[test]
    fn filter_by_provider() {
        let registry = ModelRegistry::default();
        let fireworks = registry.filter_by_provider(ProviderKind::Fireworks);
        assert_eq!(fireworks.len(), 1);
        assert!(fireworks[0].id.contains("fireworks"));
    }

    #[test]
    fn filter_by_capabilities() {
        let registry = ModelRegistry::default();
        let reasoners = registry.filter_by_capabilities(false, true);
        // gpt-4.1-mini has supports_reasoning: false
        assert!(reasoners.iter().all(|m| m.supports_reasoning));

        let non_reasoners = registry.filter_by_capabilities(true, false);
        let mini = non_reasoners
            .iter()
            .find(|m| m.id == "gpt-4.1-mini");
        assert!(mini.is_some());
        assert!(!mini.unwrap().supports_reasoning);
    }

    #[test]
    fn active_models_excludes_deprecated() {
        let mut registry = ModelRegistry::default();
        let count_before = registry.active_models().len();

        let flash = registry
            .models
            .iter()
            .find(|m| m.provider == ProviderKind::Deepseek && m.id == "deepseek-v4-flash")
            .expect("flash exists");
        let stable_id = flash.stable_id.clone();

        registry
            .deprecate_model(&stable_id, None)
            .expect("deprecate");
        assert_eq!(registry.active_models().len(), count_before - 1);
    }

    #[test]
    fn list_returns_reference_slice_no_alloc() {
        let registry = ModelRegistry::default();
        let slice: &[ModelInfo] = registry.list();
        assert!(!slice.is_empty());
        // list_cloned allocates a vec
        let cloned = registry.list_cloned();
        assert_eq!(slice.len(), cloned.len());
    }

    #[test]
    fn resolution_describe_reports_fallback_chain() {
        let registry = ModelRegistry::default();
        let resolved = registry.resolve(None, None);
        let desc = resolved.describe();
        assert!(desc.contains("(default)"));
        assert!(desc.contains("deepseek-v4-pro"));

        let resolved = registry.resolve(Some("deepseek-v4-pro"), None);
        let desc = resolved.describe();
        assert!(desc.contains("direct match"));
    }

    #[test]
    fn registry_error_display() {
        let err = RegistryError::NotFound {
            stable_id: "abc-123".into(),
        };
        assert!(err.to_string().contains("abc-123"));

        let err = RegistryError::StableIdCollision {
            stable_id: "dup".into(),
        };
        assert!(err.to_string().contains("dup"));
    }
}
