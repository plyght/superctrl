# OpenAI Computer Use API Integration

This document describes the Computer Use API integration for superctrl.

## Architecture

The integration consists of three main modules:

### 1. `screenshot.rs` - Screen Capture

**Purpose**: Captures screenshots from the primary macOS display and encodes them as base64 PNG data.

**Key Components**:

- `ScreenCapture` struct: Manages screenshot capture with configurable display size
- Uses `xcap` crate for native macOS screen capture
- Automatically resizes screenshots to target resolution
- Returns base64-encoded PNG data ready for API transmission

**Usage**:

```rust
use superctrl::ScreenCapture;

let capture = ScreenCapture::new(1920, 1080);
let screenshot_base64 = capture.capture_screenshot()?;
```

### 2. `automation.rs` - macOS Automation

**Purpose**: Executes computer actions on macOS using the `enigo` crate.

**Key Components**:

- `MacAutomation` struct: Handles all automation operations
- `Action` enum: Defines all supported actions
  - `Click { x, y, button }`: Mouse clicks (left/right/middle)
  - `Type { text }`: Text input
  - `Keypress { keys }`: Keyboard key presses
  - `Scroll { x, y, scroll_x, scroll_y }`: Mouse scrolling
  - `Wait { duration_ms }`: Delays

**Supported Keys**:

- Modifiers: shift, control/ctrl, alt/option, meta/command/cmd
- Navigation: up/down/left/right arrows, home, end, pageup, pagedown
- Function keys: f1-f12
- Special: return/enter, tab, space, backspace, delete, escape/esc
- Single characters: Any unicode character

**Usage**:

```rust
use superctrl::{MacAutomation, Action, MouseButton};

let mut automation = MacAutomation::new()?;

automation.execute_action(Action::Click {
    x: 100,
    y: 200,
    button: MouseButton::Left,
})?;

automation.execute_action(Action::Type {
    text: "Hello, World!".to_string(),
})?;
```

### 3. `computer_use.rs` - Main Agent Loop

**Purpose**: Orchestrates the OpenAI Computer Use API loop.

**Key Components**:

- `ComputerUseAgent` struct: Main agent managing the API interaction
- Uses `async-openai` 0.24+ with the Responses API
- Implements the computer use loop pattern

**Features**:

- Async/await API using tokio
- Emergency stop via `Arc<AtomicBool>` flag
- Configurable display size
- Full trust mode (auto-acknowledges safety checks)
- Maximum iteration limit (default: 50)
- Comprehensive error handling

**Usage**:

```rust
use superctrl::ComputerUseAgent;
use std::sync::{Arc, atomic::AtomicBool};

let api_key = std::env::var("OPENAI_API_KEY")?;
let stop_flag = Arc::new(AtomicBool::new(false));

let mut agent = ComputerUseAgent::new(api_key, stop_flag)?
    .with_display_size(1920, 1080)
    .with_full_trust_mode(true);

let response = agent.execute_command("Open Safari and go to github.com").await?;
println!("Result: {}", response);
```

## API Flow

1. **Initialize**: Create agent with API key and stop flag
2. **Capture**: Take initial screenshot of desktop
3. **Request**: Send command + screenshot to OpenAI API with computer_use_preview tool
4. **Parse**: Extract tool calls from API response
5. **Execute**: Perform requested actions (click, type, etc.)
6. **Capture**: Take new screenshot after action
7. **Report**: Send action result + new screenshot back to API
8. **Repeat**: Continue loop until task complete or max iterations reached

## Configuration

### Display Size

```rust
agent = agent.with_display_size(1920, 1080);
```

Screenshots are resized to this resolution before sending to API.

### Full Trust Mode

```rust
agent = agent.with_full_trust_mode(true);
```

When enabled, automatically acknowledges all safety checks. Disable for manual approval.

### Emergency Stop

```rust
let stop_flag = Arc::new(AtomicBool::new(false));

// In another thread:
stop_flag.store(true, Ordering::Relaxed);
```

Immediately halts execution at next iteration.

## Model

Currently uses: `gpt-4o` (standard GPT-4 with vision)

Note: As of implementation, OpenAI has not released a dedicated "gpt-4o-computer-use-preview" model. This integration uses the standard GPT-4o model with tool calling capabilities. Update the `MODEL` constant in `computer_use.rs` when the official computer use model becomes available.

## Error Handling

All functions return `anyhow::Result<T>` for comprehensive error propagation:

- Screenshot capture failures
- Automation execution errors
- API communication errors
- Parsing/serialization errors

## Safety Considerations

1. **Full Trust Mode**: Only enable in trusted environments
2. **Emergency Stop**: Always provide a stop mechanism for long-running tasks
3. **Iteration Limit**: Prevents infinite loops (default: 50 iterations)
4. **Action Validation**: All coordinates and parameters are validated before execution

## Testing

Run the integration tests:

```bash
cargo test --test computer_use_integration
```

Run the example:

```bash
OPENAI_API_KEY=your-key cargo run --example computer_use_example
```

## Dependencies

- `async-openai` 0.24+: OpenAI API client
- `tokio`: Async runtime
- `xcap`: Screen capture on macOS
- `enigo`: Cross-platform automation
- `image`: Image processing
- `base64`: Encoding
- `serde/serde_json`: Serialization
- `anyhow`: Error handling

## Future Enhancements

Potential improvements:

- Multi-monitor support
- Action recording/replay
- Safety guardrails configuration
- Performance metrics
- Action history/undo
- Screenshot caching
- Adaptive display sizing
