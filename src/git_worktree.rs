#![allow(dead_code)]
#![allow(unused_assignments)]

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorktreeError {
    #[error("Git command failed: {0}")]
    GitError(String),

    #[error("Worktree already exists: {0}")]
    WorktreeExists(String),

    #[error("Worktree not found: {0}")]
    WorktreeNotFound(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Not in git repository")]
    NotInGitRepo,

    #[error("Uncommitted changes in worktree")]
    UncommittedChanges,
}

type Result<T> = std::result::Result<T, WorktreeError>;

#[derive(Debug, Clone)]
pub struct Worktree {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub created_at: String,
}

impl Worktree {
    pub fn new(phase_id: &str) -> Self {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let name = format!("claude-phase-{}-{}", phase_id, timestamp);
        let path = PathBuf::from(format!("../{}", name));
        Self {
            name: name.clone(),
            path,
            branch: name,
            created_at: timestamp,
        }
    }
}

// Add validation functions
pub fn validate_git_repo() -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()?;

    if !output.status.success() {
        return Err(WorktreeError::NotInGitRepo);
    }

    Ok(())
}

pub fn check_uncommitted_changes(path: &Path) -> Result<()> {
    let output = Command::new("git")
        .current_dir(path)
        .args(["status", "--porcelain"])
        .output()?;

    if !output.status.success() {
        return Err(WorktreeError::GitError(
            "Failed to check git status".to_string(),
        ));
    }

    let status = String::from_utf8_lossy(&output.stdout);
    if !status.trim().is_empty() {
        return Err(WorktreeError::UncommittedChanges);
    }

    Ok(())
}

// Enhanced create_worktree with validation
pub fn create_worktree(phase_id: &str, base_branch: &str) -> Result<Worktree> {
    // Validate we're in a git repo
    validate_git_repo()?;

    // Check if base branch exists
    let output = Command::new("git")
        .args(["rev-parse", "--verify", base_branch])
        .output()?;

    if !output.status.success() {
        return Err(WorktreeError::GitError(format!(
            "Base branch '{}' does not exist",
            base_branch
        )));
    }
    let mut worktree = Worktree::new(phase_id);

    // Check if worktree already exists
    if worktree.path.exists() {
        return Err(WorktreeError::WorktreeExists(worktree.name.clone()));
    }

    // Check if branch already exists
    let branch_check = Command::new("git")
        .args(["rev-parse", "--verify", &worktree.branch])
        .output()?;

    if branch_check.status.success() {
        // Branch exists, use a different name
        worktree = Worktree {
            branch: format!("{}-retry", worktree.branch),
            ..worktree
        };
    }

    // Create parent directory if needed
    if let Some(parent) = worktree.path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create worktree with new branch
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            &worktree.branch,
            worktree.path.to_str().unwrap(),
            base_branch,
        ])
        .output()?;

    if !output.status.success() {
        return Err(WorktreeError::GitError(format!(
            "Failed to create worktree: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(worktree)
}

// Enhanced remove_worktree with safety checks
pub fn remove_worktree(worktree_name: &str) -> Result<()> {
    validate_git_repo()?;

    // Find the worktree path
    let worktrees = list_all_worktrees()?;
    let worktree = worktrees
        .iter()
        .find(|w| w.name == worktree_name)
        .ok_or_else(|| WorktreeError::WorktreeNotFound(worktree_name.to_string()))?;

    // Check for uncommitted changes
    if let Err(WorktreeError::UncommittedChanges) = check_uncommitted_changes(&worktree.path) {
        eprintln!("Warning: Worktree has uncommitted changes. Force removing...");
    }

    // Remove worktree
    let output = Command::new("git")
        .args([
            "worktree",
            "remove",
            worktree.path.to_str().unwrap(),
            "--force",
        ])
        .output()?;

    if !output.status.success() {
        return Err(WorktreeError::GitError(format!(
            "Failed to remove worktree: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // Delete the branch if it exists
    let _ = Command::new("git")
        .args(["branch", "-D", &worktree.branch])
        .output();

    // Prune worktree refs
    Command::new("git").args(["worktree", "prune"]).output()?;

    Ok(())
}

// Helper function to list all worktrees
pub fn list_all_worktrees() -> Result<Vec<Worktree>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()?;

    if !output.status.success() {
        return Err(WorktreeError::GitError(
            "Failed to list worktrees".to_string(),
        ));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();

    // Parse worktree list output
    let mut current_path = None;
    let mut current_branch = None;

    for line in output_str.lines() {
        if line.starts_with("worktree ") {
            current_path = Some(line.trim_start_matches("worktree ").to_string());
        } else if line.starts_with("branch refs/heads/") {
            current_branch = Some(line.trim_start_matches("branch refs/heads/").to_string());

            if let (Some(path), Some(branch)) = (&current_path, &current_branch) {
                let parts: Vec<&str> = branch.split('-').collect();
                let timestamp = if parts.len() >= 4 && branch.starts_with("claude-phase-") {
                    parts[3..].join("-")
                } else {
                    "unknown".to_string()
                };
                worktrees.push(Worktree {
                    name: branch.clone(),
                    path: PathBuf::from(path),
                    branch: branch.clone(),
                    created_at: timestamp,
                });
            }
        }
    }

    Ok(worktrees)
}

pub fn list_claude_worktrees() -> Result<Vec<Worktree>> {
    let all_worktrees = list_all_worktrees()?;
    Ok(all_worktrees
        .into_iter()
        .filter(|w| w.branch.starts_with("claude-phase-"))
        .collect())
}

pub fn cleanup_old_worktrees(max_worktrees: usize) -> Result<()> {
    let mut worktrees = list_claude_worktrees()?;

    if worktrees.len() <= max_worktrees {
        return Ok(());
    }

    // Sort by creation time (oldest first)
    worktrees.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    // Remove oldest worktrees
    let to_remove = worktrees.len() - max_worktrees;
    for worktree in worktrees.iter().take(to_remove) {
        println!("Removing old worktree: {}", worktree.name);
        remove_worktree(&worktree.name)?;
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorktreeState {
    pub active_worktrees: Vec<ActiveWorktree>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActiveWorktree {
    pub phase_id: String,
    pub worktree_name: String,
    pub worktree_path: PathBuf,
    pub created_at: String,
    pub status: WorktreeStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum WorktreeStatus {
    Active,
    Completed,
    Failed,
}

impl WorktreeState {
    pub fn new() -> Self {
        WorktreeState {
            active_worktrees: Vec::new(),
        }
    }

    pub fn load() -> std::io::Result<Self> {
        let state_path = ".claude-launcher/worktree_state.json";
        if Path::new(state_path).exists() {
            let contents = std::fs::read_to_string(state_path)?;
            Ok(serde_json::from_str(&contents)?)
        } else {
            Ok(Self::new())
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let state_path = ".claude-launcher/worktree_state.json";
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(state_path, contents)?;
        Ok(())
    }

    pub fn add_worktree(&mut self, phase_id: String, worktree: &Worktree) {
        self.active_worktrees.push(ActiveWorktree {
            phase_id,
            worktree_name: worktree.name.clone(),
            worktree_path: worktree.path.clone(),
            created_at: worktree.created_at.clone(),
            status: WorktreeStatus::Active,
        });
    }

    pub fn mark_completed(&mut self, phase_id: &str) {
        if let Some(wt) = self
            .active_worktrees
            .iter_mut()
            .find(|w| w.phase_id == phase_id && w.status == WorktreeStatus::Active)
        {
            wt.status = WorktreeStatus::Completed;
        }
    }

    pub fn mark_failed(&mut self, phase_id: &str) {
        if let Some(wt) = self
            .active_worktrees
            .iter_mut()
            .find(|w| w.phase_id == phase_id && w.status == WorktreeStatus::Active)
        {
            wt.status = WorktreeStatus::Failed;
        }
    }

    pub fn get_active_worktree(&self, phase_id: &str) -> Option<&ActiveWorktree> {
        self.active_worktrees
            .iter()
            .find(|w| w.phase_id == phase_id && w.status == WorktreeStatus::Active)
    }

    pub fn cleanup_completed(&mut self, config: &crate::WorktreeConfig) -> std::io::Result<()> {
        let completed: Vec<ActiveWorktree> = self
            .active_worktrees
            .iter()
            .filter(|w| w.status == WorktreeStatus::Completed)
            .cloned()
            .collect();

        for worktree in completed {
            println!("Cleaning up completed worktree: {}", worktree.worktree_name);
            if let Err(e) = remove_worktree(&worktree.worktree_name) {
                eprintln!(
                    "Warning: Failed to remove worktree {}: {}",
                    worktree.worktree_name, e
                );
            }

            // Remove from state
            self.active_worktrees
                .retain(|w| w.worktree_name != worktree.worktree_name);
        }

        // Apply max worktrees limit
        if config.auto_cleanup {
            match cleanup_old_worktrees(config.max_worktrees) {
                Ok(_) => {}
                Err(e) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to cleanup old worktrees: {}", e),
                    ));
                }
            }
        }

        self.save()?;
        Ok(())
    }
}

// Add recovery function for orphaned worktrees
pub fn recover_orphaned_worktrees() -> Result<Vec<String>> {
    validate_git_repo()?;

    let mut recovered = Vec::new();

    // Run worktree prune in dry-run mode to find orphaned worktrees
    let output = Command::new("git")
        .args(["worktree", "prune", "--dry-run", "-v"])
        .output()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("Removing worktrees") {
                if let Some(path) = line.split("Removing worktrees/").nth(1) {
                    recovered.push(path.trim_end_matches(':').to_string());
                }
            }
        }

        // Actually prune if we found orphaned worktrees
        if !recovered.is_empty() {
            Command::new("git").args(["worktree", "prune"]).output()?;
        }
    }

    Ok(recovered)
}

// Add function to safely sync worktree changes
pub fn sync_worktree_safely(worktree: &Worktree, target_branch: &str) -> Result<()> {
    validate_git_repo()?;

    // Ensure worktree exists
    if !worktree.path.exists() {
        return Err(WorktreeError::WorktreeNotFound(worktree.name.clone()));
    }

    // Fetch latest changes
    Command::new("git")
        .current_dir(&worktree.path)
        .args(["fetch", "origin"])
        .output()?;

    // Check if we can fast-forward merge
    let merge_base = Command::new("git")
        .current_dir(&worktree.path)
        .args([
            "merge-base",
            &worktree.branch,
            &format!("origin/{}", target_branch),
        ])
        .output()?;

    if !merge_base.status.success() {
        return Err(WorktreeError::GitError(
            "Cannot determine merge base".to_string(),
        ));
    }

    // Attempt rebase to keep history clean
    let rebase = Command::new("git")
        .current_dir(&worktree.path)
        .args(["rebase", &format!("origin/{}", target_branch)])
        .output()?;

    if !rebase.status.success() {
        // Abort rebase if it failed
        Command::new("git")
            .current_dir(&worktree.path)
            .args(["rebase", "--abort"])
            .output()?;

        return Err(WorktreeError::GitError(
            "Cannot rebase worktree changes".to_string(),
        ));
    }

    Ok(())
}

// Helper function to get current git branch
pub fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;

    if !output.status.success() {
        return Err(WorktreeError::GitError(
            "Failed to get current branch".to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
#[path = "git_worktree_tests.rs"]
mod tests;
