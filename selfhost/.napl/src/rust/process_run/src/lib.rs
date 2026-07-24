//! Subprocess orchestration and the gen lock.
//!
//! This module is the CLI's subprocess seam: it spawns the coding agent and the
//! LLM client, runs the target's test command, probes that a program exists on
//! `PATH`, and holds the single-writer gen lock file. Every function shells out
//! to a real binary or touches the real filesystem, so it is an I/O shell, not a
//! pure module. It depends on nothing but the Rust standard library.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// The uniform result of running the coding agent or a command.
pub struct RunOutput {
    /// The captured process text.
    pub output: String,
    /// The process exit code.
    pub code: i32,
}

/// Merge the two captured streams the way the agent runners do: stdout alone
/// when the trimmed stderr is empty, otherwise stdout, a newline, then stderr.
fn merge_streams(stdout: &str, stderr: &str) -> String {
    if stderr.trim().is_empty() {
        stdout.to_string()
    } else {
        format!("{stdout}\n{stderr}")
    }
}

/// Run `command` with `args` in directory `cwd`, with stdin connected to null
/// and stdout+stderr captured. The output is stdout immediately followed by
/// stderr, with no separator between them.
pub fn run_command(command: &str, args: &[String], cwd: &Path) -> RunOutput {
    let spawned = Command::new(command)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    match spawned {
        Ok(out) => RunOutput {
            output: format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            code: out.status.code().unwrap_or(1),
        },
        Err(error) => RunOutput {
            output: format!("\n{error}"),
            code: 1,
        },
    }
}

/// Run `program --version` with its output discarded and report whether it
/// launched and exited successfully.
pub fn program_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Spawn `command` with the three streams piped, write `task` to its stdin, wait
/// for it to finish, and merge the captured streams.
fn run_piped(mut command: Command, task: &str) -> std::io::Result<RunOutput> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(task.as_bytes())?;
    }
    let out = child.wait_with_output()?;
    Ok(RunOutput {
        output: merge_streams(
            &String::from_utf8_lossy(&out.stdout),
            &String::from_utf8_lossy(&out.stderr),
        ),
        code: out.status.code().unwrap_or(1),
    })
}

/// Run the `claude` CLI as a coding agent in `cwd`, feeding `task` on stdin.
pub fn run_agent(
    task: &str,
    cwd: &Path,
    model: &str,
    allowed_tools: &[String],
) -> Result<RunOutput, String> {
    let mut command = Command::new("claude");
    command
        .args([
            "-p",
            "--output-format",
            "text",
            "--model",
            model,
            "--no-session-persistence",
            "--permission-mode",
            "acceptEdits",
            "--allowedTools",
        ])
        .args(allowed_tools)
        .current_dir(cwd);
    run_piped(command, task).map_err(|e| format!("failed to spawn the \"claude\" agent: {e}"))
}

/// Run the `codex` CLI in headless mode in `cwd`, feeding `task` on stdin.
pub fn run_codex(task: &str, cwd: &Path, model: &str) -> Result<RunOutput, String> {
    let mut command = Command::new("codex");
    command
        .args(["exec", "--full-auto", "--model", model])
        .current_dir(cwd);
    run_piped(command, task).map_err(|e| format!("failed to spawn the \"codex\" agent: {e}"))
}

/// Run a user-supplied agent command template. `{task}` is replaced with the
/// path of a temporary file holding `task`, and `{dir}` with `cwd`.
pub fn run_custom(task: &str, cwd: &Path, command: &[String]) -> Result<RunOutput, String> {
    let Some((program_template, arg_templates)) = command.split_first() else {
        return Err("the custom agent command is empty".to_string());
    };

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let task_file = std::env::temp_dir().join(format!(
        "napl-agent-task-{}-{}.txt",
        std::process::id(),
        nanos
    ));
    fs::write(&task_file, task)
        .map_err(|e| format!("could not write the custom agent task file: {e}"))?;

    let task_path = task_file.to_string_lossy().to_string();
    let dir_path = cwd.to_string_lossy().to_string();
    let substitute = |arg: &String| arg.replace("{task}", &task_path).replace("{dir}", &dir_path);
    let program = substitute(program_template);
    let args: Vec<String> = arg_templates.iter().map(substitute).collect();

    let spawned = Command::new(&program)
        .args(&args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let _ = fs::remove_file(&task_file);

    match spawned {
        Ok(out) => Ok(RunOutput {
            output: merge_streams(
                &String::from_utf8_lossy(&out.stdout),
                &String::from_utf8_lossy(&out.stderr),
            ),
            code: out.status.code().unwrap_or(1),
        }),
        Err(error) => Err(format!(
            "failed to spawn the custom agent \"{program}\": {error}"
        )),
    }
}

/// Run the `claude` CLI as a one-shot completion and return its raw stdout.
pub fn llm_complete(model: &str, system: &str, user: &str) -> Result<String, String> {
    let mut command = Command::new("claude");
    command.args([
        "-p",
        "--output-format",
        "text",
        "--model",
        model,
        "--no-session-persistence",
    ]);
    if !system.trim().is_empty() {
        command.args(["--system-prompt", system]);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn the \"claude\" CLI: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(user.as_bytes())
            .map_err(|e| format!("failed to spawn the \"claude\" CLI: {e}"))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("failed to spawn the \"claude\" CLI: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let code = out.status.code().unwrap_or(1);
    if code != 0 {
        let detail = stderr.trim();
        let detail = if detail.is_empty() {
            "the claude CLI produced no stderr output"
        } else {
            detail
        };
        return Err(format!(
            "the \"claude\" CLI exited with code {code}: {detail}"
        ));
    }
    if stdout.trim().is_empty() {
        return Err("the \"claude\" CLI returned an empty response".to_string());
    }
    Ok(stdout)
}

/// A held gen lock: the CLI's single-writer guard.
pub struct GenLock {
    path: PathBuf,
}

impl GenLock {
    /// Remove the lock file if it still exists, ignoring any error.
    pub fn release(&self) {
        if self.path.exists() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Report whether a process with the given id is alive, via `kill -0 {pid}`.
pub fn is_alive(pid: i32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Read the currently held pid: the file's trimmed contents parsed as an `i32`,
/// or `None` when the file is unreadable or unparseable. An existing but empty
/// file means a competing acquirer created it and has not written its pid yet, so
/// give that write a bounded chance to land before calling the lock unheld.
fn read_holder(lock_path: &Path) -> Option<i32> {
    for _ in 0..100 {
        let text = fs::read_to_string(lock_path).ok()?;
        if !text.trim().is_empty() {
            return text.trim().parse::<i32>().ok();
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    None
}

/// Acquire the gen lock atomically, with the process id and the liveness check
/// injected so tests are deterministic.
pub fn acquire_gen_lock_with(
    lock_path: &Path,
    pid: i32,
    is_alive: &dyn Fn(i32) -> bool,
) -> Result<GenLock, String> {
    if let Some(parent) = lock_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }

    let steal = |lock: GenLock| -> Result<GenLock, String> {
        fs::write(&lock.path, format!("{pid}\n")).map_err(|e| e.to_string())?;
        Ok(lock)
    };

    let lock = GenLock {
        path: lock_path.to_path_buf(),
    };
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)
    {
        Ok(mut file) => {
            file.write_all(format!("{pid}\n").as_bytes())
                .map_err(|e| e.to_string())?;
            Ok(lock)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let holder = read_holder(lock_path);
            match holder {
                Some(other) if other != pid && is_alive(other) => Err(format!(
                    "another napl gen is already running (pid {other}); the lock {} is held. Wait for it to finish or remove the lock if the process is gone.",
                    lock_path.to_string_lossy()
                )),
                _ => steal(lock),
            }
        }
        Err(error) => Err(format!(
            "could not acquire gen lock at {}: {error}",
            lock_path.to_string_lossy()
        )),
    }
}

/// Acquire the gen lock with the real process id and the real liveness probe.
pub fn acquire_gen_lock(lock_path: &Path) -> Result<GenLock, String> {
    let pid = i32::try_from(std::process::id()).unwrap_or(0);
    acquire_gen_lock_with(lock_path, pid, &is_alive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "process-run-test-{label}-{}-{nanos}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| part.to_string()).collect()
    }

    #[test]
    fn run_command_concatenates_stdout_then_stderr() {
        let dir = temp_dir("run-command");
        let run = run_command(
            "sh",
            &argv(&["-c", "printf OUT; printf ERR >&2; exit 3"]),
            &dir,
        );

        assert_eq!(run.output, "OUTERR");
        assert_eq!(run.code, 3);
    }

    #[test]
    fn run_command_reports_a_spawn_failure_as_a_leading_newline() {
        let dir = temp_dir("run-command-missing");
        let run = run_command("napl-no-such-program", &[], &dir);

        assert!(run.output.starts_with('\n'), "unexpected: {:?}", run.output);
        assert_eq!(run.code, 1);
    }

    #[test]
    fn program_available_reflects_whether_the_program_launches() {
        assert!(program_available("cargo"));
        assert!(!program_available("napl-no-such-program"));
    }

    #[test]
    fn run_custom_rejects_an_empty_command() {
        let dir = temp_dir("custom-empty");

        match run_custom("task", &dir, &[]) {
            Ok(_) => panic!("an empty command must be refused"),
            Err(error) => assert_eq!(error, "the custom agent command is empty"),
        }
    }

    #[test]
    fn run_custom_substitutes_the_task_file_and_the_directory() {
        let dir = temp_dir("custom-subst");

        let run = run_custom("the task body", &dir, &argv(&["cat", "{task}"])).expect("ran");
        assert_eq!(run.output, "the task body");
        assert_eq!(run.code, 0);

        let run = run_custom("ignored", &dir, &argv(&["printf", "%s", "{dir}"])).expect("ran");
        assert_eq!(run.output, dir.to_string_lossy());
    }

    #[test]
    fn run_custom_reports_a_spawn_failure_with_the_program_name() {
        let dir = temp_dir("custom-missing");

        let error = match run_custom("task", &dir, &argv(&["napl-no-such-program"])) {
            Ok(_) => panic!("a missing program must fail to spawn"),
            Err(error) => error,
        };
        assert!(
            error.starts_with("failed to spawn the custom agent \"napl-no-such-program\": "),
            "unexpected message: {error}"
        );
    }

    #[test]
    fn acquires_a_fresh_lock_and_releases_it() {
        let dir = temp_dir("fresh");
        let path = dir.join("nested").join("gen.lock");

        let lock = acquire_gen_lock_with(&path, 4242, &|_| false).expect("acquired");
        assert_eq!(fs::read_to_string(&path).unwrap(), "4242\n");

        lock.release();
        assert!(!path.exists());
    }

    #[test]
    fn refuses_a_lock_held_by_a_live_other_process() {
        let dir = temp_dir("live");
        let path = dir.join("gen.lock");
        fs::write(&path, "9999\n").unwrap();

        let error = match acquire_gen_lock_with(&path, 4242, &|_| true) {
            Ok(_) => panic!("expected the contended lock to be refused"),
            Err(error) => error,
        };
        assert!(
            error.contains("another napl gen is already running (pid 9999)"),
            "unexpected message: {error}"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "9999\n");
    }

    #[test]
    fn steals_a_lock_whose_holder_is_dead() {
        let dir = temp_dir("stale");
        let path = dir.join("gen.lock");
        fs::write(&path, "9999\n").unwrap();

        acquire_gen_lock_with(&path, 4242, &|_| false).expect("stolen");
        assert_eq!(fs::read_to_string(&path).unwrap(), "4242\n");
    }

    #[test]
    fn reacquires_a_lock_held_by_this_same_pid() {
        let dir = temp_dir("same-pid");
        let path = dir.join("gen.lock");
        fs::write(&path, "4242\n").unwrap();

        acquire_gen_lock_with(&path, 4242, &|_| true).expect("reacquired");
        assert_eq!(fs::read_to_string(&path).unwrap(), "4242\n");
    }

    #[test]
    fn only_one_racing_thread_acquires_a_fresh_lock() {
        let dir = temp_dir("race");
        let path = dir.join("gen.lock");

        let outcomes: Vec<Result<GenLock, String>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|index| {
                    let path = path.clone();
                    scope.spawn(move || acquire_gen_lock_with(&path, 5000 + index, &|_| true))
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("thread"))
                .collect()
        });

        let acquired = outcomes.iter().filter(|outcome| outcome.is_ok()).count();
        assert_eq!(acquired, 1, "exactly one thread must hold the lock");
        for outcome in &outcomes {
            if let Err(error) = outcome {
                assert!(
                    error.contains("another napl gen is already running (pid 5"),
                    "unexpected message: {error}"
                );
            }
        }
        let contents = fs::read_to_string(&path).unwrap();
        assert!(
            contents.trim().parse::<i32>().is_ok(),
            "lock file never holds a partial pid: {contents:?}"
        );
    }

    #[test]
    fn a_broken_child_stdin_pipe_is_an_error() {
        let mut command = Command::new("sh");
        command.args(["-c", "exit 0"]);
        let task = "x".repeat(4 * 1024 * 1024);

        assert!(
            run_piped(command, &task).is_err(),
            "a broken stdin pipe must surface as an error"
        );
    }
}
