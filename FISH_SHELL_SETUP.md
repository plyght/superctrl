# Fish Shell Setup for superctrl

Fish shell uses different syntax than bash/zsh for environment variables.

## Setting ANTHROPIC_API_KEY

### Temporary (current session only):
```fish
set -x ANTHROPIC_API_KEY 'your-key-here'
```

### Permanent (persists across sessions):
Add to `~/.config/fish/config.fish`:
```fish
set -x ANTHROPIC_API_KEY 'your-key-here'
```

### Verify it's set:
```fish
echo $ANTHROPIC_API_KEY
```

## Running superctrl

After setting the API key, run:
```fish
cd ~/superctrl
./target/release/superctrl
```

Or if you've installed it:
```fish
superctrl
```

## Testing

```fish
# In one terminal - start daemon
./target/release/superctrl

# In another terminal - test command
./target/release/superctrl -e "Open Safari"
```
