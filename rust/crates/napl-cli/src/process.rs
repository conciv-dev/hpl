//! Subprocess orchestration: the coding-agent runner, the LLM client, the test
//! command runner, and the gen lock. The subprocess/lock primitives are the
//! generated `process_run` crate; this shell adds the agent-engine dispatch over
//! the lock's schema config and maps generated error strings to `CliError`.

use std::path::Path;

use crate::error::{CliError, CliResult};

pub use process_run::{run_command, GenLock, RunOutput};

/// Probe that the `claude` CLI exists, mirroring `requireClaudeAgent`/`requireClaudeCli`.
pub fn require_claude() -> CliResult<()> {
    if process_run::program_available("claude") {
        Ok(())
    } else {
        Err(CliError::new(
            "the \"claude\" CLI was not found on PATH. Install Claude Code (claude.ai/code) — napl gen runs it as a coding agent.",
        ))
    }
}

/// Run the coding agent, mirroring `createClaudeAgentRunner().run`.
pub fn run_agent(
    task: &str,
    cwd: &Path,
    model: &str,
    allowed_tools: &[String],
) -> CliResult<RunOutput> {
    process_run::run_agent(task, cwd, model, allowed_tools).map_err(CliError::new)
}

/// The resolved coding-agent engine the toolchain compiles through.
pub enum AgentEngine {
    /// The `claude` CLI in agentic mode (the default).
    Claude,
    /// The `codex` CLI in headless mode.
    Codex,
    /// A user-supplied command template with `{task}`/`{dir}` placeholders.
    Custom {
        /// The command and its argument template.
        command: Vec<String>,
    },
}

/// Resolve the agent engine from the lock's agent configuration.
#[must_use]
pub fn resolve_engine(agent: &napl_core::schemas::AgentConfig) -> AgentEngine {
    match agent.preset {
        napl_core::schemas::AgentPreset::Claude => AgentEngine::Claude,
        napl_core::schemas::AgentPreset::Codex => AgentEngine::Codex,
        napl_core::schemas::AgentPreset::Custom => AgentEngine::Custom {
            command: agent.command.clone().unwrap_or_default(),
        },
    }
}

/// Probe that the configured coding-agent engine is available on `PATH`.
pub fn require_engine(engine: &AgentEngine) -> CliResult<()> {
    match engine {
        AgentEngine::Claude => require_claude(),
        AgentEngine::Codex => require_program(
            "codex",
            "the \"codex\" CLI was not found on PATH. Install it, or set the agent preset to \"claude\" or \"custom\" in .napl/lock.json.",
        ),
        AgentEngine::Custom { command } => {
            if command.is_empty() {
                return Err(CliError::new(
                    "the \"custom\" agent preset has an empty command in .napl/lock.json.",
                ));
            }
            Ok(())
        }
    }
}

fn require_program(program: &str, message: &str) -> CliResult<()> {
    if process_run::program_available(program) {
        Ok(())
    } else {
        Err(CliError::new(message))
    }
}

/// Run the coding agent through the resolved engine. The contract is uniform:
/// the agent runs in `cwd`, receives the task text, edits files, and exits 0.
pub fn run_coding_agent(
    engine: &AgentEngine,
    task: &str,
    cwd: &Path,
    model: &str,
    allowed_tools: &[String],
) -> CliResult<RunOutput> {
    match engine {
        AgentEngine::Claude => run_agent(task, cwd, model, allowed_tools),
        AgentEngine::Codex => process_run::run_codex(task, cwd, model).map_err(CliError::new),
        AgentEngine::Custom { command } => {
            process_run::run_custom(task, cwd, command).map_err(CliError::new)
        }
    }
}

/// Complete an LLM request via the `claude` CLI, mirroring `createClaudeCliClient`.
pub fn llm_complete(model: &str, system: &str, user: &str) -> CliResult<String> {
    process_run::llm_complete(model, system, user).map_err(CliError::new)
}

/// Acquire the gen lock, mirroring `acquireGenLock` (pid + liveness injectable
/// for tests).
pub fn acquire_gen_lock_with(
    lock_path: &Path,
    pid: i32,
    is_alive: &dyn Fn(i32) -> bool,
) -> CliResult<GenLock> {
    process_run::acquire_gen_lock_with(lock_path, pid, is_alive).map_err(CliError::new)
}

/// Acquire the gen lock with the real process id and liveness check.
pub fn acquire_gen_lock(lock_path: &Path) -> CliResult<GenLock> {
    let pid = i32::try_from(std::process::id()).unwrap_or(0);
    acquire_gen_lock_with(lock_path, pid, &process_run::is_alive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_creates_lock_and_release_removes_it() {
        let dir = std::env::temp_dir().join(format!("napl-lock-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("gen.lock");
        let _ = std::fs::remove_file(&path);
        let lock = acquire_gen_lock_with(&path, 4242, &|_| false).unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "4242\n");
        lock.release();
        assert!(!path.exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn contention_when_held_by_live_other_pid() {
        let dir = std::env::temp_dir().join(format!("napl-lock-c-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("gen.lock");
        std::fs::write(&path, "9999\n").unwrap();
        let result = acquire_gen_lock_with(&path, 4242, &|_| true);
        match result {
            Err(error) => {
                assert!(error
                    .0
                    .contains("another napl gen is already running (pid 9999)"));
            }
            Ok(_) => panic!("expected a contention error"),
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "9999\n");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn stale_lock_is_stolen_when_holder_dead() {
        let dir = std::env::temp_dir().join(format!("napl-lock-s-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("gen.lock");
        std::fs::write(&path, "9999\n").unwrap();
        let lock = acquire_gen_lock_with(&path, 4242, &|_| false).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "4242\n");
        lock.release();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn same_pid_reacquires_without_contention() {
        let dir = std::env::temp_dir().join(format!("napl-lock-r-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("gen.lock");
        std::fs::write(&path, "4242\n").unwrap();
        let lock = acquire_gen_lock_with(&path, 4242, &|_| true).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "4242\n");
        lock.release();
        std::fs::remove_dir_all(&dir).ok();
    }
}
