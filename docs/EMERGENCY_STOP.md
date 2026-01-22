# Emergency Stop System Implementation

## Overview

The emergency stop system allows users to immediately halt any ongoing Computer Use API operations using a global keyboard shortcut or menu bar action.

## Keyboard Shortcut

**⌘⇧⎋** (Command + Shift + Escape)

This shortcut works system-wide, even when superctrl is not in focus.

## Architecture

### Core Components

#### 1. `src/hotkey.rs` - Global Hotkey Handler

- **EmergencyStop struct**: Manages the emergency stop system
- Uses `global-hotkey` crate version 0.6
- Registers Command+Shift+Escape as the global shortcut
- Thread-safe flag using `Arc<AtomicBool>`

**Key Methods:**

- `new()` - Initialize the emergency stop system
- `register_hotkey()` - Register the global keyboard shortcut
- `is_stopped()` - Check if stop was triggered
- `reset()` - Clear the stop flag
- `get_stop_flag()` - Get a clone of the stop flag for integration
- `start_listener()` - Start the background listener thread

#### 2. `src/gui.rs` - GUI State Integration

- `GuiState` includes `stop_flag: Arc<AtomicBool>`
- Methods added:
  - `trigger_stop()` - Set the stop flag to true
  - `reset_stop()` - Clear the stop flag
  - `get_stop_flag()` - Get a reference to the stop flag

#### 3. `src/computer_use.rs` - Computer Use Integration

- Already includes stop flag checking on line 119-121
- Before each AI iteration, checks if stop flag is set
- If stopped, returns error and halts execution

### Integration Points

#### Main Application (`src/main.rs`)

1. Initializes `EmergencyStop` on startup
2. Registers the global hotkey
3. Starts listener thread that monitors the stop flag
4. When triggered, updates GUI state to Idle

#### Menu Bar (`src/menu_bar.rs`)

1. "Stop Current Task" menu item
2. Enabled only when a task is running (AppState::Working)
3. Clicking triggers the stop flag via `gui_state.trigger_stop()`

#### IPC Server (`src/ipc.rs` via `main.rs`)

1. Handles stop commands from CLI
2. Triggers the stop flag when IPC stop command received
3. Updates GUI state to Idle

## Usage

### For Users

#### Via Keyboard Shortcut

Press **⌘⇧⎋** at any time to stop the current task.

#### Via Menu Bar

1. Click on the superctrl menu bar icon
2. Click "Stop Current Task" (only enabled when a task is running)

#### Via CLI

```bash
superctrl stop
```

### For Developers

#### Integrating with Computer Use

The Computer Use agent already checks the stop flag before each iteration:

```rust
if self.stop_flag.load(Ordering::Relaxed) {
    anyhow::bail!("Execution stopped by user");
}
```

#### Accessing the Stop Flag

From the GUI state:

```rust
let gui_state = state.lock().unwrap();
let stop_flag = gui_state.get_stop_flag();

// Check if stopped
if stop_flag.load(Ordering::Acquire) {
    // Handle stop
}
```

#### Creating a ComputerUseAgent with Stop Flag

```rust
let api_key = std::env::var("OPENAI_API_KEY")?;
let stop_flag = gui_state.get_stop_flag();
let agent = ComputerUseAgent::new(api_key, stop_flag)?;
```

## macOS Accessibility Permissions

### Required Permissions

Global keyboard shortcuts require Accessibility permissions on macOS.

### Granting Permissions

1. Open **System Settings**
2. Go to **Privacy & Security**
3. Click **Accessibility**
4. Add **superctrl** to the allowed apps list

### Permission Denied Behavior

If permissions are not granted:

- The app will still run normally
- A warning message is logged on startup
- Emergency stop will not be available via keyboard shortcut
- Menu bar stop and CLI stop will still work

### Permission Check

The system automatically detects permission issues and logs warnings:

```
Warning: Failed to register emergency stop hotkey: ...
  The app will still work, but emergency stop (⌘⇧⎋) won't be available.
```

## Thread Safety

The emergency stop system is fully thread-safe:

- Uses `Arc<AtomicBool>` for the stop flag
- Atomic operations with proper memory ordering (Acquire/Release)
- Can be safely accessed from multiple threads:
  - Main thread (GUI updates)
  - Background listener thread (hotkey monitoring)
  - Computer Use thread (action execution)
  - IPC handler thread (CLI commands)

## Event Flow

### Keyboard Shortcut Triggered

1. User presses ⌘⇧⎋
2. global-hotkey library detects the keypress
3. Listener thread in `hotkey.rs` receives event
4. Stop flag set to `true` using `Ordering::Release`
5. Main thread monitor detects flag change
6. GUI state updated to Idle
7. Stop flag reset to `false`
8. Computer Use loop breaks on next iteration

### Menu Bar Stop Triggered

1. User clicks "Stop Current Task" in menu bar
2. Menu bar handler calls `gui_state.trigger_stop()`
3. Stop flag set to `true`
4. GUI state updated to Idle
5. Computer Use loop breaks on next iteration

### IPC Stop Triggered

1. User runs `superctrl stop` command
2. IPC server receives stop command
3. IPC handler calls `gui_state.trigger_stop()`
4. Stop flag set to `true`
5. GUI state updated to Idle
6. Computer Use loop breaks on next iteration

## Error Handling

### Hotkey Registration Failure

- Gracefully degrades: app continues without emergency stop
- Logs warning with instructions for granting permissions
- Other stop mechanisms (menu bar, IPC) still available

### Permission Denied

- Detected during hotkey registration
- Clear error message with instructions
- App remains fully functional except for keyboard shortcut

### Stop During Non-Working State

- Stop flag can be set anytime
- No-op if no task is running
- Flag automatically reset after handling

## Testing

### Manual Testing

1. Start superctrl
2. Initiate a Computer Use task
3. Press ⌘⇧⎋ while task is running
4. Verify task stops immediately
5. Check logs for "🛑 EMERGENCY STOP ACTIVATED" message

### Testing Without Accessibility Permissions

1. Remove superctrl from Accessibility list
2. Start superctrl
3. Verify warning message in logs
4. Test menu bar stop still works
5. Test CLI stop still works

## Dependencies

- `global-hotkey = "0.6"` - Global keyboard shortcut handling
- `tokio` - Async runtime for IPC server
- Standard library atomic operations

## Future Enhancements

Potential improvements:

- Configurable keyboard shortcut
- Visual feedback (overlay notification when stopped)
- Stop action undo capability
- Graceful task cleanup before stopping
- Stop reason logging (keyboard vs menu vs IPC)
