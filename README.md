# superctrl

A macOS menu bar application that bridges Superwhisper/macrowhisper voice recordings with the OpenAI Computer Use API to enable voice-controlled macOS automation.

## Architecture

Built with Iced.rs, superctrl provides a native macOS menu bar interface for controlling your computer through natural language voice commands.

## Setup

### Prerequisites

- macOS
- Rust toolchain (install via [rustup](https://rustup.rs/))
- OpenAI API key with access to Computer Use API

### Environment Variables

Set the following environment variable before running:

```bash
export OPENAI_API_KEY="your-api-key-here"
```

Alternatively, create a `.env` file in the project root:

```
OPENAI_API_KEY=your-api-key-here
```

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run
```

## Development

Build in development mode:

```bash
cargo build
```

Run with logging:

```bash
RUST_LOG=debug cargo run
```

## License

All rights reserved.
