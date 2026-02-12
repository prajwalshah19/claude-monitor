use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::monitor::ClaudeSession;

pub type NotifiedPids = Arc<Mutex<HashSet<u32>>>;

pub fn check_and_notify(
    sessions: &[ClaudeSession],
    threshold_gb: f64,
    notified: &NotifiedPids,
) {
    let mut notified_lock = notified.lock().unwrap();

    // Clean up PIDs no longer running
    let active_pids: HashSet<u32> = sessions.iter().map(|s| s.pid).collect();
    notified_lock.retain(|pid| active_pids.contains(pid));

    for session in sessions {
        if session.ram_gb() > threshold_gb && !notified_lock.contains(&session.pid) {
            notified_lock.insert(session.pid);
            let pid = session.pid;
            let project = session.project.clone();
            let ram_gb = session.ram_gb();

            std::thread::spawn(move || {
                send_notification(pid, &project, ram_gb);
            });
        }
    }
}

fn send_notification(pid: u32, project: &str, ram_gb: f64) {
    let title = "Claude Monitor: High Memory";
    let message = format!("{} (PID {}) using {:.1}G RAM", project, pid, ram_gb);

    // Try mac-notification-sys first
    if try_mac_notification(pid, title, &message).is_ok() {
        return;
    }

    // Fallback: osascript
    let script = format!(
        "display notification \"{}\" with title \"{}\"",
        message.replace('"', "\\\""),
        title.replace('"', "\\\""),
    );
    let _ = std::process::Command::new("osascript")
        .args(["-e", &script])
        .status();
}

fn try_mac_notification(
    pid: u32,
    title: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use mac_notification_sys::*;

    let _ = set_application("com.apple.Terminal");

    let response = Notification::new()
        .title(title)
        .message(message)
        .main_button(MainButton::SingleAction("Kill"))
        .send()?;

    if let NotificationResponse::ActionButton(_) = response {
        let _ = std::process::Command::new("kill")
            .arg(pid.to_string())
            .status();
    }

    Ok(())
}
