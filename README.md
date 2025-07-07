# Claude Launcher

A Rust CLI tool that launches multiple Claude AI instances in separate iTerm tabs to work on tasks in parallel. Perfect for breaking down complex projects into smaller tasks that can be executed simultaneously.

## What it does

`claude-launcher` opens new iTerm tabs, each running Claude with a specific task. Each Claude instance:

1. Changes to the directory where you ran the command
2. Reads the `todos.md` file in that directory
3. Executes the assigned task
4. Updates `todos.md` to mark the task as complete
5. Stops after completing the task

The exact command run in each tab:
```bash
cd "/your/current/directory" && claude --dangerously-skip-permissions "look in todos.md, [your task], ONCE YOUR DONE, update todos.md to mark you task as done, thank you for your service, YOU STOP HERE"
```

## Prerequisites

- macOS (uses AppleScript to control iTerm)
- [iTerm2](https://iterm2.com/) installed
- [Claude CLI](https://claude.ai/code) installed and configured
- Rust (for building from source)

## Installation

### From Source

1. Clone this repository:
   ```bash
   git clone https://github.com/CharlonTank/claude-launcher.git
   cd claude-launcher
   ```

2. Run the install script (requires sudo to install to `/usr/local/bin`):
   ```bash
   ./install.sh
   ```

### Manual Build

```bash
cargo build --release
sudo cp target/release/claude-launcher /usr/local/bin/
```

## Usage

```bash
claude-launcher "task1" ["task2" ...] 
```

### Examples

Launch a single Claude instance:
```bash
claude-launcher "implement user authentication"
```

Launch multiple Claude instances for parallel tasks:
```bash
claude-launcher "create login form" "implement JWT tokens" "add password hashing" "write auth tests"
```

### Best Practices

1. **Use with todos.md**: Create a `todos.md` file in your project with a list of tasks
2. **Be specific**: Provide clear, actionable task descriptions
3. **Limit scope**: Each task should be completable independently
4. **Max 10 tasks**: The tool limits you to 10 simultaneous tasks to prevent system overload

## How it Works

1. Captures the current working directory
2. For each task provided as an argument:
   - Generates an AppleScript command
   - Opens a new iTerm tab
   - Changes to the original directory
   - Runs Claude with the specific task instruction
3. Each Claude instance works independently and updates `todos.md` when complete

## Limitations

- macOS only (uses AppleScript)
- Requires iTerm2 (not Terminal.app)
- Maximum of 10 concurrent tasks
- Requires `todos.md` file in the working directory for best results

## Contributing

Pull requests are welcome! Please feel free to submit issues or enhancement requests.

## License

This project is open source and available under the [MIT License](LICENSE).