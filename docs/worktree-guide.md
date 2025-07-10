# Git Worktree Integration Guide

## Overview

The git worktree feature in Claude Launcher provides isolated environments for each phase of development, enabling parallel execution without branch conflicts.

## Architecture

### Components

1. **Worktree Module** (`src/git_worktree.rs`)
   - Core worktree operations
   - State management
   - Cleanup utilities

2. **Configuration** (`WorktreeConfig`)
   - Runtime configuration
   - Naming patterns
   - Cleanup policies

3. **State Tracking** (`worktree_state.json`)
   - Active worktree tracking
   - Phase association
   - Status management

### Flow Diagram

```
Main Repository
├── .claude-launcher/
│   ├── todos.json
│   ├── config.json
│   └── worktree_state.json
└── src/

Worktrees:
../claude-phase-1-20240115_143022/
├── .claude-launcher/ (copied)
├── src/ (from base branch)
└── [phase 1 changes]

../claude-phase-2-20240115_144512/
├── .claude-launcher/ (copied)
├── src/ (from base branch)
└── [phase 2 changes]
```

## Implementation Details

### Worktree Lifecycle

1. **Creation**
   - Check for existing worktree for phase
   - Create new worktree with unique name
   - Copy configuration files
   - Update state tracking

2. **Execution**
   - Launch Claude in worktree directory
   - Monitor todos.json for updates
   - Sync changes back to main repo

3. **Completion**
   - Mark phase as completed in state
   - Create commit in worktree
   - Optional: auto-merge to base branch
   - Cleanup if auto_cleanup enabled

### Best Practices

1. **Branch Strategy**
   - Use descriptive base branches
   - Keep worktree branches focused
   - Merge completed work promptly

2. **Cleanup Policy**
   - Enable auto_cleanup for CI/CD
   - Set reasonable max_worktrees
   - Manually review before cleanup

3. **State Management**
   - Don't edit worktree_state.json manually
   - Use --list-worktrees to inspect
   - Reset state by deleting the file

## Advanced Usage

### Custom Naming Patterns

```json
{
  "worktree": {
    "naming_pattern": "feature-{id}-{timestamp}"
  }
}
```

### Integration with CI/CD

```yaml
# .github/workflows/claude-phases.yml
name: Execute Claude Phases
on:
  workflow_dispatch:

jobs:
  run-phases:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run Claude Launcher with Worktrees
        run: |
          claude-launcher --worktree-per-phase
          claude-launcher --list-worktrees
      - name: Cleanup
        if: always()
        run: claude-launcher --cleanup-worktrees
```

### Debugging

Enable verbose logging:
```bash
RUST_LOG=debug claude-launcher --worktree-per-phase
```

Inspect worktree state:
```bash
cat .claude-launcher/worktree_state.json | jq
```

Manual worktree operations:
```bash
# List all worktrees
git worktree list

# Remove specific worktree
git worktree remove ../claude-phase-1-20240115_143022

# Prune stale worktrees
git worktree prune
```

## Common Patterns

### Sequential Phase Execution with Worktrees

When phases depend on each other:

```json
{
  "worktree": {
    "enabled": true,
    "auto_cleanup": false
  }
}
```

This keeps worktrees around for reference during dependent phases.

### Parallel Feature Development

For independent features:

```json
{
  "worktree": {
    "enabled": true,
    "max_worktrees": 10,
    "auto_cleanup": true
  }
}
```

This allows many parallel developments with automatic cleanup.

### Testing and Validation

Each worktree can run its own test suite:

```bash
# In worktree
cd ../claude-phase-1-20240115_143022
cargo test
cargo clippy -- -D warnings
```

### Merging Strategies

**Fast-forward merge** (for clean linear history):
```bash
git merge claude-phase-1-20240115_143022
```

**No-fast-forward merge** (preserves phase boundaries):
```bash
git merge --no-ff claude-phase-1-20240115_143022
```

**Squash merge** (single commit per phase):
```bash
git merge --squash claude-phase-1-20240115_143022
git commit -m "Phase 1: Foundation Setup"
```

## Troubleshooting Guide

### Common Issues

1. **"Not in a git repository" error**
   - Solution: Initialize git with `git init`
   - Ensure you're in the project root

2. **"Base branch does not exist" error**
   - Solution: Create the branch or update config
   - Check: `git branch -a`

3. **"Worktree already exists" error**
   - Solution: Use `--list-worktrees` to check status
   - Clean up with `--cleanup-worktrees`

4. **Merge conflicts after phase completion**
   - Solution: Resolve in worktree before merging
   - Use: `git merge --no-commit` for review

5. **Lost worktree state**
   - Solution: Run `--list-worktrees` to rebuild
   - Check: `git worktree list` for all worktrees

### Recovery Procedures

**Recover from interrupted phase:**
```bash
# Find the worktree
claude-launcher --list-worktrees

# Resume in existing worktree
claude-launcher --worktree-per-phase
```

**Clean up orphaned worktrees:**
```bash
# Prune git's worktree list
git worktree prune

# Clean launcher state
rm .claude-launcher/worktree_state.json
claude-launcher --list-worktrees
```

**Force cleanup all worktrees:**
```bash
# List all claude worktrees
git worktree list | grep claude-phase

# Remove each one
git worktree remove -f ../claude-phase-*

# Reset state
rm .claude-launcher/worktree_state.json
```

## Performance Considerations

1. **Disk Space**: Each worktree is a full copy of your repository
2. **Memory**: Multiple Claude instances consume significant RAM
3. **CPU**: Parallel compilation/testing can saturate CPU

### Recommendations

- Limit max_worktrees based on system resources
- Enable auto_cleanup for space management
- Use SSDs for better worktree performance
- Consider --step-by-step mode for resource-constrained systems