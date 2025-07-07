# Claude Launcher

A Rust-based command-line tool that opens new iTerm tabs and runs Claude with specified tasks.

## Installation

1. Make sure you have Rust installed. If not, install it from https://rustup.rs/

2. Clone this repository and navigate to it:
   ```bash
   cd /path/to/claude-launcher
   ```

3. Run the install script:
   ```bash
   ./install.sh
   ```

## Usage

```bash
claude-launcher "task1" ["task2" ...] 
```

### Examples

Single task:
```bash
claude-launcher "implement a new feature"
```

Multiple tasks (opens multiple iTerm tabs):
```bash
claude-launcher "fix the login bug" "add unit tests" "update documentation"
```

### What it does

- Opens a new iTerm tab for each task
- Runs `claude --dangerously-skip-permissions "look in todos.md, [your task]"` in each tab
- Supports up to 10 tasks at once

## Building from Source

```bash
cargo build --release
```

The binary will be available at `target/release/claude-launcher`