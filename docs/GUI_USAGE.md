# GUI Implementation - Usage Guide

## Overview

The superctrl menu bar GUI is implemented using Iced.rs for the preferences window and tray-icon for the macOS menu bar. The implementation is split across multiple modules:

- `src/gui.rs` - Core state management and data structures
- `src/menu_bar.rs` - Menu bar implementation using tray-icon
- `src/preferences.rs` - Preferences window using Iced
- `src/gui_integration.rs` - Helper functions for other modules to interact with the GUI

## Architecture

### State Management

The GUI uses a shared state pattern with `Arc<Mutex<GuiState>>`:

```rust
pub struct GuiState {
    pub app_state: AppState,           // Current status
    pub action_history: Vec<ActionRecord>, // Recent actions
    pub settings: Settings,             // App settings
    pub max_history: usize,            // Max actions to keep
    pub stop_flag: Arc<AtomicBool>,   // Emergency stop flag
}
```

### Application States

```rust
pub enum AppState {
    Idle,                   // Gray/white icon
    Working(String),        // Blue animated icon, task description
    Error(String),          // Red icon, error message
}
```

## Menu Bar

The menu bar displays:

1. **Status Line** - Shows current app state with emoji icon:
   - ⚪ Idle
   - 🔵 Working...
   - 🔴 Error

2. **Recent Actions** - Last 5 actions with timestamp, command, and description

3. **Stop Current Task** - Button (only enabled when Working)

4. **Preferences...** - Opens preferences window

5. **Quit** - Exits application

## Integration with Other Modules

### From Computer Use Module

Update status when starting a task:

```rust
use crate::gui_integration;

gui_integration::update_status(&state, AppState::Working("Processing voice command".to_string()));
```

Add an action record:

```rust
gui_integration::add_action(
    &state,
    "click".to_string(),
    "Clicked at (500, 300)".to_string()
);
```

Complete a task:

```rust
gui_integration::update_status(&state, AppState::Idle);
```

Handle errors:

```rust
gui_integration::update_status(&state, AppState::Error("API request failed".to_string()));
```

### Checking for Stop Signal

Computer Use module should periodically check:

```rust
if gui_integration::is_stopped(&state) {
    tracing::info!("Task stopped by user");
    gui_integration::reset_stop(&state);
    gui_integration::update_status(&state, AppState::Idle);
    return;
}
```

## Preferences Window

Opens in a separate thread when "Preferences..." is clicked. Shows:

1. **API Key Status** - ✓ if OPENAI_API_KEY is set, ✗ if not
2. **Emergency Stop Shortcut** - Read-only display (⌘⇧⎋)
3. **Test Connection** - Button that tests OpenAI API connectivity
4. **View Logs** - Opens ~/Library/Logs/superctrl in Finder

## Icon Colors

The menu bar icon changes color based on state:

- **Gray (128, 128, 128)** - Idle
- **Blue (0, 122, 255)** - Working (macOS system blue)
- **Red (255, 59, 48)** - Error (macOS system red)

## Thread Model

- **Main Thread** - Runs event loop
- **Menu Bar Thread** - Handles tray icon and menu updates (100ms update interval)
- **Preferences Thread** - Spawned when preferences window is opened
- **IPC Thread** - Handles IPC connections from CLI
- **Hotkey Thread** - Monitors emergency stop hotkey

## Action Records

Each action creates a record with:

```rust
pub struct ActionRecord {
    pub timestamp: DateTime<Local>,
    pub command: String,      // e.g., "click", "type", "screenshot"
    pub description: String,  // e.g., "Clicked at (500, 300)"
}
```

Format in menu: `HH:MM:SS | command | description`

## Emergency Stop

Three ways to trigger emergency stop:

1. **Keyboard Shortcut** - ⌘⇧⎋ (handled by hotkey module)
2. **Menu Bar** - "Stop Current Task" button
3. **IPC** - `superctrl stop` command

All three set the `stop_flag` which Computer Use module should check.

## Example: Full Integration Flow

```rust
// 1. Computer Use starts a task
gui_integration::update_status(&state, AppState::Working("Voice command: open browser".to_string()));

// 2. Execute actions
for action in actions {
    // Check stop flag
    if gui_integration::is_stopped(&state) {
        gui_integration::reset_stop(&state);
        gui_integration::update_status(&state, AppState::Idle);
        return Ok(());
    }

    // Execute action
    match action {
        Action::Click(x, y) => {
            gui_integration::add_action(
                &state,
                "click".to_string(),
                format!("Clicked at ({}, {})", x, y)
            );
            // ... actual click implementation
        }
        Action::Type(text) => {
            gui_integration::add_action(
                &state,
                "type".to_string(),
                format!("Typed '{}'", text)
            );
            // ... actual typing implementation
        }
    }
}

// 3. Complete successfully
gui_integration::update_status(&state, AppState::Idle);

// OR handle error
gui_integration::update_status(&state, AppState::Error(format!("Failed: {}", err)));
```

## Testing

To test the menu bar GUI:

```bash
# Set API key first
export OPENAI_API_KEY="your-key-here"

# Run the app
cargo run

# The menu bar icon should appear in the macOS menu bar
# Click it to see the menu
# Click "Preferences..." to test the preferences window
```

## Future Enhancements

Potential improvements for other agents:

1. Add animation to the "Working" icon (pulsing blue)
2. Add sound effects for actions
3. Add keyboard shortcuts for menu items
4. Add "Clear History" menu item
5. Add "About" window with version info
6. Add preferences for customizing behavior
7. Add status bar text when hovering over icon
