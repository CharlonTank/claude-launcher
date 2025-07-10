# Git Worktree Integration Example

This example demonstrates using Claude Launcher with git worktrees for a typical web application project.

## Project Setup

Let's say we're building a task management API with authentication. Here's how to use worktrees for parallel development:

### 1. Initialize the Project

```bash
# Create project directory
mkdir task-api && cd task-api
git init

# Initialize Claude Launcher
claude-launcher --init

# Create initial commit
echo "# Task API" > README.md
git add .
git commit -m "Initial commit"
```

### 2. Configure Worktrees

Edit `.claude-launcher/config.json`:

```json
{
  "name": "Task Management API",
  "agent": {
    "before_stop_commands": []
  },
  "cto": {
    "validation_commands": [
      {
        "command": "npm test",
        "description": "Run test suite"
      },
      {
        "command": "npm run lint",
        "description": "Check code style"
      },
      {
        "command": "npm run type-check",
        "description": "TypeScript validation"
      }
    ],
    "few_errors_max": 5
  },
  "worktree": {
    "enabled": true,
    "naming_pattern": "task-api-phase-{id}-{timestamp}",
    "max_worktrees": 5,
    "base_branch": "main",
    "auto_cleanup": false
  }
}
```

### 3. Create Task Plan

```bash
claude-launcher --create-task "Build a REST API with:
- User authentication (JWT)
- Task CRUD operations
- PostgreSQL database
- Express.js framework
- TypeScript
- Jest for testing"
```

This generates phases like:

```json
{
  "phases": [
    {
      "id": 1,
      "name": "Foundation Setup",
      "steps": [
        {
          "id": "1A",
          "name": "Database Schema and Models",
          "status": "TODO"
        },
        {
          "id": "1B",
          "name": "Express Server Setup",
          "status": "TODO"
        },
        {
          "id": "1C",
          "name": "TypeScript Configuration",
          "status": "TODO"
        }
      ]
    },
    {
      "id": 2,
      "name": "Authentication System",
      "steps": [
        {
          "id": "2A",
          "name": "JWT Implementation",
          "status": "TODO"
        },
        {
          "id": "2B",
          "name": "User Routes and Controllers",
          "status": "TODO"
        },
        {
          "id": "2C",
          "name": "Auth Middleware",
          "status": "TODO"
        }
      ]
    },
    {
      "id": 3,
      "name": "Task Management Features",
      "steps": [
        {
          "id": "3A",
          "name": "Task CRUD Routes",
          "status": "TODO"
        },
        {
          "id": "3B",
          "name": "Task Validation and Business Logic",
          "status": "TODO"
        }
      ]
    }
  ]
}
```

### 4. Execute with Worktrees

```bash
# This creates a worktree for Phase 1 and runs all steps in parallel
claude-launcher --worktree-per-phase
```

Output:
```
Running in worktree-per-phase mode...
Starting phase 1 in worktree mode: Foundation Setup
Creating new worktree for phase 1...
Created worktree: task-api-phase-1-20240115_143022 at ../task-api-phase-1-20240115_143022
Executing phase in worktree: task-api-phase-1-20240115_143022
```

### 5. Monitor Progress

```bash
claude-launcher --list-worktrees
```

Output:
```
Claude Launcher Active Worktrees
================================

Found 1 worktree(s):

1. task-api-phase-1-20240115_143022
   Path: ../task-api-phase-1-20240115_143022
   Branch: task-api-phase-1-20240115_143022
   Created: 20240115_143022
   Phase ID: 1
   Status: Active
   Phase: Foundation Setup
   Progress: 0 TODO, 3 IN PROGRESS, 0 DONE
```

### 6. Check Worktree Progress

```bash
# Navigate to worktree
cd ../task-api-phase-1-20240115_143022

# Check what's been done
git status
git log --oneline

# Run tests in the worktree
npm test

# Return to main repo
cd ../task-api
```

### 7. Phase Completion

When Phase 1 completes, the Phase CTO will:
1. Run validation commands in the worktree
2. Fix any issues
3. Mark phase as DONE
4. Start Phase 2 in a new worktree

### 8. Review and Merge

```bash
# List completed worktrees
claude-launcher --list-worktrees

# Review changes
git log --oneline --graph --all

# Merge Phase 1
git merge --no-ff task-api-phase-1-20240115_143022 -m "Merge Phase 1: Foundation Setup"

# Clean up completed worktree
claude-launcher --cleanup-worktrees
```

### 9. Continue Development

Phase 2 automatically starts in a new worktree:
```
Created worktree: task-api-phase-2-20240115_150234
```

Each phase builds on the previous one, with isolated development and clean merging.

## Advanced Scenarios

### Handling Conflicts

If Phase 2 and Phase 3 modify the same files:

```bash
# In main repo, after merging Phase 2
git merge --no-ff task-api-phase-2-20240115_150234

# Before merging Phase 3, rebase it
cd ../task-api-phase-3-20240115_151122
git fetch origin
git rebase origin/main

# Resolve any conflicts
git status
# Edit conflicted files
git add .
git rebase --continue

# Now merge in main repo
cd ../task-api
git merge --no-ff task-api-phase-3-20240115_151122
```

### Debugging Failed Phases

```bash
# List worktrees to find failed phase
claude-launcher --list-worktrees

# Navigate to failed worktree
cd ../task-api-phase-2-20240115_150234

# Check logs and status
git status
cat .claude-launcher/todos.json | jq '.phases[] | select(.id == 2)'

# Fix issues manually or re-run
claude-launcher --worktree-per-phase
```

### Parallel Feature Development

For independent features, configure:

```json
{
  "worktree": {
    "enabled": true,
    "max_worktrees": 10,
    "auto_cleanup": false
  }
}
```

Then run multiple phases simultaneously:
```bash
# Terminal 1
claude-launcher --worktree-per-phase  # Starts Phase 1

# Terminal 2 (after Phase 1 starts)
# Manually edit todos.json to start Phase 3
claude-launcher --worktree-per-phase  # Starts Phase 3
```

## Tips

1. **Disk Space**: Each worktree is a full copy. Monitor with:
   ```bash
   du -sh ../task-api-phase-*
   ```

2. **Branch Visualization**:
   ```bash
   git log --graph --pretty=format:'%Cred%h%Creset -%C(yellow)%d%Creset %s %Cgreen(%cr)%Creset' --abbrev-commit --all
   ```

3. **Worktree Comparison**:
   ```bash
   git diff main..task-api-phase-1-20240115_143022
   ```

4. **Emergency Cleanup**:
   ```bash
   git worktree list | grep task-api-phase | awk '{print $1}' | xargs -I {} git worktree remove -f {}
   ```

This workflow enables efficient parallel development while maintaining code quality and clean git history!