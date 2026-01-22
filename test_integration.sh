#!/bin/bash
# Test script for superctrl integration

echo "🧪 Testing superctrl integration..."
echo ""

# Check if daemon is running
if ./target/release/superctrl status 2>/dev/null; then
    echo "✅ Daemon is running"
else
    echo "⚠️  Daemon is not running. Starting it..."
    echo "   Run: ./target/release/superctrl"
    echo "   Or in background: ./target/release/superctrl &"
    exit 1
fi

echo ""
echo "Testing command execution..."
echo "Command: 'Open Safari'"
./target/release/superctrl -e "Open Safari"

echo ""
echo "✅ Test command sent. Check the daemon for results."
