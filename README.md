# Superctrl

AI-powered computer automation for macOS via natural language commands. Superctrl runs as a menu bar daemon, leveraging OpenAI's Computer Use API to understand and execute complex desktop tasks through vision and action.

## Overview

Superctrl bridges human intent and machine execution by providing an intelligent automation layer for macOS. Through natural language commands, it captures screenshots, interprets the desktop state, and performs precise mouse and keyboard actions to complete tasks autonomously.

## Features

- **Computer Use API**: OpenAI GPT-4o integration with vision and tool calling for intelligent desktop automation
- **Menu Bar Daemon**: Persistent background service with status monitoring and action history
- **Emergency Stop**: System-wide hotkey (⌘⇧⎋) to immediately halt any running automation
- **IPC Command Server**: Unix socket interface for external command execution and integration
- **Native Automation**: Cross-platform mouse and keyboard control via enigo
- **Screen Capture**: High-fidelity screenshot capture and encoding with xcap
- **Safety Controls**: Iteration limits, trust mode, and stop flags prevent runaway execution

## Installation

```bash
# From source
git clone https://github.com/yourusername/superctrl.git
cd superctrl
cargo build --release
sudo cp target/release/superctrl /usr/local/bin/

# Configure API key
export OPENAI_API_KEY="your-api-key"

# Run daemon
superctrl
```

## Usage

```bash
# Start daemon (menu bar)
superctrl

# Check daemon status
superctrl status

# Execute command via IPC
superctrl execute "Open Safari and navigate to github.com"

# Stop daemon
superctrl stop
```

The menu bar interface shows real-time status (Idle/Working) and maintains history of the five most recent actions. Use ⌘⇧⎋ to emergency stop any task.

## Configuration

Superctrl can be configured as a launch agent for automatic startup:

```bash
# Copy plist to LaunchAgents
cp superctrl.plist ~/Library/LaunchAgents/com.superctrl.daemon.plist

# Edit to add your API key
nano ~/Library/LaunchAgents/com.superctrl.daemon.plist

# Load service
launchctl load ~/Library/LaunchAgents/com.superctrl.daemon.plist
```

### Display Settings

Modify `computer_use.rs` to adjust screenshot resolution (default: 1920x1080):

```rust
let mut agent = ComputerUseAgent::new(api_key, stop_flag)?
    .with_display_size(2560, 1440)
    .with_full_trust_mode(true);
```

### Safety Configuration

- **Full Trust Mode**: Auto-acknowledges safety checks (enabled by default)
- **Iteration Limit**: Maximum API loop cycles before timeout (default: 50)
- **Emergency Stop**: Always available via hotkey or IPC command

## Architecture

- `main.rs`: Application entry point and daemon orchestration
- `menu_bar.rs`: macOS menu bar interface and tray icon management
- `computer_use.rs`: OpenAI Computer Use API client and agent loop
- `screenshot.rs`: Screen capture with xcap and base64 encoding
- `automation.rs`: Mouse/keyboard automation via enigo with action parsing
- `ipc.rs`: Unix socket server for command execution
- `hotkey.rs`: Emergency stop hotkey registration and handling
- `gui.rs`: Shared state management and status tracking
- `preferences.rs`: Configuration UI and settings

### Computer Use Flow

1. Capture initial screenshot of desktop
2. Send command + image to OpenAI API with computer_use_preview tool
3. Parse tool calls from response (click, type, keypress, scroll, wait)
4. Execute actions via native automation
5. Capture new screenshot showing result
6. Send action result + new screenshot back to API
7. Repeat until task complete or max iterations reached

## Development

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

Requires Rust 1.70+. Key dependencies: iced, tokio, async-openai, xcap, enigo, tray-icon, global-hotkey.

### Examples

```bash
# Computer Use API example
OPENAI_API_KEY=your-key cargo run --example computer_use_example

# Emergency stop example
cargo run --example emergency_stop_example
```

## License

MIT License
