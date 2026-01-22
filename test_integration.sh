#!/bin/bash

set -e

echo "🧪 Testing superctrl E2E integration..."
echo ""

echo "📦 Step 1: Building project..."
cargo build --release

echo ""
echo "🧪 Step 2: Running unit and integration tests..."
cargo test

echo ""
echo "🔑 Step 3: Checking environment..."
if [ -z "$ANTHROPIC_API_KEY" ]; then
	echo "⚠️  Warning: ANTHROPIC_API_KEY not set"
	echo "   Real API tests will be skipped"
	echo "   Set it with: export ANTHROPIC_API_KEY='your-key'"
else
	echo "✅ ANTHROPIC_API_KEY is set"
fi

echo ""
echo "🔌 Step 4: Testing IPC communication..."
if ./target/release/superctrl status 2>/dev/null; then
	echo "✅ Daemon is already running"
else
	echo "⚠️  Daemon is not running"
	echo "   To test IPC, start daemon in another terminal:"
	echo "   ./target/release/superctrl"
	echo ""
	echo "   Then run this script again to test command execution"
	exit 0
fi

echo ""
echo "📤 Step 5: Testing command execution via IPC..."
echo "Command: 'What can you see on the screen?'"
./target/release/superctrl -e "What can you see on the screen?"

echo ""
echo "✅ All tests completed successfully!"
echo ""
echo "To run real API test:"
echo "  cargo test --test e2e_daemon_test test_real_api_call -- --ignored --nocapture"
