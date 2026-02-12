mod config;
mod monitor;
mod notify;
mod tray;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::MenuEvent;

fn main() {
    let config = config::Config::load();
    let event_loop = EventLoopBuilder::new().build();

    let mut tray_state = tray::create_tray(&config);

    let sessions: monitor::SharedSessions = Arc::new(Mutex::new(Vec::new()));
    let notified_pids: notify::NotifiedPids = Arc::new(Mutex::new(HashSet::new()));

    monitor::start_monitor(sessions.clone(), Duration::from_secs(config.poll_interval_secs));

    // Menu event receiver lives on main thread
    let menu_receiver = MenuEvent::receiver();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(
            std::time::Instant::now() + Duration::from_secs(config.poll_interval_secs),
        );

        // Handle menu clicks
        if let Ok(event) = menu_receiver.try_recv() {
            if tray_state.quit_id == event.id() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            if let Some(&pid) = tray_state.kill_actions.get(event.id()) {
                monitor::kill_process(pid);
            }
        }

        // On each wake: rebuild menu if sessions changed, check notifications
        if let Event::NewEvents(_) = event {
            let snap = sessions.lock().unwrap();
            tray_state.rebuild_if_changed(&snap, &config);

            if config.notify_enabled {
                notify::check_and_notify(&snap, config.ram_threshold_gb, &notified_pids);
            }
        }
    });
}
