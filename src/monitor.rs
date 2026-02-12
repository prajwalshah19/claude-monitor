use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use eframe::egui;
use sysinfo::System;

#[derive(Debug, Clone)]
pub struct ClaudeSession {
    pub pid: u32,
    pub project: String,
    pub cwd: PathBuf,
    pub ram_bytes: u64,
    pub cpu_percent: f32,
    pub started: SystemTime,
    pub flags: String,
}

impl ClaudeSession {
    pub fn ram_gb(&self) -> f64 {
        self.ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn age_string(&self) -> String {
        let elapsed = self.started.elapsed().unwrap_or_default();
        let secs = elapsed.as_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else if secs < 86400 {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        } else {
            format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
        }
    }
}

pub type SharedSessions = Arc<Mutex<Vec<ClaudeSession>>>;

pub fn start_monitor(
    sessions: SharedSessions,
    poll_interval: Duration,
    ctx: egui::Context,
) {
    std::thread::spawn(move || {
        let mut sys = System::new();
        loop {
            sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

            let mut found: Vec<ClaudeSession> = Vec::new();

            for (pid, process) in sys.processes() {
                if !is_claude_process(process) {
                    continue;
                }

                let pid_u32 = pid.as_u32();
                let cwd = get_cwd(pid_u32);
                let project = cwd
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let cmd: Vec<String> = process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().to_string())
                    .collect();
                let flags = cmd
                    .iter()
                    .filter(|a| a.starts_with('-'))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" ");

                let started =
                    SystemTime::UNIX_EPOCH + Duration::from_secs(process.start_time());

                found.push(ClaudeSession {
                    pid: pid_u32,
                    project,
                    cwd,
                    ram_bytes: process.memory(),
                    cpu_percent: process.cpu_usage(),
                    started,
                    flags,
                });
            }

            found.sort_by(|a, b| b.ram_bytes.cmp(&a.ram_bytes));

            if let Ok(mut lock) = sessions.lock() {
                *lock = found;
            }

            ctx.request_repaint();
            std::thread::sleep(poll_interval);
        }
    });
}

fn is_claude_process(process: &sysinfo::Process) -> bool {
    let name = process.name().to_string_lossy();
    if name == "claude" {
        return true;
    }

    // Check executable path
    if let Some(exe) = process.exe() {
        if let Some(exe_name) = exe.file_name() {
            if exe_name.to_string_lossy() == "claude" {
                return true;
            }
        }
    }

    false
}

fn get_cwd(pid: u32) -> PathBuf {
    // Try libproc first
    if let Ok(path) = libproc::proc_pid::pidcwd(pid as i32) {
        return path;
    }

    // Fallback: lsof
    if let Ok(output) = std::process::Command::new("lsof")
        .args(["-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(path) = line.strip_prefix('n') {
                if !path.is_empty() {
                    return PathBuf::from(path);
                }
            }
        }
    }

    PathBuf::from("unknown")
}

pub fn total_ram_bytes(sessions: &[ClaudeSession]) -> u64 {
    sessions.iter().map(|s| s.ram_bytes).sum()
}

pub fn format_bytes(bytes: u64) -> String {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gb >= 1.0 {
        format!("{:.1}G", gb)
    } else {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        format!("{:.0}M", mb)
    }
}

pub fn kill_process(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg(pid.to_string())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
