use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Shell metacharacters that could enable command injection
const DANGEROUS_CHARS: &[char] = &[
    ';', '&', '|', '`', '$', '\n', '\r', '\'', '"', '(', ')', '{', '}', '<', '>', '*', '?', '[',
    ']', '!', '#',
];

/// Validate that a path doesn't contain shell metacharacters that could enable command injection
fn validate_path(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();

    for c in DANGEROUS_CHARS {
        if path_str.contains(*c) {
            return Err(anyhow::anyhow!(
                "Invalid path: contains shell metacharacter '{}'",
                c
            ));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunnerStatus {
    Active,
    Inactive,
    Failed,
    NotFound,
}

impl RunnerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunnerStatus::Active => "active",
            RunnerStatus::Inactive => "inactive",
            RunnerStatus::Failed => "failed",
            RunnerStatus::NotFound => "not-found",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            RunnerStatus::Active => "●",
            RunnerStatus::Inactive => "○",
            RunnerStatus::Failed => "✗",
            RunnerStatus::NotFound => "?",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Runner {
    pub name: String,
    pub number: u32,
    pub repo: String,
    pub status: RunnerStatus,
    pub service_name: String,
    pub path: PathBuf,
}

impl Runner {
    pub fn display_name(&self) -> String {
        format!("{}-runner-{}", self.repo, self.number)
    }
}

/// Discover all runners from the action-runners directory
pub fn discover_runners() -> Result<Vec<Runner>> {
    let mut runners = Vec::new();

    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    let runners_dir = home.join("action-runners");

    if !runners_dir.exists() {
        return Ok(runners);
    }

    // Get current username for service naming
    let username = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());

    // Iterate through repository directories
    for repo_entry in std::fs::read_dir(&runners_dir)? {
        let repo_entry = repo_entry?;
        let repo_path = repo_entry.path();

        if !repo_path.is_dir() {
            continue;
        }

        let repo_name = repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if repo_name.is_empty() {
            continue;
        }

        // Iterate through runner directories within the repo
        for runner_entry in std::fs::read_dir(&repo_path)? {
            let runner_entry = runner_entry?;
            let runner_path = runner_entry.path();

            if !runner_path.is_dir() {
                continue;
            }

            // Check if this looks like a runner directory (has run.sh)
            if !runner_path.join("run.sh").exists() {
                continue;
            }

            let runner_num_str = runner_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("0");

            let runner_num: u32 = runner_num_str.parse().unwrap_or(0);

            let service_name = format!(
                "actions.runner.{}.{}-runner-{}",
                username, &repo_name, runner_num
            );

            let status = get_service_status(&service_name, &runner_path);

            runners.push(Runner {
                name: format!("runner-{}", runner_num),
                number: runner_num,
                repo: repo_name.to_string(),
                status,
                service_name,
                path: runner_path,
            });
        }
    }

    // Sort runners by repo then by number
    runners.sort_by(|a, b| a.repo.cmp(&b.repo).then_with(|| a.number.cmp(&b.number)));

    Ok(runners)
}

/// Get the status of a runner service (cross-platform)
fn get_service_status(service_name: &str, runner_path: &std::path::Path) -> RunnerStatus {
    if cfg!(target_os = "macos") {
        get_macos_service_status(service_name, runner_path)
    } else {
        get_linux_service_status(service_name, runner_path)
    }
}

/// Get service status on Linux using systemctl with process-based fallback
fn get_linux_service_status(service_name: &str, runner_path: &std::path::Path) -> RunnerStatus {
    // Try to get status from systemd service unit
    if let Some(status) = check_systemd_service_status(service_name) {
        return status;
    }

    // Fallback: check if runner process is running
    check_runner_status_fallback(runner_path)
}

/// Check systemd service status, returns None if service doesn't exist
fn check_systemd_service_status(service_name: &str) -> Option<RunnerStatus> {
    let unit_exists = Command::new("systemctl")
        .args(["cat", service_name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !unit_exists {
        return None;
    }

    let output = Command::new("systemctl")
        .args(["is-active", service_name])
        .output()
        .ok()?;

    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match status.as_str() {
        "active" => Some(RunnerStatus::Active),
        "inactive" => Some(RunnerStatus::Inactive),
        "failed" => Some(RunnerStatus::Failed),
        _ => None, // Fall through to process check
    }
}

/// Check runner status using process and configuration file checks
fn check_runner_status_fallback(runner_path: &std::path::Path) -> RunnerStatus {
    if is_runner_process_running(runner_path) {
        return RunnerStatus::Active;
    }

    if runner_path.join(".runner").exists() {
        return RunnerStatus::Inactive;
    }

    RunnerStatus::NotFound
}

/// Get service status on macOS using launchctl or process check
fn get_macos_service_status(service_name: &str, runner_path: &std::path::Path) -> RunnerStatus {
    // Try exact service name match
    if let Some(status) = check_launchctl_exact_service(service_name) {
        return status;
    }

    // Try partial match for service name variations
    if let Some(status) = check_launchctl_partial_match(runner_path) {
        return status;
    }

    // Fallback: check runner process and configuration
    check_runner_status_fallback(runner_path)
}

/// Check launchctl for exact service name match
fn check_launchctl_exact_service(service_name: &str) -> Option<RunnerStatus> {
    let output = Command::new("launchctl")
        .args(["list", service_name])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?;
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    let pid = parts.first()?;

    if *pid != "-" && pid.parse::<u32>().is_ok() {
        Some(RunnerStatus::Active)
    } else {
        Some(RunnerStatus::Inactive)
    }
}

/// Check launchctl list for partial service name match
fn check_launchctl_partial_match(runner_path: &std::path::Path) -> Option<RunnerStatus> {
    let output = Command::new("launchctl").arg("list").output().ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parent_dir = runner_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())?;

    for line in stdout.lines() {
        if line.contains("actions.runner") && line.contains(parent_dir) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(pid) = parts.first() {
                if *pid != "-" && pid.parse::<u32>().is_ok() {
                    return Some(RunnerStatus::Active);
                }
            }
        }
    }

    None
}

/// Check if a runner process is running by looking for Runner.Worker/Listener
fn is_runner_process_running(runner_path: &std::path::Path) -> bool {
    // Validate path to prevent command injection via pgrep pattern
    if validate_path(runner_path).is_err() {
        return false;
    }

    let path_str = runner_path.to_string_lossy();

    // Patterns to search for (Runner.Worker, Runner.Listener, or just the path in any dotnet process)
    let patterns = [
        format!("Runner.Worker.*{}", path_str),
        format!("Runner.Listener.*{}", path_str),
        format!("dotnet.*{}", path_str),
        path_str.to_string(), // Just the path - catches any process with this dir
    ];

    for pattern in &patterns {
        let output = Command::new("pgrep").args(["-f", pattern]).output();

        if let Ok(output) = output {
            if output.status.success() && !output.stdout.is_empty() {
                return true;
            }
        }
    }

    false
}

/// Refresh the status of all runners
pub fn refresh_runners(runners: &mut [Runner]) {
    for runner in runners.iter_mut() {
        runner.status = get_service_status(&runner.service_name, &runner.path);
    }
}

/// Allowed actions for runner control
const ALLOWED_ACTIONS: &[&str] = &["start", "stop", "restart"];

/// Control a runner service with input validation (cross-platform)
pub fn control_runner(runner: &Runner, action: &str) -> Result<String> {
    // Validate action is allowed
    if !ALLOWED_ACTIONS.contains(&action) {
        return Err(anyhow::anyhow!("Invalid action: {}", action));
    }

    // Validate service name matches expected pattern (alphanumeric, dots, hyphens only)
    if !runner
        .service_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(anyhow::anyhow!("Invalid service name format"));
    }

    // Validate service name starts with expected prefix
    if !runner.service_name.starts_with("actions.runner.") {
        return Err(anyhow::anyhow!(
            "Service name must start with 'actions.runner.'"
        ));
    }

    if cfg!(target_os = "macos") {
        control_runner_macos(runner, action)
    } else {
        control_runner_linux(runner, action)
    }
}

/// Control runner on Linux using systemctl with svc.sh/run.sh fallback
fn control_runner_linux(runner: &Runner, action: &str) -> Result<String> {
    // Try systemctl first
    if let Some(result) = try_systemctl_control(runner, action)? {
        return Ok(result);
    }

    // Fallback to svc.sh script
    if let Some(result) = try_svc_script_control(runner, action, true)? {
        return Ok(result);
    }

    // Final fallback: direct run.sh control
    control_runner_direct(runner, action)
}

/// Attempt to control runner using systemctl, returns None if service doesn't exist
fn try_systemctl_control(runner: &Runner, action: &str) -> Result<Option<String>> {
    let unit_exists = Command::new("systemctl")
        .args(["cat", &runner.service_name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !unit_exists {
        return Ok(None);
    }

    let output = Command::new("sudo")
        .args(["systemctl", action, &runner.service_name])
        .output()?;

    if output.status.success() {
        Ok(Some(format!(
            "Successfully {}ed {}",
            action,
            runner.display_name()
        )))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(
            "Failed to {} {}: {}",
            action,
            runner.display_name(),
            stderr
        ))
    }
}

/// Attempt to control runner using svc.sh script, returns None if script doesn't exist
fn try_svc_script_control(runner: &Runner, action: &str, use_sudo: bool) -> Result<Option<String>> {
    let svc_script = runner.path.join("svc.sh");
    if !svc_script.exists() {
        return Ok(None);
    }

    // For start action, ensure service is installed first
    if action == "start" && needs_service_installation(&svc_script, &runner.path, use_sudo)? {
        install_service(&svc_script, &runner.path, runner, use_sudo)?;
    }

    let output = if use_sudo {
        Command::new("sudo")
            .arg(&svc_script)
            .arg(action)
            .current_dir(&runner.path)
            .output()?
    } else {
        Command::new(&svc_script)
            .arg(action)
            .current_dir(&runner.path)
            .output()?
    };

    if output.status.success() {
        Ok(Some(format!(
            "Successfully {}ed {}",
            action,
            runner.display_name()
        )))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(
            "Failed to {} {}: {}",
            action,
            runner.display_name(),
            stderr
        ))
    }
}

/// Check if service needs installation by running status command
fn needs_service_installation(
    svc_script: &Path,
    runner_path: &Path,
    use_sudo: bool,
) -> Result<bool> {
    let status_output = if use_sudo {
        Command::new("sudo")
            .arg(svc_script)
            .arg("status")
            .current_dir(runner_path)
            .output()
    } else {
        Command::new(svc_script)
            .arg("status")
            .current_dir(runner_path)
            .output()
    };

    match status_output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(stdout.contains("not installed") || stderr.contains("not installed"))
        }
        Err(_) => Ok(true),
    }
}

/// Install service using svc.sh install command
fn install_service(
    svc_script: &Path,
    runner_path: &Path,
    runner: &Runner,
    use_sudo: bool,
) -> Result<()> {
    let install_output = if use_sudo {
        Command::new("sudo")
            .arg(svc_script)
            .arg("install")
            .current_dir(runner_path)
            .output()?
    } else {
        Command::new(svc_script)
            .arg("install")
            .current_dir(runner_path)
            .output()?
    };

    if !install_output.status.success() {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        return Err(anyhow::anyhow!(
            "Failed to install service for {}: {}",
            runner.display_name(),
            stderr
        ));
    }

    Ok(())
}

/// Control runner directly using run.sh script and process management
fn control_runner_direct(runner: &Runner, action: &str) -> Result<String> {
    validate_path(&runner.path)?;

    let run_script = runner.path.join("run.sh");
    let run_script_str = run_script
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in path: {:?}", run_script))?;

    match action {
        "start" => {
            Command::new("nohup")
                .arg(run_script_str)
                .current_dir(&runner.path)
                .spawn()
                .with_context(|| format!("Failed to start runner {}", runner.display_name()))?;
            Ok(format!("Started {}", runner.display_name()))
        }
        "stop" => {
            stop_runner_process(runner)?;
            Ok(format!("Stopped {}", runner.display_name()))
        }
        "restart" => {
            restart_runner_process(runner, run_script_str)?;
            Ok(format!("Restarted {}", runner.display_name()))
        }
        _ => Err(anyhow::anyhow!("Invalid action: {}", action)),
    }
}

/// Stop runner process using pkill
fn stop_runner_process(runner: &Runner) -> Result<()> {
    // Validate path to prevent command injection via pkill pattern
    validate_path(&runner.path)?;

    let path_str = runner.path.to_string_lossy();
    Command::new("pkill")
        .args(["-f", &format!("Runner.*{}", path_str)])
        .output()
        .with_context(|| format!("Failed to stop runner {}", runner.display_name()))?;
    Ok(())
}

/// Restart runner process by stopping, waiting for termination, and starting again
fn restart_runner_process(runner: &Runner, run_script_str: &str) -> Result<()> {
    // Validate path to prevent command injection via pkill pattern
    validate_path(&runner.path)?;

    let path_str = runner.path.to_string_lossy();
    let _ = Command::new("pkill")
        .args(["-f", &format!("Runner.*{}", path_str)])
        .output();

    // Poll for process termination (up to 5 seconds)
    let timeout = std::time::Duration::from_secs(5);
    let start = std::time::Instant::now();
    while is_runner_process_running(&runner.path) {
        if start.elapsed() > timeout {
            return Err(anyhow::anyhow!(
                "Timeout waiting for runner {} to stop",
                runner.display_name()
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Command::new("nohup")
        .arg(run_script_str)
        .current_dir(&runner.path)
        .spawn()
        .with_context(|| format!("Failed to restart runner {}", runner.display_name()))?;

    Ok(())
}

/// Control runner on macOS using launchctl or direct script
fn control_runner_macos(runner: &Runner, action: &str) -> Result<String> {
    // Try launchctl first
    if let Some(result) = try_launchctl_control(runner, action)? {
        return Ok(result);
    }

    // Fallback to svc.sh script (without sudo on macOS)
    if let Some(result) = try_svc_script_control(runner, action, false)? {
        return Ok(result);
    }

    // Final fallback: direct run.sh control
    control_runner_direct(runner, action)
}

/// Attempt to control runner using launchctl, returns None if service doesn't exist
fn try_launchctl_control(runner: &Runner, action: &str) -> Result<Option<String>> {
    let plist_path = format!("~/Library/LaunchAgents/{}.plist", runner.service_name);
    let expanded_plist = shellexpand::tilde(&plist_path);

    if !std::path::Path::new(expanded_plist.as_ref()).exists() {
        return Ok(None);
    }

    let launchctl_action = match action {
        "start" => "load",
        "stop" => "unload",
        "restart" => "kickstart",
        _ => return Err(anyhow::anyhow!("Invalid action")),
    };

    let output = if action == "restart" {
        Command::new("launchctl")
            .args([
                "kickstart",
                "-k",
                &format!("gui/{}/{}", get_uid(), runner.service_name),
            ])
            .output()?
    } else {
        Command::new("launchctl")
            .args([launchctl_action, expanded_plist.as_ref()])
            .output()?
    };

    if output.status.success() {
        Ok(Some(format!(
            "Successfully {}ed {}",
            action,
            runner.display_name()
        )))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(
            "Failed to {} {}: {}",
            action,
            runner.display_name(),
            stderr
        ))
    }
}

/// Get current user ID for launchctl service domain.
///
/// # Safety
/// This is safe because `libc::getuid()` is a simple read-only system call that:
/// - Always succeeds (no error conditions)
/// - Has no side effects
/// - Only reads process state
/// - Returns a primitive value (u32)
fn get_uid() -> u32 {
    // SAFETY: libc::getuid() is always safe to call - it's a simple
    // read-only syscall with no failure modes or preconditions
    unsafe { libc::getuid() }
}

/// Get recent logs for a runner (cross-platform)
pub fn get_runner_logs(runner: &Runner, lines: usize) -> Result<Vec<String>> {
    if cfg!(target_os = "macos") {
        get_runner_logs_macos(runner, lines)
    } else {
        get_runner_logs_linux(runner, lines)
    }
}

/// Get logs on Linux using journalctl
fn get_runner_logs_linux(runner: &Runner, lines: usize) -> Result<Vec<String>> {
    let output = Command::new("journalctl")
        .args([
            "-u",
            &runner.service_name,
            "-n",
            &lines.to_string(),
            "--no-pager",
            "-o",
            "short-iso",
        ])
        .output()?;

    let logs = String::from_utf8_lossy(&output.stdout);
    Ok(logs.lines().map(|s| s.to_string()).collect())
}

/// Get logs on macOS from _diag directory
fn get_runner_logs_macos(runner: &Runner, lines: usize) -> Result<Vec<String>> {
    let diag_dir = runner.path.join("_diag");

    if !diag_dir.exists() {
        return Ok(vec!["No logs found (no _diag directory)".to_string()]);
    }

    // Find the most recent Worker log file
    let mut log_files: Vec<_> = std::fs::read_dir(&diag_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("Worker_"))
        .collect();

    log_files.sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok())));

    if let Some(latest_log) = log_files.first() {
        let content = std::fs::read_to_string(latest_log.path())?;
        let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let start = all_lines.len().saturating_sub(lines);
        Ok(all_lines[start..].to_vec())
    } else {
        // Try Runner log if no Worker log
        let mut runner_logs: Vec<_> = std::fs::read_dir(&diag_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("Runner_"))
            .collect();

        runner_logs
            .sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok())));

        if let Some(latest_log) = runner_logs.first() {
            let content = std::fs::read_to_string(latest_log.path())?;
            let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            let start = all_lines.len().saturating_sub(lines);
            Ok(all_lines[start..].to_vec())
        } else {
            Ok(vec!["No log files found in _diag".to_string()])
        }
    }
}
