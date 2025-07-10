# Claude Launcher

A Rust CLI tool that launches multiple Claude AI instances in separate iTerm tabs to work on tasks in parallel. Specialized for Elm and Lamdera development, with built-in support for Test-Driven Development (TDD) using lamdera-program-test.

## What it does

`claude-launcher` opens new iTerm tabs, each running Claude with a specific task. Each Claude instance:

1. Changes to the directory where you ran the command
2. Reads the `.claude-launcher/todos.json` file in that directory
3. Executes the assigned task
4. Updates `.claude-launcher/todos.json` to mark the task as complete with a comment
5. Stops after completing the task

### Automatic Task Detection

When run without arguments, `claude-launcher` automatically:
- Reads `.claude-launcher/todos.json` to find the next phase with TODO status
- Launches all TODO tasks in that phase in parallel
- Phase CTOs automatically spawn the next phase when complete

### Step-by-Step Mode

For debugging or when tasks must run sequentially:
```bash
claude-launcher --step-by-step
```
- Runs only the first TODO task in the current phase
- Each task calls `claude-launcher --step-by-step` when done
- Ensures only one agent is active at a time
- Useful for debugging or when tasks have dependencies

## Git Worktree Integration

Claude Launcher supports running each phase in isolated git worktrees, allowing parallel development without branch conflicts. This implements [Anthropic's recommended workflow](https://docs.anthropic.com/en/docs/claude-code/common-workflows#run-parallel-claude-code-sessions-with-git-worktrees) for running parallel Claude Code sessions, with additional automation and management features.

### Features

- **Isolated Execution**: Each phase runs in its own git worktree
- **Automatic Management**: Worktrees are created and cleaned up automatically
- **Parallel Development**: Multiple phases can be developed simultaneously
- **Branch Management**: Each worktree gets its own branch (claude-phase-{id}-{timestamp})
- **State Tracking**: Track which phases are running in which worktrees

### Configuration

Add worktree configuration to `.claude-launcher/config.json`:

```json
{
  "name": "Your Project",
  "agent": {
    "before_stop_commands": []
  },
  "cto": {
    "validation_commands": [],
    "few_errors_max": 3
  },
  "worktree": {
    "enabled": false,
    "naming_pattern": "claude-phase-{id}-{timestamp}",
    "max_worktrees": 5,
    "base_branch": "main",
    "auto_cleanup": true
  }
}
```

#### Configuration Options

- `enabled`: Enable worktree mode by default (default: false)
- `naming_pattern`: Pattern for worktree names (supports {id} and {timestamp})
- `max_worktrees`: Maximum number of worktrees to keep (default: 5)
- `base_branch`: Branch to create worktrees from (default: "main")
- `auto_cleanup`: Automatically remove completed worktrees (default: true)

### Usage

#### Run with Worktrees

```bash
# Run next phase in a worktree
claude-launcher --worktree-per-phase

# Or enable in config and use auto mode
claude-launcher
```

#### List Active Worktrees

```bash
claude-launcher --list-worktrees
```

Output:
```
Claude Launcher Active Worktrees
================================

Found 2 worktree(s):

1. claude-phase-1-20240115_143022
   Path: ../claude-phase-1-20240115_143022
   Branch: claude-phase-1-20240115_143022
   Created: 20240115_143022
   Phase ID: 1
   Status: Active
   Phase: Foundation Setup
   Progress: 1 TODO, 2 IN PROGRESS, 1 DONE

2. claude-phase-2-20240115_144512
   Path: ../claude-phase-2-20240115_144512
   Branch: claude-phase-2-20240115_144512
   Created: 20240115_144512
   Phase ID: 2
   Status: Completed
   Phase: Feature Implementation
   Progress: 0 TODO, 0 IN PROGRESS, 4 DONE
```

#### Clean Up Worktrees

```bash
# Manual cleanup
claude-launcher --cleanup-worktrees

# Automatic cleanup happens when:
# - auto_cleanup is enabled
# - A phase is marked as completed
# - max_worktrees limit is exceeded
```

### Workflow Example

1. **Initialize Project**
   ```bash
   claude-launcher --init
   ```

2. **Configure Worktrees**
   Edit `.claude-launcher/config.json` to enable worktrees:
   ```json
   {
     "name": "Your Project",
     "agent": {
       "before_stop_commands": []
     },
     "cto": {
       "validation_commands": [],
       "few_errors_max": 3
     },
     "worktree": {
       "enabled": true,
       "max_worktrees": 3
     }
   }
   ```

3. **Create Task Plan**
   ```bash
   claude-launcher --create-task "Implement new feature"
   ```

4. **Execute with Worktrees**
   ```bash
   claude-launcher  # Auto mode with worktrees
   ```

5. **Monitor Progress**
   ```bash
   claude-launcher --list-worktrees
   ```

6. **Merge Completed Work**
   When a phase is completed, the worktree branch can be merged:
   ```bash
   git merge --no-ff claude-phase-1-20240115_143022
   ```

### Benefits

- **Isolation**: Each phase's changes are isolated from others
- **Parallel Work**: Multiple Claude instances can work on different phases simultaneously
- **Easy Rollback**: If a phase fails, simply delete the worktree
- **Clean History**: Each phase gets its own branch with clear commits
- **No Conflicts**: Phases can't interfere with each other's changes

### Troubleshooting

**Worktree Creation Fails**
- Ensure you're in a git repository
- Check that the base branch exists
- Verify you have sufficient disk space

**Can't Remove Worktree**
- Check if you have uncommitted changes in the worktree
- Use `--cleanup-worktrees` for safe removal
- Manually remove with `git worktree remove -f <path>`

**State File Issues**
- State is tracked in `.claude-launcher/worktree_state.json`
- Delete this file to reset worktree tracking
- Run `--list-worktrees` to rebuild state

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

# Initialize with Lamdera preset
claude-launcher --init-lamdera

# Smart initialization (analyzes your project)
claude-launcher --smart-init

# Generate task phases from requirements
claude-launcher --create-task "your project requirements"

# Auto-launch next available tasks
claude-launcher

# Or manually launch specific tasks
claude-launcher "Phase 1, Step 1A: Task name" "Phase 1, Step 1B: Another task"
```

### Commands

- `--init`: Creates `.claude-launcher/` directory with empty config and todos.json
- `--init-lamdera`: Creates `.claude-launcher/` with Lamdera preset configuration
- `--smart-init`: Analyzes your project and creates appropriate configuration
- `--create-task "requirements"`: Analyzes your requirements and generates detailed task phases
- No arguments: Automatically detects and launches the next TODO phase (parallel execution)
- `--step-by-step`: Runs tasks sequentially, one at a time
- `--worktree-per-phase`: Run each phase in its own git worktree
- `--list-worktrees`: List all active claude-launcher worktrees
- `--cleanup-worktrees`: Clean up completed worktrees

### Workflow

1. **Initialize your project**:
   ```bash
   # For a generic project
   claude-launcher --init
   
   # For a Lamdera project
   claude-launcher --init-lamdera
   
   # To auto-detect your project type
   claude-launcher --smart-init
   ```
   This creates a `.claude-launcher/` directory with `config.json` and `todos.json` files.

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

### Directory Structure

Claude Launcher uses a `.claude-launcher/` directory to store configuration and task tracking:

```
.claude-launcher/
├── config.json    # Project-specific validation commands and settings
└── todos.json     # Task phases and progress tracking
```

#### config.json

The configuration file defines validation commands that CTOs will run and commands available to agents:

```json
{
  "name": "Project Name",
  "agent": {
    "before_stop_commands": [],
    "commands": [
      {
        "description": "Add internationalization keys",
        "pattern": "elm-i18n add --fr \"French text\" --en \"English text\" KEY_NAME",
        "use_instead_of": "editing src/I18n.elm directly"
      }
    ]
  },
  "cto": {
    "validation_commands": [
      {
        "command": "npm test",
        "description": "Run tests"
      },
      {
        "command": "npm run lint",
        "description": "Check code quality"
      }
    ],
    "few_errors_max": 5
  },
  "worktree": {
    "enabled": false,
    "naming_pattern": "claude-phase-{id}-{timestamp}",
    "max_worktrees": 5,
    "base_branch": "main",
    "auto_cleanup": true
  }
}
```

##### Agent Commands

The `commands` array allows you to define project-specific commands that agents should use instead of directly editing files. This is particularly useful for:
- Code generation tools
- Internationalization (i18n) management
- Database migrations
- Schema updates
- Any tool that manages file updates programmatically

When commands are configured, agents will be instructed to use these commands rather than manually editing the specified files.

#### todos.json

The task file contains phases and steps:

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

### Elm/Lamdera Specialization

Claude-launcher is optimized for Elm and Lamdera projects:
- **Compilation Validation**: Phase CTOs run validation commands defined in config.json
- **Test-Driven Development**: When lamdera-program-test is detected, agents follow TDD practices
- **Test Execution**: Configurable test commands based on your project
- **Smart Error Handling**: CTOs analyze compilation and test failures to create fix tasks
- **Tool Integration**: Pre-configured commands for elm-i18n and other Elm tools

#### Example: Using elm-i18n Commands

When `--init-lamdera` is used, the configuration includes elm-i18n commands:

```json
"commands": [
  {
    "description": "Add internationalization keys",
    "pattern": "elm-i18n add --fr \"French text\" --en \"English text\" KEY_NAME",
    "use_instead_of": "editing src/I18n.elm directly"
  },
  {
    "description": "Add function-based translations",
    "pattern": "elm-i18n add-fn --type-sig \"Int -> String\" --en \"\\n -> ...\" --fr \"\\n -> ...\" functionName",
    "use_instead_of": "editing src/I18n.elm for parameterized translations"
  }
]
```

With these commands configured, agents will automatically use `elm-i18n` commands instead of manually editing the I18n.elm file, ensuring consistent formatting and preventing merge conflicts.

### Phase CTO Behavior

When all tasks in a phase are complete but the phase status is still TODO, running `claude-launcher` will:
1. Detect that all steps are DONE
2. Spawn a dedicated Phase CTO agent
3. The Phase CTO will:
   - Review all completed tasks in the phase
   - Run validation commands from config.json
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

Initialize and plan a new Lamdera app:
```bash
claude-launcher --init
claude-launcher --create-task "Build a Lamdera property management app with tenant portals and real-time notifications"
claude-launcher  # Auto-launches Phase 1 tasks in parallel
```

For Test-Driven Development with lamdera-program-test:
```bash
claude-launcher --create-task "Add user authentication with TDD using lamdera-program-test"
claude-launcher --step-by-step  # Runs tests first, then implementation
```

For sequential execution (useful for debugging):
```bash
claude-launcher --step-by-step  # Runs Phase 1, Step 1A
# After 1A completes, it automatically runs 1B, then 1C, etc.
```

## Limitations

- macOS only (uses AppleScript)
- Requires iTerm2 (not Terminal.app)
- Maximum of 10 concurrent tasks

## Contributing

Pull requests are welcome! Please feel free to submit issues or enhancement requests.

### Running Tests

**Important:** Tests may fail intermittently when run in parallel due to race conditions with directory changes. This is a known issue.

If you see random test failures like:
- `No such file or directory`
- `Failed to load config`
- Tests that pass sometimes and fail other times

Run tests single-threaded to ensure they pass consistently:

```bash
cargo test -- --test-threads=1
```

This forces tests to run sequentially, preventing conflicts when multiple tests change the current directory simultaneously. The failures are not bugs in the code, but rather a limitation of the test setup.

## License

This project is open source and available under the [MIT License](LICENSE).