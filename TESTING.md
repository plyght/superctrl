# Testing superctrl Integration (Without Voice)

There are **3 ways** to test the integration without using voice commands:

## Method 1: Direct CLI Command (Simulates macrowhisper)

This is exactly what macrowhisper does internally when it detects a voice trigger.

### Step 1: Start the superctrl daemon

```bash
# Option A: Run in foreground (you'll see menu bar icon)
cd ~/superctrl
./target/release/superctrl

# Option B: Run in background
./target/release/superctrl &
```

### Step 2: Send a test command

In another terminal:

```bash
cd ~/superctrl

# Test with a simple command
./target/release/superctrl -e "Open Safari"

# Or test with other commands
./target/release/superctrl -e "Take a screenshot"
./target/release/superctrl -e "Open Finder"
./target/release/superctrl -e "Open Notes app"
```

The command will be sent to the daemon via IPC, and you'll see the automation execute.

---

## Method 2: Use macrowhisper's exec-action Command

You can manually trigger the superctrl action using macrowhisper's CLI:

```bash
# Execute the superctrl action with a test command
macrowhisper --exec-action superctrl

# Note: This uses the last Superwhisper transcription result
# To test with a specific command, you'd need to have a recent transcription
```

---

## Method 3: Direct API Test (Bypasses IPC)

For testing the Anthropic API integration directly without the daemon:

```bash
cd ~/superctrl

# Make sure ANTHROPIC_API_KEY is set
export ANTHROPIC_API_KEY='your-key-here'

# Run the example program
cargo run --example computer_use_example
```

This will directly call the `ComputerUseAgent` and execute a test command.

---

## Quick Test Script

Use the provided test script:

```bash
cd ~/superctrl
./test_integration.sh
```

This script:
1. Checks if daemon is running
2. Sends a test command
3. Shows results

---

## Troubleshooting

### "Failed to connect to daemon"
- Make sure superctrl daemon is running: `./target/release/superctrl status`
- Start it: `./target/release/superctrl`

### "ANTHROPIC_API_KEY not set"
- Export the key: `export ANTHROPIC_API_KEY='your-key'`
- Or add to `~/.zshrc` for persistence

### "macrowhisper service not running"
- Start it: `macrowhisper --start-service`
- Check status: `macrowhisper --service-status`

---

## Example Test Commands

```bash
# Simple actions
./target/release/superctrl -e "Open Safari"
./target/release/superctrl -e "Open Finder"
./target/release/superctrl -e "Open Notes"

# More complex
./target/release/superctrl -e "Open Safari and navigate to github.com"
./target/release/superctrl -e "Take a screenshot and save it to Desktop"
./target/release/superctrl -e "Open Terminal and type 'ls'"
```
