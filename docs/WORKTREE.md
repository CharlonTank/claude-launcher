# Git Worktree Quick Reference

## Enable Worktree Mode

Add to your `.claude-launcher/config.json`:

```json
"worktree": {
  "enabled": true
}
```

## Commands

```bash
# Run with worktrees
claude-launcher --worktree-per-phase

# List worktrees
claude-launcher --list-worktrees

# Cleanup
claude-launcher --cleanup-worktrees
```

## Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `enabled` | `false` | Enable worktree mode by default |
| `naming_pattern` | `"claude-phase-{id}-{timestamp}"` | Pattern for worktree names |
| `max_worktrees` | `5` | Maximum number of worktrees to keep |
| `base_branch` | `"main"` | Branch to create worktrees from |
| `auto_cleanup` | `true` | Automatically remove completed worktrees |

## Common Workflows

### Parallel Feature Development
```json
{
  "worktree": {
    "enabled": true,
    "max_worktrees": 10,
    "auto_cleanup": true
  }
}
```

### Sequential with Manual Review
```json
{
  "worktree": {
    "enabled": true,
    "max_worktrees": 3,
    "auto_cleanup": false
  }
}
```

### Feature Branch Workflow
```json
{
  "worktree": {
    "enabled": true,
    "base_branch": "develop",
    "naming_pattern": "feature-{id}-{timestamp}"
  }
}
```

## Troubleshooting

### Reset Worktree State
```bash
rm .claude-launcher/worktree_state.json
claude-launcher --list-worktrees
```

### Manual Cleanup
```bash
git worktree list
git worktree remove -f ../claude-phase-*
git worktree prune
```

### Check Worktree Status
```bash
cd ../claude-phase-1-*
git status
git log --oneline -5
```

## Best Practices

1. **Commit frequently** in worktrees to preserve work
2. **Review changes** before merging worktree branches
3. **Use descriptive commit messages** in each worktree
4. **Keep worktrees focused** on their specific phase
5. **Clean up regularly** to free disk space

## Merging Worktree Changes

```bash
# Review changes
git log --oneline claude-phase-1-20240115_143022

# Merge with history
git merge --no-ff claude-phase-1-20240115_143022

# Or squash merge
git merge --squash claude-phase-1-20240115_143022
git commit -m "Phase 1: Foundation implementation"
```