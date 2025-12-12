use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

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

            let status = get_service_status(&service_name);

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

/// Get the status of a systemd service
fn get_service_status(service_name: &str) -> RunnerStatus {
    let output = Command::new("systemctl")
        .args(["is-active", service_name])
        .output();

    match output {
        Ok(output) => {
            let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
            match status.as_str() {
                "active" => RunnerStatus::Active,
                "inactive" => RunnerStatus::Inactive,
                "failed" => RunnerStatus::Failed,
                _ => RunnerStatus::NotFound,
            }
        }
        Err(_) => RunnerStatus::NotFound,
    }
}

/// Refresh the status of all runners
pub fn refresh_runners(runners: &mut [Runner]) {
    for runner in runners.iter_mut() {
        runner.status = get_service_status(&runner.service_name);
    }
}

/// Allowed systemctl actions for runner control
const ALLOWED_ACTIONS: &[&str] = &["start", "stop", "restart"];

/// Control a runner service with input validation
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

    let output = Command::new("sudo")
        .args(["systemctl", action, &runner.service_name])
        .output()?;

    if output.status.success() {
        Ok(format!(
            "Successfully {}ed {}",
            action,
            runner.display_name()
        ))
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

/// Get recent logs for a runner
pub fn get_runner_logs(runner: &Runner, lines: usize) -> Result<Vec<String>> {
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
