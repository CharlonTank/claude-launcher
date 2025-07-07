# Claude Launcher

A Rust CLI tool that launches multiple Claude AI instances in separate iTerm tabs to work on tasks in parallel. Perfect for breaking down complex projects into smaller tasks that can be executed simultaneously.

## What it does

`claude-launcher` opens new iTerm tabs, each running Claude with a specific task. Each Claude instance:

1. Changes to the directory where you ran the command
2. Reads the `todos.md` file in that directory
3. Executes the assigned task
4. Updates `todos.md` to mark the task as complete
5. Stops after completing the task

### Recursive Mode

When `--recursive` is enabled:
- Phase CTOs are instructed to spawn new agents for the next phase using `claude-launcher --recursive`
- The Final CTO can create additional phases if needed
- This allows for continuous, autonomous project execution

Example workflow:
1. Launch Phase 1: `claude-launcher --recursive "Phase 1: Task A" "Phase 1: Task B"`
2. Phase 1 CTO completes and spawns Phase 2: `claude-launcher --recursive "Phase 2: Task C" "Phase 2: Task D"`
3. Process continues until all phases are complete

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
# Initialize a new project
claude-launcher --init

# Generate task phases from requirements
claude-launcher --create-task "your project requirements"

# Launch agents to work on tasks
claude-launcher [--recursive] "task1" ["task2" ...] 
```

### Commands

- `--init`: Creates a new todos.md file with the parallel agent planning template
- `--create-task "requirements"`: Analyzes your requirements and generates detailed task phases
- `--recursive`: Enable recursive mode where Phase CTOs can spawn new agents for subsequent phases

### Workflow

1. **Initialize your project**:
   ```bash
   claude-launcher --init
   ```
   This creates a todos.md template file.

2. **Generate task phases**:
   ```bash
   claude-launcher --create-task "I want to build a REST API with user authentication and todo management"
   ```
   Claude will analyze your requirements and create detailed phases with parallel tasks.

3. **Launch agents**:
   ```bash
   # Launch specific tasks
   claude-launcher "Phase 1, Step 1A: Create database schema" "Phase 1, Step 1B: Setup Express server"
   
   # Or use recursive mode for autonomous execution
   claude-launcher --recursive "Phase 1, Step 1A: Create database schema" "Phase 1, Step 1B: Setup Express server"
   ```

### Examples

Initialize and plan a new web app:
```bash
claude-launcher --init
claude-launcher --create-task "Build a React app with TypeScript, user auth, and real-time chat"
```

Launch agents for parallel execution:
```bash
claude-launcher "Phase 1, Step 1A: Setup React with TypeScript" "Phase 1, Step 1B: Create auth components"
```

Launch with recursive mode (CTOs spawn next phases automatically):
```bash
claude-launcher --recursive "Phase 1, Step 1A: Design API" "Phase 1, Step 1B: Create database schema"
```

### Best Practices

1. **Use with todos.md**: Create a `todos.md` file in your project with a list of tasks
2. **Be specific**: Provide clear, actionable task descriptions
3. **Limit scope**: Each task should be completable independently
4. **Max 10 tasks**: The tool limits you to 10 simultaneous tasks to prevent system overload

## How it Works

1. Captures the current working directory
2. For each task provided as an argument:
   - Creates a temporary prompt file (`agent_prompt_task_N.txt`) with the full instructions
   - Generates an AppleScript command
   - Opens a new iTerm tab
   - Changes to the original directory
   - Runs `claude < agent_prompt_task_N.txt` to avoid command line escaping issues
   - Automatically removes the prompt file after execution
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