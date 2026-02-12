use std::collections::HashMap;

use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

use crate::config::Config;
use crate::monitor::{self, ClaudeSession};

pub struct TrayState {
    pub tray: TrayIcon,
    pub quit_id: MenuId,
    pub kill_actions: HashMap<MenuId, u32>,
    session_hash: u64,
}

impl TrayState {
    pub fn rebuild_if_changed(&mut self, sessions: &[ClaudeSession], config: &Config) -> bool {
        let hash = session_hash(sessions);
        if hash == self.session_hash {
            return false;
        }
        self.session_hash = hash;

        let (menu, quit_id, kill_actions) = build_menu(sessions, config);
        self.tray.set_menu(Some(Box::new(menu)));
        self.quit_id = quit_id;
        self.kill_actions = kill_actions;

        let total = monitor::total_ram_bytes(sessions);
        let title = if sessions.is_empty() {
            "CC".to_string()
        } else {
            format!("CC {}", monitor::format_bytes(total))
        };
        self.tray.set_title(Some(&title));

        true
    }
}

pub fn create_tray(config: &Config) -> TrayState {
    let icon = create_icon();
    let (menu, quit_id, kill_actions) = build_menu(&[], config);

    let tray = TrayIconBuilder::new()
        .with_icon(icon)
        .with_title("CC")
        .with_menu(Box::new(menu))
        .build()
        .expect("Failed to create tray icon");

    TrayState {
        tray,
        quit_id,
        kill_actions,
        session_hash: 0,
    }
}

fn build_menu(
    sessions: &[ClaudeSession],
    config: &Config,
) -> (Menu, MenuId, HashMap<MenuId, u32>) {
    let menu = Menu::new();
    let mut kill_actions = HashMap::new();

    let total_ram = monitor::total_ram_bytes(sessions);
    let threshold_bytes = (config.ram_threshold_gb * 1024.0 * 1024.0 * 1024.0) as u64;

    // Summary header
    let status = if total_ram > threshold_bytes {
        format!(
            "!! {} / {}G limit  ({} sessions)",
            monitor::format_bytes(total_ram),
            config.ram_threshold_gb,
            sessions.len()
        )
    } else {
        format!(
            "{} / {}G limit  ({} sessions)",
            monitor::format_bytes(total_ram),
            config.ram_threshold_gb,
            sessions.len()
        )
    };
    let header = MenuItem::new(status, false, None);
    menu.append(&header).unwrap();

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    if sessions.is_empty() {
        let empty = MenuItem::new("No Claude sessions", false, None);
        menu.append(&empty).unwrap();
    } else {
        for session in sessions {
            let ram_str = monitor::format_bytes(session.ram_bytes);

            // Session info line
            let info = MenuItem::new(
                format!(
                    "{} — {}   (PID {} · {} · CPU {:.0}%)",
                    session.project,
                    ram_str,
                    session.pid,
                    session.age_string(),
                    session.cpu_percent,
                ),
                false,
                None,
            );
            menu.append(&info).unwrap();

            // Kill button
            let kill = MenuItem::new(
                format!("    Kill {}", session.project),
                true,
                None,
            );
            kill_actions.insert(kill.id().clone(), session.pid);
            menu.append(&kill).unwrap();
        }
    }

    menu.append(&PredefinedMenuItem::separator()).unwrap();

    let quit = MenuItem::new("Quit", true, None);
    let quit_id = quit.id().clone();
    menu.append(&quit).unwrap();

    (menu, quit_id, kill_actions)
}

fn session_hash(sessions: &[ClaudeSession]) -> u64 {
    let mut hash = sessions.len() as u64;
    for s in sessions {
        hash = hash.wrapping_mul(31).wrapping_add(s.pid as u64);
        hash = hash.wrapping_mul(31).wrapping_add(s.ram_bytes);
    }
    hash
}

fn create_icon() -> tray_icon::Icon {
    let size: u32 = 22;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let center = size as f32 / 2.0;
    let radius = 8.0f32;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center + 0.5;
            let dy = y as f32 - center + 0.5;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * size + x) * 4) as usize;
            if dist <= radius {
                rgba[idx] = 0;
                rgba[idx + 1] = 0;
                rgba[idx + 2] = 0;
                rgba[idx + 3] = 255;
            }
        }
    }

    tray_icon::Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}
