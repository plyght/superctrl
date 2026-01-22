# superctrl

Voice-controlled macOS automation powered by Anthropic's Computer Use API. Speak natural language commands to control your computer through a persistent menu bar daemon.

## Overview

superctrl bridges voice input and computer automation by integrating Superwhisper voice transcription with Anthropic's Claude Computer Use API. The system runs as a macOS menu bar application, listening for voice commands via macrowhisper trigger patterns and executing them through Claude's vision-enabled automation capabilities.

## Features

- **Voice Control**: Natural language commands via macrowhisper integration ("Computer, open Safari and go to github.com")
- **Computer Use API**: Full Anthropic Claude Computer Use implementation with screenshot analysis and action execution
- **Menu Bar Interface**: Real-time status, action history, and task control via native macOS menu bar
- **Emergency Stop**: Global hotkey (Command+Shift+Escape) to halt any running automation
- **Daemon Architecture**: IPC-based daemon with Unix socket communication for reliable background operation
- **Learning System**: Optional keyboard and clipboard monitoring for behavior analysis
- **Cross-Interface**: CLI, voice, and GUI control methods

## Installation

```bash
git clone https://github.com/yourusername/superctrl.git
cd superctrl
./install.sh
```

The installation script will:
- Build the release binary
- Install to `/usr/local/bin/superctrl`
- Configure macrowhisper action
- Set up launchd daemon for automatic startup

### Manual Installation

```bash
cargo build --release
sudo cp target/release/superctrl /usr/local/bin/
sudo chmod +x /usr/local/bin/superctrl

./install-macrowhisper-action.sh

cp superctrl.plist ~/Library/LaunchAgents/com.superctrl.daemon.plist
launchctl load ~/Library/LaunchAgents/com.superctrl.daemon.plist
```

Edit the plist file to add your Anthropic API key before loading.

## Usage

### Voice Commands

Trigger patterns (configurable in macrowhisper):

```
Computer, [command]
Automate [command]
Control [command]
Do this: [command]
```

Examples:

```
"Computer, open Safari and navigate to github.com"
"Automate taking a screenshot and saving to Desktop"
"Control moving all PDFs from Downloads to Documents"
```

### CLI

```bash
superctrl --execute "open Terminal and run 'git status'"
superctrl status
superctrl stop
```

### Menu Bar

Click the menu bar icon to:
- View current status and recent action history
- Stop running tasks
- Open preferences
- Quit the application

### Emergency Stop

Press Command+Shift+Escape at any time to immediately halt execution.

## Configuration

### Environment Variables

```bash
export ANTHROPIC_API_KEY=your-key-here
export SUPERCTRL_LEARNING_ENABLED=true
export SUPERCTRL_LEARNING_DB_PATH=~/.config/superctrl/learning.db
export SUPERCTRL_SYSTEM_PROMPT_PATH=~/.config/superctrl/system_prompt.txt
```

### macrowhisper Trigger Patterns

Edit `~/.config/macrowhisper/macrowhisper.json`:

```json
{
  "scriptsShell": {
    "superctrl": {
      "action": "/usr/local/bin/superctrl --execute '{{swResult}}'",
      "triggerVoice": "computer|automate|control|do this"
    }
  }
}
```

### Fish Shell Setup

Fish shell uses different syntax for environment variables:

**Temporary (current session):**
```fish
set -x ANTHROPIC_API_KEY 'your-key-here'
```

**Permanent (add to `~/.config/fish/config.fish`):**
```fish
set -x ANTHROPIC_API_KEY 'your-key-here'
```

**Verify:**
```fish
echo $ANTHROPIC_API_KEY
```

## Architecture

- `computer_use.rs`: Anthropic Computer Use API loop with claude-sonnet-4-5
- `automation.rs`: macOS action execution via enigo (mouse, keyboard, scroll)
- `screenshot.rs`: Screen capture with xcap and automatic scaling
- `menu_bar.rs`: Native menu bar implementation using tray-icon
- `gui.rs`: Shared state management with Arc<Mutex<GuiState>>
- `hotkey.rs`: Global keyboard shortcut handling via global-hotkey
- `ipc.rs`: Unix socket server for daemon communication
- `learning.rs`: User behavior collection with SQLite storage
- `cli.rs`: Command-line interface using clap

## API Details

- **Model**: claude-sonnet-4-5
- **API**: Anthropic Messages API with computer-use-2025-01-24 beta
- **Tools**: computer_20250124 tool version
- **Display**: Automatic screen resolution detection with dynamic scaling
- **Actions**: left_click, right_click, type, key, mouse_move, scroll, screenshot, double_click, triple_click, left_click_drag
- **Safety**: 50 iteration limit, atomic stop flag, full trust mode toggle

## Development

```bash
cargo build
cargo test
cargo clippy
cargo fmt
```

Requires Rust 1.70+. Key dependencies: iced, tokio, reqwest, xcap, enigo, global-hotkey, tray-icon, rusqlite, rdev, arboard.

### Running Locally

Terminal 1 (daemon):
```bash
ANTHROPIC_API_KEY=your-key cargo run
```

Terminal 2 (client):
```bash
cargo run -- --execute "test command"
```

### Testing IPC

```bash
echo '{"Execute":{"command":"test"}}' | nc -U /tmp/superctrl.sock
```

### Testing Without Voice

**Method 1: Direct CLI (simulates macrowhisper)**

```bash
./target/release/superctrl -e "Open Safari"
./target/release/superctrl -e "Take a screenshot"
```

**Method 2: Quick Test Script**

```bash
./test_integration.sh
```

This script checks daemon status, sends a test command, and shows results.

## Requirements

- macOS (10.15+)
- Rust toolchain
- Superwhisper with macrowhisper installed
- Anthropic API key with Computer Use beta access
- Accessibility permissions for keyboard shortcuts and automation

## Security

- Socket permissions restricted to owner only (0600)
- API key loaded from environment, never hardcoded
- No credential logging
- Learning data stored locally with configurable opt-out
- Command validation before execution

## Troubleshooting

### Check Daemon Status

```bash
superctrl status
```

### View Logs

```bash
tail -f ~/Library/Logs/superctrl.log
tail -f ~/Library/Logs/superctrl.error.log
```

### Restart Daemon

```bash
launchctl unload ~/Library/LaunchAgents/com.superctrl.daemon.plist
launchctl load ~/Library/LaunchAgents/com.superctrl.daemon.plist
```

### Verify macrowhisper Configuration

```bash
macrowhisper --service-status
cat ~/.config/macrowhisper/macrowhisper.json | grep -A 3 superctrl
```

## License

MIT License
