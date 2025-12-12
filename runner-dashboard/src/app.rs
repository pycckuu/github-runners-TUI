use crate::runner::{
    control_runner, discover_runners, get_runner_logs, refresh_runners, Runner, RunnerStatus,
};
use anyhow::Result;
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    Logs,
    Help,
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
}

impl App {
    pub fn new() -> Result<Self> {
        let runners = discover_runners()?;
        let mut system = System::new_all();
        system.refresh_all();

        let system_stats = Self::collect_system_stats(&system);

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

    pub fn refresh(&mut self) {
        // Refresh runner statuses
        refresh_runners(&mut self.runners);

        // Refresh system stats
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.system_stats = Self::collect_system_stats(&self.system);

        // Refresh logs if in log mode
        if self.mode == AppMode::Logs {
            self.refresh_logs();
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
        if let Some(runner) = self.selected_runner().cloned() {
            match control_runner(&runner, action) {
                Ok(msg) => self.status_message = Some(msg),
                Err(e) => self.status_message = Some(format!("Error: {}", e)),
            }
        }
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
