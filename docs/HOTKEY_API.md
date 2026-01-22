# Emergency Stop Hotkey API Reference

## Module: `hotkey`

### `EmergencyStop` Struct

Thread-safe emergency stop system using global keyboard shortcuts.

```rust
pub struct EmergencyStop {
    stop_flag: Arc<AtomicBool>,
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
}
```

### Methods

#### `new() -> Result<Self>`

Creates a new emergency stop instance.

**Returns:**

- `Ok(EmergencyStop)` on success
- `Err(anyhow::Error)` if initialization fails (e.g., permission denied)

**Example:**

```rust
use superctrl::hotkey::EmergencyStop;

match EmergencyStop::new() {
    Ok(es) => println!("Emergency stop initialized"),
    Err(e) => eprintln!("Failed to initialize: {}", e),
}
```

**Notes:**

- Requires Accessibility permissions on macOS
- Automatically configures Command+Shift+Escape as the hotkey

---

#### `register_hotkey(&self) -> Result<()>`

Registers the global keyboard shortcut with the operating system.

**Returns:**

- `Ok(())` on success
- `Err(anyhow::Error)` if registration fails

**Example:**

```rust
let es = EmergencyStop::new()?;
es.register_hotkey()?;
println!("✓ Emergency stop hotkey registered: ⌘⇧⎋");
```

**Notes:**

- Must be called after `new()` to activate the hotkey
- Logs success message to stderr when registered

---

#### `unregister_hotkey(&self) -> Result<()>`

Unregisters the global keyboard shortcut.

**Returns:**

- `Ok(())` on success
- `Err(anyhow::Error)` if unregistration fails

**Example:**

```rust
es.unregister_hotkey()?;
```

**Notes:**

- Automatically called when EmergencyStop is dropped
- Safe to call multiple times

---

#### `is_stopped(&self) -> bool`

Checks if the emergency stop has been triggered.

**Returns:**

- `true` if stop was triggered
- `false` otherwise

**Example:**

```rust
if es.is_stopped() {
    println!("Emergency stop is active");
    break;
}
```

**Thread Safety:**

- Uses `Ordering::Acquire` for memory ordering
- Safe to call from any thread

---

#### `reset(&self)`

Resets the stop flag to allow new operations.

**Example:**

```rust
es.reset();
println!("Emergency stop flag reset");
```

**Thread Safety:**

- Uses `Ordering::Release` for memory ordering
- Safe to call from any thread

**Notes:**

- Logs reset message to stderr

---

#### `get_stop_flag(&self) -> Arc<AtomicBool>`

Returns a cloned reference to the stop flag for integration with other components.

**Returns:**

- `Arc<AtomicBool>` - Thread-safe stop flag

**Example:**

```rust
let stop_flag = es.get_stop_flag();

// Use in another thread
std::thread::spawn(move || {
    loop {
        if stop_flag.load(Ordering::Acquire) {
            println!("Stopping...");
            break;
        }
        // Do work...
    }
});
```

**Thread Safety:**

- Returns an Arc clone, so multiple threads can hold references
- Use `Ordering::Acquire` for reads, `Ordering::Release` for writes

---

#### `start_listener(stop_flag: Arc<AtomicBool>)` (Static)

Starts a background listener thread that monitors for hotkey events.

**Parameters:**

- `stop_flag` - The atomic boolean to set when hotkey is pressed

**Example:**

```rust
let es = EmergencyStop::new()?;
let stop_flag = es.get_stop_flag();
EmergencyStop::start_listener(stop_flag);
```

**Notes:**

- Spawns a tokio async task
- Polls for events every 10ms
- Sets stop_flag when Command+Shift+Escape is pressed
- Logs "🛑 EMERGENCY STOP ACTIVATED (⌘⇧⎋)" when triggered

---

## Integration Examples

### Basic Usage

```rust
use superctrl::hotkey::EmergencyStop;
use std::sync::atomic::Ordering;

fn main() -> anyhow::Result<()> {
    // Initialize
    let es = EmergencyStop::new()?;
    es.register_hotkey()?;

    // Start listener
    let stop_flag = es.get_stop_flag();
    EmergencyStop::start_listener(stop_flag.clone());

    // Main work loop
    loop {
        if stop_flag.load(Ordering::Acquire) {
            println!("Stopped by user");
            es.reset();
            break;
        }

        // Do work...
    }

    Ok(())
}
```

### With Computer Use Agent

```rust
use superctrl::computer_use::ComputerUseAgent;
use superctrl::hotkey::EmergencyStop;

async fn run_automation() -> anyhow::Result<()> {
    // Initialize emergency stop
    let es = EmergencyStop::new()?;
    es.register_hotkey()?;
    let stop_flag = es.get_stop_flag();
    EmergencyStop::start_listener(stop_flag.clone());

    // Create agent with stop flag
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let mut agent = ComputerUseAgent::new(api_key, stop_flag)?;

    // Execute command (will respect emergency stop)
    let result = agent.execute_command("Open Chrome and search for Rust").await?;
    println!("Result: {}", result);

    Ok(())
}
```

### With GUI State

```rust
use superctrl::gui::{GuiState, create_shared_state};
use superctrl::hotkey::EmergencyStop;

fn main() -> anyhow::Result<()> {
    let state = create_shared_state();

    // Initialize emergency stop
    let es = EmergencyStop::new()?;
    es.register_hotkey()?;

    // Use GUI state's stop flag
    let stop_flag = {
        let gui_state = state.lock().unwrap();
        gui_state.get_stop_flag()
    };

    EmergencyStop::start_listener(stop_flag.clone());

    // Monitor stop flag and update GUI
    std::thread::spawn(move || {
        loop {
            if stop_flag.load(std::sync::atomic::Ordering::Acquire) {
                let mut gui_state = state.lock().unwrap();
                gui_state.update_status(superctrl::gui::AppState::Idle);
                stop_flag.store(false, std::sync::atomic::Ordering::Release);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });

    Ok(())
}
```

## Memory Ordering

The emergency stop system uses proper atomic memory ordering:

- **Acquire** - Used when reading the stop flag

  ```rust
  if stop_flag.load(Ordering::Acquire) { ... }
  ```

- **Release** - Used when setting the stop flag

  ```rust
  stop_flag.store(true, Ordering::Release);
  ```

- **Relaxed** - Used in performance-critical sections (Computer Use loop)
  ```rust
  if self.stop_flag.load(Ordering::Relaxed) { ... }
  ```

This ensures proper synchronization between threads without unnecessary overhead.

## Error Handling

### Common Errors

1. **Accessibility Permission Denied**

   ```
   Error: Failed to create GlobalHotKeyManager. On macOS, this requires
   Accessibility permissions. Go to System Settings > Privacy & Security >
   Accessibility and add superctrl to the allowed apps.
   ```

   **Solution:** Grant Accessibility permissions in System Settings

2. **Hotkey Already Registered**

   ```
   Error: Failed to register global hotkey (⌘⇧⎋)
   ```

   **Solution:** Another app may be using the same shortcut

### Graceful Degradation

The application handles permission errors gracefully:

```rust
let emergency_stop = match EmergencyStop::new() {
    Ok(es) => {
        if let Err(e) = es.register_hotkey() {
            eprintln!("Warning: {}", e);
            None
        } else {
            Some(es)
        }
    }
    Err(e) => {
        eprintln!("Warning: {}", e);
        None
    }
};

// App continues without emergency stop if initialization fails
```

## Platform Support

- **macOS**: Full support with Accessibility permissions
- **Linux**: Supported (via global-hotkey crate)
- **Windows**: Supported (via global-hotkey crate)

## Dependencies

- `global-hotkey = "0.6"` - Cross-platform global hotkey handling
- `tokio` - Async runtime for listener
- `anyhow` - Error handling

## Troubleshooting

### Hotkey Not Working

1. **Check permissions**: Ensure Accessibility permissions are granted
2. **Check registration**: Look for "✓ Emergency stop hotkey registered" log message
3. **Check conflicts**: Verify no other app is using Command+Shift+Escape
4. **Check listener**: Ensure `start_listener()` was called

### Stop Flag Not Resetting

- Call `es.reset()` after handling the stop
- Or manually reset: `stop_flag.store(false, Ordering::Release)`

### Multiple Stops Needed

- Ensure stop flag is reset between operations
- Check that `reset()` is being called after each stop
