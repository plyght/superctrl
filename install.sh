#!/bin/bash

set -e

echo "🚀 Installing superctrl..."

if ! command -v cargo &>/dev/null; then
	echo "❌ Error: Rust/Cargo not found. Install from https://rustup.rs"
	exit 1
fi

echo "📦 Building release binary..."
cargo build --release

echo "📝 Installing to /usr/local/bin..."
sudo cp target/release/superctrl /usr/local/bin/

echo "✅ Binary installed successfully!"
echo ""
echo "Next steps:"
echo "1. Set your Anthropic API key:"
echo "   export ANTHROPIC_API_KEY='your-key-here'"
echo "   # Add to ~/.zshrc or ~/.bashrc to persist"
echo ""
echo "2. Configure macrowhisper integration:"
echo "   ./install-macrowhisper-action.sh"
echo ""
echo "3. Ensure macrowhisper service is running:"
echo "   macrowhisper --start-service"
echo "   # Check status: macrowhisper --service-status"
echo ""
echo "4. (Optional) Set up launch agent for superctrl:"
echo "   cp superctrl.plist ~/Library/LaunchAgents/com.superctrl.daemon.plist"
echo "   # Edit plist to add your ANTHROPIC_API_KEY"
echo "   nano ~/Library/LaunchAgents/com.superctrl.daemon.plist"
echo "   launchctl load ~/Library/LaunchAgents/com.superctrl.daemon.plist"
echo ""
echo "5. Start superctrl daemon manually (if not using launch agent):"
echo "   superctrl"
