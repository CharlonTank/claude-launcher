# Claude Launcher

A Rust CLI tool that launches multiple Claude AI instances in separate iTerm tabs to work on tasks in parallel. Perfect for breaking down complex projects into smaller tasks that can be executed simultaneously.

## What it does

`claude-launcher` opens new iTerm tabs, each running Claude with a specific task. Each Claude instance:

1. Changes to the directory where you ran the command
2. Reads the `todos.json` file in that directory
3. Executes the assigned task
4. Updates `todos.json` to mark the task as complete with a comment
5. Stops after completing the task

### Automatic Task Detection

When run without arguments, `claude-launcher` automatically:
- Reads `todos.json` to find the next phase with TODO status
- Launches all TODO tasks in that phase in parallel
- Phase CTOs automatically spawn the next phase when complete

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

# Auto-launch next available tasks
claude-launcher

# Or manually launch specific tasks
claude-launcher "Phase 1, Step 1A: Task name" "Phase 1, Step 1B: Another task"
```

### Commands

- `--init`: Creates a new todos.json file
- `--create-task "requirements"`: Analyzes your requirements and generates detailed task phases
- No arguments: Automatically detects and launches the next TODO phase

### Workflow

1. **Initialize your project**:
   ```bash
   claude-launcher --init
   ```
   This creates an empty todos.json file.

2. **Generate task phases**:
   ```bash
   claude-launcher --create-task "I want to build a REST API with user authentication and todo management"
   ```
   Claude will analyze your requirements and create detailed phases with parallel tasks.

3. **Launch agents**:
   ```bash
   # Automatic mode - launches next TODO phase
   claude-launcher
   
   # Or manually launch specific tasks
   claude-launcher "Phase 1, Step 1A: Create database schema" "Phase 1, Step 1B: Setup Express server"
   ```

### JSON Structure

The `todos.json` file contains phases and steps:

```json
{
  "phases": [
    {
      "id": 1,
      "name": "Core Setup",
      "steps": [
        {
          "id": "1A",
          "name": "Create database schema",
          "prompt": "Create a PostgreSQL schema with users and todos tables...",
          "status": "TODO",
          "comment": ""
        }
      ],
      "status": "TODO",
      "comment": ""
    }
  ]
}
```

### Phase CTO Behavior

When all tasks in a phase are complete but the phase status is still TODO, running `claude-launcher` will:
1. Detect that all steps are DONE
2. Spawn a dedicated Phase CTO agent
3. The Phase CTO will:
   - Review all completed tasks in the phase
   - Run validation commands (`lamdera make` and `elm-test-rs`)
   - Based on results:
     - **No errors**: Mark phase DONE and proceed to next phase
     - **Few errors (1-5)**: Fix them, mark phase DONE, proceed
     - **Many errors (6+)**: Create a remediation phase with fix tasks
   - Add comprehensive phase summary with test results
   - Call `claude-launcher` to start the next phase

This ensures code quality and proper phase review before proceeding.

### Best Practices

1. **Task Independence**: Ensure tasks in the same phase don't modify the same files
2. **Clear Prompts**: Provide detailed instructions in each task prompt
3. **Limit Scope**: Each task should be completable independently
4. **Max 10 tasks**: The tool limits you to 10 simultaneous tasks to prevent system overload

## How it Works

1. Captures the current working directory
2. For each task:
   - Creates a temporary prompt file with instructions
   - Generates an AppleScript command
   - Opens a new iTerm tab
   - Runs Claude with the prompt
   - Automatically removes the prompt file after execution
3. Each Claude instance works independently and updates `todos.json` when complete

## Example

Initialize and plan a new web app:
```bash
claude-launcher --init
claude-launcher --create-task "Build a React app with TypeScript, user auth, and real-time chat"
claude-launcher  # Auto-launches Phase 1 tasks
```

## Limitations

- macOS only (uses AppleScript)
- Requires iTerm2 (not Terminal.app)
- Maximum of 10 concurrent tasks

## Contributing

Pull requests are welcome! Please feel free to submit issues or enhancement requests.

## License

This project is open source and available under the [MIT License](LICENSE).