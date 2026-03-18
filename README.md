# evtr

`evtr` is a terminal UI for inspecting Linux evdev input devices. Select a
device from `/dev/input/event*`, then watch axes, relative motion, buttons,
hats, joysticks, and touch state update live.

## Requirements

- Linux with evdev support
- A terminal supported by `crossterm`
- Permission to read the selected `/dev/input/event*` node

If you see permission errors, grant your user read access to the relevant input devices.
The exact group or udev rule is distro-specific.

## Run

```sh
cargo run --release
```

`evtr` does not currently expose any command-line flags.

## Controls

### Selector

- Type to filter devices
- Up/Down, Ctrl-P/Ctrl-N, PageUp/PageDown, Home/End to move
- Enter to open the selected device
- Backspace or Ctrl-U to edit or clear the query
- Ctrl-R to refresh device discovery
- `?` to open help
- Esc or Ctrl-C to exit

### Monitor

- Up/Down or j/k to scroll
- PageUp/PageDown and g/G or Home/End to jump
- Shift-J and Shift-K to move focus between axes and buttons when both are visible
- r to reset relative axes
- i to show device info
- y to invert joystick Y rendering
- `?` to open help
- Esc to return to the selector when no popup is open
- Ctrl-C to exit the app

## Failure Modes

`evtr` will report actionable errors when:

- `/dev/input` cannot be read
- Event nodes exist but cannot be opened
- The selected device stream ends or returns an I/O error
- Terminal initialization or redraw fails

## Development

```sh
cargo fmt
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## License

MIT. See `LICENSE`.
