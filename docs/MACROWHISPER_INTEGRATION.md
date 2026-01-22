# macrowhisper Integration

This document describes the integration between superctrl and macrowhisper for voice-controlled computer automation.

## Architecture

```
User speaks → Superwhisper transcribes → macrowhisper detects trigger pattern
                                              ↓
                                    superctrl --execute "command"
                                              ↓
                                    Unix socket IPC → Daemon
                                              ↓
                                    Computer Use API execution
                                              ↓
                                    Result shown in menu bar
```

## Installation

### Quick Install

Run the automated installation script:

```bash
cd /Users/nicojaffer/superctrl
./install.sh
```

This will:

1. Build the superctrl binary
2. Install it to `/usr/local/bin/superctrl`
3. Configure macrowhisper action
4. Set up launchd daemon to run on login

### Manual Installation

#### 1. Build and Install Binary

```bash
cargo build --release
sudo cp target/release/superctrl /usr/local/bin/superctrl
sudo chmod +x /usr/local/bin/superctrl
```

#### 2. Install macrowhisper Action

```bash
./install-macrowhisper-action.sh
```

#### 3. Set up Daemon

Copy and configure the launchd plist:

```bash
cp superctrl.plist ~/Library/LaunchAgents/com.superctrl.daemon.plist
```

Edit the plist and replace `REPLACE_WITH_YOUR_API_KEY` with your OpenAI API key.

Load the daemon:

```bash
launchctl load ~/Library/LaunchAgents/com.superctrl.daemon.plist
```

## Usage

### Voice Commands

Trigger superctrl using one of these voice patterns:

- "Computer, [command]"
- "Automate [command]"
- "Control [command]"
- "Do this: [command]"

**Examples:**

- "Computer, open Safari and go to github.com"
- "Automate creating a new folder called Projects"
- "Control moving all PDFs to Downloads"
- "Do this: take a screenshot and save it to Desktop"

### CLI Commands

```bash
superctrl --execute "open Safari"
```

```bash
superctrl status
```

```bash
superctrl stop
```

```bash
superctrl daemon
```

## IPC Protocol

The daemon listens on a Unix socket at `/tmp/superctrl.sock`.

### Request Format

JSON messages with the following structure:

```json
{
  "Execute": {
    "command": "open Safari and go to github.com"
  }
}
```

```json
{
  "Status": null
}
```

```json
{
  "Stop": null
}
```

### Response Format

```json
{
  "success": true,
  "message": "Command execution started"
}
```

```json
{
  "success": false,
  "message": "Error message here"
}
```

## Configuration

### macrowhisper Configuration

Location: `~/.config/macrowhisper/macrowhisper.json`

The installation script adds this action:

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

### Customizing Trigger Patterns

Edit `~/.config/macrowhisper/macrowhisper.json` and modify the `triggerVoice` field:

```json
"triggerVoice": "computer|hey computer|jarvis|assistant"
```

Restart macrowhisper for changes to take effect.

## Troubleshooting

### Check Daemon Status

```bash
superctrl status
```

### View Daemon Logs

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
cat ~/.config/macrowhisper/macrowhisper.json | grep -A 3 superctrl
```

### Test IPC Connection

```bash
superctrl --execute "test command"
```

If you get "Failed to connect to daemon", the daemon is not running.

### Socket Permission Issues

The socket at `/tmp/superctrl.sock` has permissions `0600` (owner only).

If you get permission errors, check:

```bash
ls -la /tmp/superctrl.sock
```

### Daemon Already Running

If you see "Daemon is already running" but want to restart:

```bash
rm /tmp/superctrl.sock
superctrl daemon
```

## Uninstallation

```bash
launchctl unload ~/Library/LaunchAgents/com.superctrl.daemon.plist
rm ~/Library/LaunchAgents/com.superctrl.daemon.plist
rm /tmp/superctrl.sock
sudo rm /usr/local/bin/superctrl
```

Restore macrowhisper config from backup:

```bash
cp ~/.config/macrowhisper/macrowhisper.json.backup.* ~/.config/macrowhisper/macrowhisper.json
```

## Development

### Running Locally

Terminal 1 (daemon):

```bash
OPENAI_API_KEY=your-key cargo run
```

Terminal 2 (send commands):

```bash
cargo run -- --execute "test command"
cargo run -- status
cargo run -- stop
```

### Testing IPC

```bash
echo '{"Execute":{"command":"test"}}' | nc -U /tmp/superctrl.sock
```

## Security

- Socket permissions are restricted to owner only (`0600`)
- API key is stored in environment variable, not in code
- No credentials are logged
- Commands are validated before execution

## Requirements

- macOS
- Rust toolchain
- Superwhisper with macrowhisper installed
- OpenAI API key with Computer Use API access
