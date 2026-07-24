//! Lock schema: model / backend / prompt-alias configuration.

use serde::{Deserialize, Serialize};

use crate::extensions::default_prompt_aliases;

use super::SchemaError;

/// The default model.
pub const DEFAULT_MODEL: &str = "claude-sonnet-5";

const ZWJ: char = '\u{200D}';

/// The code-generation backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Backend {
    ClaudeCli,
    AnthropicApi,
}

/// The default backend.
pub const DEFAULT_BACKEND: Backend = Backend::ClaudeCli;

fn default_backend() -> Backend {
    DEFAULT_BACKEND
}

/// The coding-agent engine preset the toolchain compiles through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentPreset {
    Claude,
    Codex,
    Custom,
}

/// The default agent preset.
pub const DEFAULT_AGENT_PRESET: AgentPreset = AgentPreset::Claude;

/// The agent-adapter configuration: which engine runs the coding agent, and,
/// for the `custom` preset, the command template invoked (with `{task}` and
/// `{dir}` placeholders).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentConfig {
    pub preset: AgentPreset,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub command: Option<Vec<String>>,
}

/// The `claude`-preset agent configuration written by `init`.
#[must_use]
pub fn default_agent_config() -> AgentConfig {
    AgentConfig {
        preset: DEFAULT_AGENT_PRESET,
        command: None,
    }
}

/// The lock document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HlLock {
    pub model: String,
    #[serde(default = "default_backend")]
    pub backend: Backend,
    #[serde(
        rename = "promptAliases",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub prompt_aliases: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub agent: Option<AgentConfig>,
}

/// The effective agent configuration: the lock's override or the `claude` default.
#[must_use]
pub fn resolve_agent_config(lock: &HlLock) -> AgentConfig {
    lock.agent.clone().unwrap_or_else(default_agent_config)
}

fn validate_agent(agent: &AgentConfig) -> Result<(), SchemaError> {
    match agent.preset {
        AgentPreset::Custom => match &agent.command {
            None => Err(SchemaError::Validation(
                "the \"custom\" agent preset requires a non-empty \"command\" array".to_string(),
            )),
            Some(command) if command.is_empty() => Err(SchemaError::Validation(
                "the \"custom\" agent preset requires a non-empty \"command\" array".to_string(),
            )),
            Some(_) => Ok(()),
        },
        AgentPreset::Claude | AgentPreset::Codex => {
            if agent.command.is_some() {
                return Err(SchemaError::Validation(format!(
                    "the \"{}\" agent preset does not accept a \"command\"; only the \"custom\" preset does",
                    match agent.preset {
                        AgentPreset::Codex => "codex",
                        _ => "claude",
                    }
                )));
            }
            Ok(())
        }
    }
}

/// Validate a single prompt alias, mirroring `promptAliasSchema`: it must start
/// with `.`, have 1-2 code points after the `.`, and contain no ZWJ.
fn validate_alias(value: &str) -> Result<(), SchemaError> {
    if !value.starts_with('.') {
        return Err(SchemaError::Validation(
            "a prompt alias must start with \".\"".to_string(),
        ));
    }
    let after_dot_codepoints = value.chars().skip(1).count();
    if !(1..=2).contains(&after_dot_codepoints) {
        return Err(SchemaError::Validation(
            "a prompt alias must have 1-2 code points after the \".\"".to_string(),
        ));
    }
    if value.contains(ZWJ) {
        return Err(SchemaError::Validation(
            "a prompt alias must not contain a ZWJ (zero-width joiner) sequence".to_string(),
        ));
    }
    Ok(())
}

/// Parse and validate a lock JSON string, mirroring `parseLock`.
pub fn parse_lock(raw: &str) -> Result<HlLock, SchemaError> {
    let lock: HlLock =
        serde_json::from_str(raw).map_err(|e| SchemaError::Deserialize(e.to_string()))?;
    if lock.model.is_empty() {
        return Err(SchemaError::Validation(
            "model must not be empty".to_string(),
        ));
    }
    if let Some(aliases) = &lock.prompt_aliases {
        for alias in aliases {
            validate_alias(alias)?;
        }
    }
    if let Some(agent) = &lock.agent {
        validate_agent(agent)?;
    }
    Ok(lock)
}

/// The effective prompt aliases: the lock's override or the curated default.
#[must_use]
pub fn resolve_prompt_aliases(lock: &HlLock) -> Vec<String> {
    lock.prompt_aliases
        .clone()
        .unwrap_or_else(default_prompt_aliases)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_backend_to_claude_cli() {
        let lock = parse_lock(r#"{"model":"claude-sonnet-5"}"#).unwrap();
        assert_eq!(lock.backend, Backend::ClaudeCli);
        assert_eq!(DEFAULT_BACKEND, Backend::ClaudeCli);
    }

    #[test]
    fn keeps_explicit_anthropic_api() {
        let lock = parse_lock(r#"{"model":"claude-sonnet-5","backend":"anthropic-api"}"#).unwrap();
        assert_eq!(lock.backend, Backend::AnthropicApi);
    }

    #[test]
    fn keeps_explicit_claude_cli() {
        let lock = parse_lock(r#"{"model":"claude-opus-5","backend":"claude-cli"}"#).unwrap();
        assert_eq!(lock.backend, Backend::ClaudeCli);
        assert_eq!(lock.model, "claude-opus-5");
    }

    #[test]
    fn rejects_unknown_backend() {
        assert!(parse_lock(r#"{"model":"x","backend":"openai"}"#).is_err());
    }

    #[test]
    fn rejects_corrupt_json() {
        assert!(parse_lock("{not json").is_err());
    }

    #[test]
    fn default_model_constant() {
        assert_eq!(DEFAULT_MODEL, "claude-sonnet-5");
    }

    #[test]
    fn prompt_aliases_default_to_curated_when_absent() {
        let lock = parse_lock(r#"{"model":"m"}"#).unwrap();
        assert!(lock.prompt_aliases.is_none());
        assert_eq!(resolve_prompt_aliases(&lock), default_prompt_aliases());
    }

    #[test]
    fn accepts_valid_override_verbatim() {
        let lock = parse_lock(r#"{"model":"m","promptAliases":[".🧑",".🤠"]}"#).unwrap();
        assert_eq!(
            lock.prompt_aliases,
            Some(vec![".\u{1F9D1}".to_string(), ".\u{1F920}".to_string()])
        );
        assert_eq!(
            resolve_prompt_aliases(&lock),
            vec![".\u{1F9D1}".to_string(), ".\u{1F920}".to_string()]
        );
    }

    #[test]
    fn rejects_alias_without_dot() {
        assert!(parse_lock(r#"{"model":"m","promptAliases":["🧑"]}"#).is_err());
    }

    #[test]
    fn rejects_alias_with_more_than_two_codepoints() {
        assert!(parse_lock(r#"{"model":"m","promptAliases":[".abc"]}"#).is_err());
    }

    #[test]
    fn rejects_zwj_sequence() {
        // ".👨‍💻" as JSON escapes: man + ZWJ + laptop.
        assert!(parse_lock(r#"{"model":"m","promptAliases":[".👨‍💻"]}"#).is_err());
    }

    #[test]
    fn agent_defaults_to_claude_when_absent() {
        let lock = parse_lock(r#"{"model":"m"}"#).unwrap();
        assert!(lock.agent.is_none());
        assert_eq!(resolve_agent_config(&lock), default_agent_config());
        assert_eq!(resolve_agent_config(&lock).preset, AgentPreset::Claude);
    }

    #[test]
    fn accepts_codex_preset() {
        let lock = parse_lock(r#"{"model":"m","agent":{"preset":"codex"}}"#).unwrap();
        assert_eq!(lock.agent.unwrap().preset, AgentPreset::Codex);
    }

    #[test]
    fn accepts_custom_preset_with_command() {
        let lock = parse_lock(
            r#"{"model":"m","agent":{"preset":"custom","command":["mycli","--task-file","{task}"]}}"#,
        )
        .unwrap();
        let agent = lock.agent.unwrap();
        assert_eq!(agent.preset, AgentPreset::Custom);
        assert_eq!(
            agent.command,
            Some(vec![
                "mycli".to_string(),
                "--task-file".to_string(),
                "{task}".to_string()
            ])
        );
    }

    #[test]
    fn rejects_unknown_preset() {
        assert!(parse_lock(r#"{"model":"m","agent":{"preset":"gpt"}}"#).is_err());
    }

    #[test]
    fn rejects_custom_without_command() {
        assert!(parse_lock(r#"{"model":"m","agent":{"preset":"custom"}}"#).is_err());
    }

    #[test]
    fn rejects_custom_with_empty_command() {
        assert!(parse_lock(r#"{"model":"m","agent":{"preset":"custom","command":[]}}"#).is_err());
    }

    #[test]
    fn rejects_claude_preset_with_command() {
        assert!(parse_lock(r#"{"model":"m","agent":{"preset":"claude","command":["x"]}}"#).is_err());
    }

    #[test]
    fn rejects_unknown_agent_field() {
        assert!(parse_lock(r#"{"model":"m","agent":{"preset":"claude","extra":1}}"#).is_err());
    }
}
