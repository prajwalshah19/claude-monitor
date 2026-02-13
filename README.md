# claude-monitor

A lightweight macOS menubar app that tracks memory usage across your running [Claude Code](https://docs.anthropic.com/en/docs/claude-code) sessions.

If you run multiple Claude Code instances, they can silently eat through your RAM. This sits in your menubar and shows you exactly where it's going.

## What it does

- Shows total Claude Code memory in your menubar (e.g. `CC 5.2G`)
- Click to see per-session breakdown: project name, RAM, PID, age, CPU
- Kill individual sessions from the dropdown
- Sends a notification when a session exceeds your RAM threshold
- Uses `phys_footprint` — the same metric Activity Monitor uses, including compressed memory

## Install

### From source (requires Rust)

```sh
cargo install --git https://github.com/prajwalshah19/claude-monitor
```

### Build locally

```sh
git clone https://github.com/prajwalshah19/claude-monitor
cd claude-monitor
cargo build --release
cp target/release/claude-monitor /usr/local/bin/
```

## Usage

```sh
claude-monitor
```

That's it. A `CC` icon appears in your menubar. Click it to see your sessions.

To run it in the background:

```sh
nohup claude-monitor &>/dev/null &
```

### Launch at login (optional)

Create `~/Library/LaunchAgents/com.claude-monitor.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.claude-monitor</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/claude-monitor</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
```

Then enable it:

```sh
launchctl load ~/Library/LaunchAgents/com.claude-monitor.plist
```

## Config

Config lives at `~/.claude-monitor/config.toml`. Created automatically on first run.

```toml
ram_threshold_gb = 8.0    # notify when a session exceeds this
poll_interval_secs = 30   # how often to check processes
notify_enabled = true     # macOS notifications on threshold breach
```

## Requirements

- macOS (uses macOS-specific APIs for memory reporting and tray icon)
- Rust 1.70+ to build

## How it works

Scans for running `claude` processes using `sysinfo`, then reads each process's `phys_footprint` via `libproc` (`proc_pid_rusage`) — the same metric Activity Monitor reports. This includes compressed memory, so the numbers match what you see in Activity Monitor.

The menubar updates every poll interval. The native dropdown menu rebuilds automatically when sessions change.

## License

MIT
