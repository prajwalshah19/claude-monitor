use libproc::pid_rusage::{pidrusage, RUsageInfoV2};
use sysinfo::System;

fn main() {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    println!("{:<8} {:>14} {:>14}", "PID", "sysinfo(RSS)", "phys_footprint");
    println!("{}", "-".repeat(40));

    for (pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_string();
        if name != "claude" {
            continue;
        }

        let pid_u32 = pid.as_u32();
        let sysinfo_mb = process.memory() as f64 / (1024.0 * 1024.0);

        let footprint_mb = match pidrusage::<RUsageInfoV2>(pid_u32 as i32) {
            Ok(info) => info.ri_phys_footprint as f64 / (1024.0 * 1024.0),
            Err(_) => 0.0,
        };

        println!(
            "{:<8} {:>11.0} MB {:>11.0} MB",
            pid_u32, sysinfo_mb, footprint_mb
        );
    }
}
