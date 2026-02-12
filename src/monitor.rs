use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use libproc::pid_rusage::{pidrusage, RUsageInfoV2};
use sysinfo::System;

#[derive(Debug, Clone)]
pub struct ClaudeSession {
    pub pid: u32,
    pub project: String,
    pub cwd: PathBuf,
    pub ram_bytes: u64,
    pub cpu_percent: f32,
    pub started: SystemTime,
}

impl ClaudeSession {
    pub fn ram_gb(&self) -> f64 {
        self.ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn age_string(&self) -> String {
        let secs = self.started.elapsed().unwrap_or_default().as_secs();
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

pub fn start_monitor(sessions: SharedSessions, poll_interval: Duration) {
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

                let started =
                    SystemTime::UNIX_EPOCH + Duration::from_secs(process.start_time());

                // Use phys_footprint (matches Activity Monitor) instead of RSS
                let ram_bytes = pidrusage::<RUsageInfoV2>(pid_u32 as i32)
                    .map(|info| info.ri_phys_footprint)
                    .unwrap_or(process.memory());

                found.push(ClaudeSession {
                    pid: pid_u32,
                    project,
                    cwd,
                    ram_bytes,
                    cpu_percent: process.cpu_usage(),
                    started,
                });
            }

            found.sort_by(|a, b| b.ram_bytes.cmp(&a.ram_bytes));

            if let Ok(mut lock) = sessions.lock() {
                *lock = found;
            }

            std::thread::sleep(poll_interval);
        }
    });
}

fn is_claude_process(process: &sysinfo::Process) -> bool {
    let name = process.name().to_string_lossy();
    if name == "claude" {
        return true;
    }
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
    // Try the process itself first
    if let Some(path) = get_pid_cwd(pid) {
        if path != PathBuf::from("/") {
            return path;
        }
    }
    // Claude's own CWD is often "/", so check parent shell's CWD
    if let Some(ppid) = get_ppid(pid) {
        if let Some(path) = get_pid_cwd(ppid) {
            if path != PathBuf::from("/") {
                return path;
            }
        }
    }
    PathBuf::from("unknown")
}

fn get_ppid(pid: u32) -> Option<u32> {
    let output = std::process::Command::new("ps")
        .args(["-o", "ppid=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout);
    s.trim().parse().ok()
}

fn get_pid_cwd(pid: u32) -> Option<PathBuf> {
    if let Ok(path) = libproc::proc_pid::pidcwd(pid as i32) {
        return Some(path);
    }
    if let Ok(output) = std::process::Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(path) = line.strip_prefix('n') {
                if !path.is_empty() && path != "/" {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }
    None
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
