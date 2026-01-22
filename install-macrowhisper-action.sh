#!/bin/bash

set -e

CONFIG_FILE="$HOME/.config/macrowhisper/macrowhisper.json"

if [ ! -f "$CONFIG_FILE" ]; then
	echo "❌ Error: macrowhisper config not found at $CONFIG_FILE"
	echo "   Install macrowhisper first: https://github.com/ognistik/macrowhisper"
	exit 1
fi

if ! command -v jq &>/dev/null; then
	echo "❌ Error: jq not found. Install with: brew install jq"
	exit 1
fi

echo "🔧 Configuring macrowhisper integration..."

BACKUP_FILE="${CONFIG_FILE}.backup.$(date +%s)"
cp "$CONFIG_FILE" "$BACKUP_FILE"
echo "📦 Backup created: $BACKUP_FILE"

SUPERCTRL_PATH="/usr/local/bin/superctrl"
if [ ! -f "$SUPERCTRL_PATH" ]; then
	echo "⚠️  Warning: $SUPERCTRL_PATH not found. Using local build path."
	SUPERCTRL_PATH="$(pwd)/target/release/superctrl"
	if [ ! -f "$SUPERCTRL_PATH" ]; then
		echo "❌ Error: superctrl binary not found. Run ./install.sh first."
		exit 1
	fi
fi

jq --arg cmd "$SUPERCTRL_PATH -e '\$swResult'" \
	'.scriptsShell.superctrl = {
      "action": $cmd,
      "triggerVoice": "computer|automate|control|do this",
      "noNoti": true
   }' "$CONFIG_FILE" >"${CONFIG_FILE}.tmp"

mv "${CONFIG_FILE}.tmp" "$CONFIG_FILE"

echo "✅ macrowhisper configured successfully!"
echo ""

echo "Checking macrowhisper service status..."
if macrowhisper --service-status 2>&1 | grep -q "Running: Yes"; then
	echo "✅ macrowhisper service is running"
	echo ""
	echo "Voice triggers configured:"
	echo "  - 'Computer, [command]'"
	echo "  - 'Automate [command]'"
	echo "  - 'Control [command]'"
	echo "  - 'Do this: [command]'"
	echo ""
	echo "⚠️  Note: Restart macrowhisper service to apply changes:"
	echo "   macrowhisper --restart-service"
else
	echo "⚠️  Warning: macrowhisper service is not running!"
	echo ""
	echo "To start macrowhisper service:"
	echo "   macrowhisper --start-service"
	echo ""
	echo "Voice triggers configured (will be active after starting service):"
	echo "  - 'Computer, [command]'"
	echo "  - 'Automate [command]'"
	echo "  - 'Control [command]'"
	echo "  - 'Do this: [command]'"
fi

echo ""
echo "To restore previous config:"
echo "  cp $BACKUP_FILE $CONFIG_FILE"
