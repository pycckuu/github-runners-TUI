use crate::runner::{
    control_runner, discover_runners, get_runner_logs, refresh_runners, Runner, RunnerStatus,
};
use anyhow::Result;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Logs,
    Help,
}

/// Messages sent from main thread to background worker
#[derive(Debug)]
pub enum WorkerCommand {
    Refresh,
    ControlRunner { runner_index: usize, action: String },
    Shutdown,
}

/// Messages sent from background worker to main thread
#[derive(Debug)]
pub enum WorkerResponse {
    RunnersUpdated(Vec<Runner>),
    ActionComplete { message: String },
}

#[derive(Debug, Clone)]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub load_avg: [f64; 3],
}

impl Default for SystemStats {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 1,
            load_avg: [0.0, 0.0, 0.0],
        }
    }
}

pub struct App {
    pub runners: Vec<Runner>,
    pub selected: usize,
    pub system_stats: SystemStats,
    pub should_quit: bool,
    pub mode: AppMode,
    pub status_message: Option<String>,
    pub logs: Vec<String>,
    pub log_scroll: usize,
    system: System,
    command_tx: Sender<WorkerCommand>,
    response_rx: Receiver<WorkerResponse>,
}

impl App {
    pub fn new() -> Result<Self> {
        let runners = discover_runners()?;
        let mut system = System::new_all();
        system.refresh_all();

        let system_stats = Self::collect_system_stats(&system);

        // Create channels for background worker communication
        let (command_tx, command_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();

        // Spawn background worker thread
        let runners_clone = runners.clone();
        std::thread::spawn(move || {
            worker_thread(runners_clone, command_rx, response_tx);
        });

        Ok(Self {
            runners,
            selected: 0,
            system_stats,
            should_quit: false,
            mode: AppMode::Normal,
            status_message: None,
            logs: Vec::new(),
            log_scroll: 0,
            system,
            command_tx,
            response_rx,
        })
    }

    fn collect_system_stats(system: &System) -> SystemStats {
        let load_avg = System::load_average();
        SystemStats {
            cpu_usage: system.global_cpu_usage(),
            memory_used: system.used_memory(),
            memory_total: system.total_memory(),
            load_avg: [load_avg.one, load_avg.five, load_avg.fifteen],
        }
    }

    /// Request a background refresh of runner statuses.
    pub fn refresh(&mut self) {
        // Send refresh command to background worker (non-blocking)
        if self.command_tx.send(WorkerCommand::Refresh).is_err() {
            self.status_message = Some("Warning: Worker thread unavailable".to_string());
        }

        // Refresh system stats (lightweight operation)
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.system_stats = Self::collect_system_stats(&self.system);

        // Refresh logs if in log mode (file I/O, could be optimized later)
        if self.mode == AppMode::Logs {
            self.refresh_logs();
        }
    }

    /// Poll for updates from the background worker (non-blocking).
    pub fn poll_worker_updates(&mut self) {
        loop {
            match self.response_rx.try_recv() {
                Ok(WorkerResponse::RunnersUpdated(updated_runners)) => {
                    // Update runners while preserving selection
                    self.runners = updated_runners;
                    // Ensure selection is still valid
                    if self.selected >= self.runners.len() && !self.runners.is_empty() {
                        self.selected = self.runners.len() - 1;
                    }
                }
                Ok(WorkerResponse::ActionComplete { message }) => {
                    self.status_message = Some(message);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.status_message =
                        Some("ERROR: Background worker crashed. Data may be stale.".to_string());
                    break;
                }
            }
        }
    }

    const LOG_LINES: usize = 100;

    pub fn refresh_logs(&mut self) {
        if let Some(runner) = self.selected_runner() {
            if let Ok(logs) = get_runner_logs(runner, Self::LOG_LINES) {
                self.logs = logs;
            }
        }
    }

    pub fn selected_runner(&self) -> Option<&Runner> {
        self.runners.get(self.selected)
    }

    pub fn select_next(&mut self) {
        if !self.runners.is_empty() {
            self.selected = (self.selected + 1) % self.runners.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.runners.is_empty() {
            self.selected = self
                .selected
                .checked_sub(1)
                .unwrap_or(self.runners.len() - 1);
        }
    }

    pub fn scroll_logs_up(&mut self) {
        self.log_scroll = self.log_scroll.saturating_sub(1);
    }

    pub fn scroll_logs_down(&mut self) {
        if self.log_scroll < self.logs.len().saturating_sub(1) {
            self.log_scroll += 1;
        }
    }

    pub fn start_selected(&mut self) {
        self.control_selected_runner("start");
    }

    pub fn stop_selected(&mut self) {
        self.control_selected_runner("stop");
    }

    pub fn restart_selected(&mut self) {
        self.control_selected_runner("restart");
    }

    fn control_selected_runner(&mut self, action: &str) {
        // Show pending status immediately
        let mut capitalized = action.to_string();
        if let Some(first) = capitalized.get_mut(0..1) {
            first.make_ascii_uppercase();
        }

        // Send command to background worker
        if self
            .command_tx
            .send(WorkerCommand::ControlRunner {
                runner_index: self.selected,
                action: action.to_string(),
            })
            .is_err()
        {
            self.status_message = Some("Error: Worker thread unavailable".to_string());
            return;
        }

        self.status_message = Some(format!("{}ing runner...", capitalized));
    }

    pub fn toggle_logs(&mut self) {
        if self.mode == AppMode::Logs {
            self.mode = AppMode::Normal;
            self.logs.clear();
            self.log_scroll = 0;
        } else {
            self.mode = AppMode::Logs;
            self.refresh_logs();
            // Scroll to bottom
            self.log_scroll = self.logs.len().saturating_sub(1);
        }
    }

    pub fn toggle_help(&mut self) {
        self.mode = if self.mode == AppMode::Help {
            AppMode::Normal
        } else {
            AppMode::Help
        };
    }

    pub fn counts(&self) -> (usize, usize, usize) {
        let active = self
            .runners
            .iter()
            .filter(|r| r.status == RunnerStatus::Active)
            .count();
        let failed = self
            .runners
            .iter()
            .filter(|r| r.status == RunnerStatus::Failed)
            .count();
        let total = self.runners.len();
        (active, failed, total)
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Signal worker thread to shutdown
        let _ = self.command_tx.send(WorkerCommand::Shutdown);
    }
}

/// Background worker thread that handles runner refresh and control operations.
fn worker_thread(
    mut runners: Vec<Runner>,
    command_rx: Receiver<WorkerCommand>,
    response_tx: Sender<WorkerResponse>,
) {
    use std::time::Duration;

    loop {
        // Wait for command with timeout to allow periodic refresh
        match command_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(WorkerCommand::Refresh) => {
                // Refresh all runners
                refresh_runners(&mut runners);

                // Send updated runners back to main thread
                let _ = response_tx.send(WorkerResponse::RunnersUpdated(runners.clone()));
            }
            Ok(WorkerCommand::ControlRunner {
                runner_index,
                action,
            }) => {
                // Execute control action with bounds checking
                let message = if let Some(runner) = runners.get(runner_index).cloned() {
                    match control_runner(&runner, &action) {
                        Ok(msg) => msg,
                        Err(e) => format!("Error: {}", e),
                    }
                } else {
                    format!(
                        "Error: Runner index {} out of bounds (have {} runners)",
                        runner_index,
                        runners.len()
                    )
                };

                // Refresh runners after control action
                refresh_runners(&mut runners);

                // Always send response
                let _ = response_tx.send(WorkerResponse::RunnersUpdated(runners.clone()));
                let _ = response_tx.send(WorkerResponse::ActionComplete { message });
            }
            Ok(WorkerCommand::Shutdown) => {
                // Exit worker thread
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // No command received, continue loop
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Main thread dropped, exit
                break;
            }
        }
    }
}
