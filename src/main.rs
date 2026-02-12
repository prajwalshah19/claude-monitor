mod config;
mod monitor;
mod notify;
mod tray;

use std::collections::HashSet;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use tray_icon::menu::MenuEvent;

struct MonitorApp {
    sessions: monitor::SharedSessions,
    notified_pids: notify::NotifiedPids,
    config: config::Config,
    tray_state: tray::TrayState,
    menu_events: mpsc::Receiver<MenuEvent>,
}

impl MonitorApp {
    fn new(
        cc: &eframe::CreationContext,
        config: config::Config,
        tray_state: tray::TrayState,
    ) -> Self {
        let sessions = Arc::new(Mutex::new(Vec::new()));
        let poll_interval = Duration::from_secs(config.poll_interval_secs);

        let monitor_ctx = cc.egui_ctx.clone();
        monitor::start_monitor(sessions.clone(), poll_interval, monitor_ctx);

        let (menu_tx, menu_rx) = mpsc::channel();
        let menu_ctx = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            let receiver = MenuEvent::receiver();
            while let Ok(event) = receiver.recv() {
                let _ = menu_tx.send(event);
                menu_ctx.request_repaint();
            }
        });

        Self {
            sessions,
            notified_pids: Arc::new(Mutex::new(HashSet::new())),
            config,
            tray_state,
            menu_events: menu_rx,
        }
    }
}

impl eframe::App for MonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle menu events
        while let Ok(event) = self.menu_events.try_recv() {
            if self.tray_state.quit_id == event.id() {
                std::process::exit(0);
            }
            if let Some(&pid) = self.tray_state.kill_actions.get(event.id()) {
                monitor::kill_process(pid);
            }
        }

        // Rebuild tray menu when sessions change
        {
            let sessions = self.sessions.lock().unwrap();
            self.tray_state.rebuild_if_changed(&sessions, &self.config);

            // Check notification thresholds
            if self.config.notify_enabled {
                notify::check_and_notify(
                    &sessions,
                    self.config.ram_threshold_gb,
                    &self.notified_pids,
                );
            }
        }

        // Keep event loop alive
        ctx.request_repaint_after(Duration::from_secs(2));
    }
}

fn main() -> eframe::Result {
    let config = config::Config::load();
    let tray_state = tray::create_tray(&config);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1.0, 1.0])
            .with_position([-1000.0, -1000.0])
            .with_decorations(false)
            .with_visible(false),
        ..Default::default()
    };

    eframe::run_native(
        "Claude Monitor",
        options,
        Box::new(move |cc| Ok(Box::new(MonitorApp::new(cc, config, tray_state)))),
    )
}
