//! Conversation settings for per-conversation configuration.
//!
//! This module defines the settings that control conversation behavior,
//! including memory mode, delegation mode, and worker context settings.

use serde::{Deserialize, Serialize};

/// Memory mode controls how memory is used in a conversation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemoryMode {
    /// Full memory context with auto-persistence (default).
    /// Knowledge synthesis + working memory + channel activity map.
    /// Memory persistence branches fire.
    #[default]
    Full,
    /// All memory context injected, but no auto-persistence and no memory tools.
    /// The agent can see memories but doesn't write new ones.
    Ambient,
    /// No memory context injected, no memory tools, no persistence.
    /// The conversation is stateless relative to the agent's memory.
    Off,
}

impl MemoryMode {
    /// Returns true if memory persistence should be enabled.
    pub fn persistence_enabled(&self) -> bool {
        matches!(self, MemoryMode::Full)
    }

    /// Returns true if memory tools should be available.
    pub fn memory_tools_enabled(&self) -> bool {
        matches!(self, MemoryMode::Full)
    }
}

/// Delegation mode controls how the conversation handles tools.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DelegationMode {
    /// Standard channel behavior: delegates via branch/worker.
    /// Channel has reply, branch, spawn_worker, route, cancel, skip, react.
    #[default]
    Standard,
    /// Direct tool access: channel gets full tool set including memory,
    /// shell, file operations, browser, web search, plus delegation tools.
    Direct,
}

impl DelegationMode {
    /// Returns true if direct tool access is enabled.
    pub fn is_direct(&self) -> bool {
        matches!(self, DelegationMode::Direct)
    }
}

/// How much conversation history a worker receives.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkerHistoryMode {
    /// No conversation history (current default).
    /// Worker sees only the task description.
    #[default]
    None,
    /// LLM-generated summary of recent conversation context.
    Summary,
    /// Last N messages from the parent conversation.
    Recent(u32),
    /// Full conversation history clone (branch-style).
    Full,
}

/// How much memory context a worker receives.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkerMemoryMode {
    /// No memory context (current default).
    /// Worker is a pure executor with no memory access.
    #[default]
    None,
    /// Knowledge synthesis + working memory injected into system prompt (read-only).
    /// Worker has ambient awareness but can't search or write.
    Ambient,
    /// Ambient context + memory_recall tool.
    /// Worker can search but not write memories.
    Tools,
    /// Ambient context + full memory tools (recall, save, delete).
    /// Worker operates at branch-level memory access.
    Full,
}

impl WorkerMemoryMode {
    /// Returns true if the worker should receive ambient memory context.
    pub fn ambient_enabled(&self) -> bool {
        matches!(
            self,
            WorkerMemoryMode::Ambient | WorkerMemoryMode::Tools | WorkerMemoryMode::Full
        )
    }

    /// Returns true if the worker should have the memory_recall tool.
    pub fn recall_enabled(&self) -> bool {
        matches!(self, WorkerMemoryMode::Tools | WorkerMemoryMode::Full)
    }

    /// Returns true if the worker should have full memory tools (save, delete).
    pub fn full_tools_enabled(&self) -> bool {
        matches!(self, WorkerMemoryMode::Full)
    }
}

/// Worker context settings control what context workers receive when spawned.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WorkerContextMode {
    /// What conversation context the worker sees.
    pub history: WorkerHistoryMode,
    /// What memory context the worker gets.
    pub memory: WorkerMemoryMode,
}

/// Per-conversation settings that control behavior.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ConversationSettings {
    /// Optional model override for this conversation's channel process.
    /// When set, overrides routing.channel for this conversation.
    /// Branches and workers spawned from this conversation inherit the override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// How memory is used in this conversation.
    #[serde(default)]
    pub memory: MemoryMode,

    /// How tools work in this conversation.
    #[serde(default)]
    pub delegation: DelegationMode,

    /// What context workers spawned from this conversation receive.
    #[serde(default)]
    pub worker_context: WorkerContextMode,
}

/// Resolved conversation settings after applying defaults.
/// This is what gets used at runtime.
#[derive(Debug, Clone)]
pub struct ResolvedConversationSettings {
    /// The resolved model override (None means use routing config).
    pub model: Option<String>,
    /// The resolved memory mode.
    pub memory: MemoryMode,
    /// The resolved delegation mode.
    pub delegation: DelegationMode,
    /// The resolved worker context settings.
    pub worker_context: WorkerContextMode,
}

impl ResolvedConversationSettings {
    /// Create default resolved settings.
    pub fn default_with_agent(_agent_id: &str) -> Self {
        Self {
            model: None,
            memory: MemoryMode::Full,
            delegation: DelegationMode::Standard,
            worker_context: WorkerContextMode::default(),
        }
    }

    /// Resolve settings from conversation-level, channel-level, and agent defaults.
    /// Resolution order: conversation > channel > agent default > system default.
    pub fn resolve(
        conversation: Option<&ConversationSettings>,
        channel: Option<&ConversationSettings>,
        agent_default: Option<&ConversationSettings>,
    ) -> Self {
        // Start with system defaults
        let mut resolved = Self::default();

        // Apply agent defaults if present
        if let Some(default) = agent_default {
            resolved.model = default.model.clone();
            resolved.memory = default.memory;
            resolved.delegation = default.delegation;
            resolved.worker_context = default.worker_context.clone();
        }

        // Apply channel overrides if present
        if let Some(channel_settings) = channel {
            if channel_settings.model.is_some() {
                resolved.model = channel_settings.model.clone();
            }
            resolved.memory = channel_settings.memory;
            resolved.delegation = channel_settings.delegation;
            resolved.worker_context = channel_settings.worker_context.clone();
        }

        // Apply conversation overrides if present (highest priority)
        if let Some(conv_settings) = conversation {
            if conv_settings.model.is_some() {
                resolved.model = conv_settings.model.clone();
            }
            resolved.memory = conv_settings.memory;
            resolved.delegation = conv_settings.delegation;
            resolved.worker_context = conv_settings.worker_context.clone();
        }

        resolved
    }
}

impl Default for ResolvedConversationSettings {
    fn default() -> Self {
        Self {
            model: None,
            memory: MemoryMode::Full,
            delegation: DelegationMode::Standard,
            worker_context: WorkerContextMode::default(),
        }
    }
}

/// Response payload for conversation defaults endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ConversationDefaultsResponse {
    /// Current default model name (from agent config).
    pub model: String,
    /// Current default memory mode.
    pub memory: MemoryMode,
    /// Current default delegation mode.
    pub delegation: DelegationMode,
    /// Current default worker context settings.
    pub worker_context: WorkerContextMode,
    /// All available models.
    pub available_models: Vec<ModelOption>,
    /// Available memory modes.
    pub memory_modes: Vec<String>,
    /// Available delegation modes.
    pub delegation_modes: Vec<String>,
    /// Available worker history modes.
    pub worker_history_modes: Vec<String>,
    /// Available worker memory modes.
    pub worker_memory_modes: Vec<String>,
}

/// Model option for the defaults response.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ModelOption {
    /// Model ID (e.g. "anthropic/claude-sonnet-4").
    pub id: String,
    /// Display name (e.g. "Claude Sonnet 4").
    pub name: String,
    /// Provider name (e.g. "anthropic").
    pub provider: String,
    /// Context window size.
    pub context_window: usize,
    /// Whether the model supports tools.
    pub supports_tools: bool,
    /// Whether the model supports thinking/claude-style extended thinking.
    pub supports_thinking: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_mode_persistence() {
        assert!(MemoryMode::Full.persistence_enabled());
        assert!(!MemoryMode::Ambient.persistence_enabled());
        assert!(!MemoryMode::Off.persistence_enabled());
    }

    #[test]
    fn test_worker_memory_modes() {
        assert!(!WorkerMemoryMode::None.ambient_enabled());
        assert!(WorkerMemoryMode::Ambient.ambient_enabled());
        assert!(WorkerMemoryMode::Tools.ambient_enabled());
        assert!(WorkerMemoryMode::Full.ambient_enabled());

        assert!(!WorkerMemoryMode::None.recall_enabled());
        assert!(!WorkerMemoryMode::Ambient.recall_enabled());
        assert!(WorkerMemoryMode::Tools.recall_enabled());
        assert!(WorkerMemoryMode::Full.recall_enabled());

        assert!(!WorkerMemoryMode::None.full_tools_enabled());
        assert!(!WorkerMemoryMode::Ambient.full_tools_enabled());
        assert!(!WorkerMemoryMode::Tools.full_tools_enabled());
        assert!(WorkerMemoryMode::Full.full_tools_enabled());
    }

    #[test]
    fn test_settings_resolution_order() {
        // Test that conversation settings override channel settings
        let agent_default = ConversationSettings {
            model: Some("agent-model".to_string()),
            memory: MemoryMode::Full,
            delegation: DelegationMode::Standard,
            worker_context: WorkerContextMode::default(),
        };

        let channel_settings = ConversationSettings {
            model: Some("channel-model".to_string()),
            memory: MemoryMode::Ambient,
            delegation: DelegationMode::Standard,
            worker_context: WorkerContextMode::default(),
        };

        let conversation_settings = ConversationSettings {
            model: Some("conversation-model".to_string()),
            memory: MemoryMode::Off,
            delegation: DelegationMode::Direct,
            worker_context: WorkerContextMode {
                history: WorkerHistoryMode::Recent(20),
                memory: WorkerMemoryMode::Tools,
            },
        };

        let resolved = ResolvedConversationSettings::resolve(
            Some(&conversation_settings),
            Some(&channel_settings),
            Some(&agent_default),
        );

        // Conversation settings should win
        assert_eq!(resolved.model, Some("conversation-model".to_string()));
        assert_eq!(resolved.memory, MemoryMode::Off);
        assert_eq!(resolved.delegation, DelegationMode::Direct);
        assert_eq!(
            resolved.worker_context.history,
            WorkerHistoryMode::Recent(20)
        );
        assert_eq!(resolved.worker_context.memory, WorkerMemoryMode::Tools);
    }

    #[test]
    fn test_settings_resolution_defaults() {
        // Test with no settings provided - should use system defaults
        let resolved = ResolvedConversationSettings::resolve(None, None, None);

        assert_eq!(resolved.model, None);
        assert_eq!(resolved.memory, MemoryMode::Full);
        assert_eq!(resolved.delegation, DelegationMode::Standard);
        assert_eq!(resolved.worker_context.history, WorkerHistoryMode::None);
        assert_eq!(resolved.worker_context.memory, WorkerMemoryMode::None);
    }
}
