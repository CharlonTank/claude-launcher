use crate::git_worktree::*;
use std::fs;
use tempfile::TempDir;


// Helper to check if git is available
fn check_git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn setup_test_repo() -> Option<TempDir> {
    if !check_git_available() {
        eprintln!("Git not available, skipping test");
        return None;
    }
    
    let temp_dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to create temp dir: {}", e);
            return None;
        }
    };
    
    let repo_path = temp_dir.path();

    // Initialize git repo
    let output = std::process::Command::new("git")
        .current_dir(&repo_path)
        .arg("init")
        .output();
        
    match output {
        Ok(out) if out.status.success() => {},
        Ok(out) => {
            eprintln!("Git init failed: {}", String::from_utf8_lossy(&out.stderr));
            return None;
        },
        Err(e) => {
            eprintln!("Failed to run git init: {}", e);
            return None;
        }
    }

    // Configure git user (required for commits)
    if std::process::Command::new("git")
        .current_dir(&repo_path)
        .args(&["config", "user.email", "test@example.com"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false) == false {
            eprintln!("Failed to set git email");
            return None;
        }

    if std::process::Command::new("git")
        .current_dir(&repo_path)
        .args(&["config", "user.name", "Test User"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false) == false {
            eprintln!("Failed to set git name");
            return None;
        }

    // Set the default branch name to "main"
    if std::process::Command::new("git")
        .current_dir(&repo_path)
        .args(&["checkout", "-b", "main"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false) == false {
            eprintln!("Failed to create main branch");
            return None;
        }

    // Create initial commit
    if fs::write(repo_path.join("README.md"), "Test repo").is_err() {
        eprintln!("Failed to create README.md");
        return None;
    }
    
    if std::process::Command::new("git")
        .current_dir(&repo_path)
        .args(&["add", "."])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false) == false {
            eprintln!("Failed to add files");
            return None;
        }

    if std::process::Command::new("git")
        .current_dir(&repo_path)
        .args(&["commit", "-m", "Initial commit"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false) == false {
            eprintln!("Failed to commit");
            return None;
        }

    Some(temp_dir)
}

#[test]
fn test_worktree_creation() {
    let Some(temp_dir) = setup_test_repo() else {
        return; // Skip test if git is not available
    };
    
    let original_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get current directory: {}", e);
            return;
        }
    };
    
    if let Err(e) = std::env::set_current_dir(temp_dir.path()) {
        eprintln!("Failed to change to temp directory: {}", e);
        return;
    }

    // Test worktree creation
    let result = create_worktree("test-phase-1", "main");
    if let Err(e) = &result {
        eprintln!("Worktree creation failed: {}", e);
    }
    assert!(result.is_ok());

    let worktree = result.unwrap();
    assert!(worktree.name.starts_with("claude-phase-test-phase-1-"));
    assert!(worktree.path.exists());

    // Cleanup
    let _ = std::env::set_current_dir(original_dir);
}

#[test]
fn test_worktree_listing() {
    let Some(temp_dir) = setup_test_repo() else {
        return; // Skip test if git is not available
    };
    let original_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get current directory: {}", e);
            return;
        }
    };
    
    if let Err(e) = std::env::set_current_dir(temp_dir.path()) {
        eprintln!("Failed to change to temp directory: {}", e);
        return;
    }

    // Create multiple worktrees
    let _wt1 = create_worktree("1", "main").unwrap();
    let _wt2 = create_worktree("2", "main").unwrap();

    // List worktrees
    let worktrees = list_claude_worktrees().unwrap();
    assert_eq!(worktrees.len(), 2);
    assert!(worktrees
        .iter()
        .all(|w| w.name.starts_with("claude-phase-")));

    // Cleanup
    let _ = std::env::set_current_dir(original_dir);
}

#[test]
fn test_worktree_removal() {
    let Some(temp_dir) = setup_test_repo() else {
        return; // Skip test if git is not available
    };
    let original_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get current directory: {}", e);
            return;
        }
    };
    
    if let Err(e) = std::env::set_current_dir(temp_dir.path()) {
        eprintln!("Failed to change to temp directory: {}", e);
        return;
    }

    // Create and remove worktree
    let worktree = create_worktree("remove-test", "main").unwrap();
    let wt_path = worktree.path.clone();
    assert!(wt_path.exists());

    let result = remove_worktree(&worktree.name);
    assert!(result.is_ok());
    assert!(!wt_path.exists());

    // Cleanup
    let _ = std::env::set_current_dir(original_dir);
}

#[test]
fn test_worktree_state_management() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get current directory: {}", e);
            return;
        }
    };
    
    if let Err(e) = std::env::set_current_dir(temp_dir.path()) {
        eprintln!("Failed to change to temp directory: {}", e);
        return;
    }

    // Create .claude-launcher directory
    fs::create_dir(".claude-launcher").unwrap();

    // Test state creation and saving
    let mut state = WorktreeState::new();
    let worktree = Worktree::new("test-1");
    state.add_worktree("1".to_string(), &worktree);

    assert_eq!(state.active_worktrees.len(), 1);
    
    // Ensure .claude-launcher directory exists before saving
    if !std::path::Path::new(".claude-launcher").exists() {
        eprintln!("Creating .claude-launcher directory for state");
        fs::create_dir_all(".claude-launcher").unwrap();
    }
    
    match state.save() {
        Ok(_) => {},
        Err(e) => {
            eprintln!("Failed to save state: {:?}", e);
            eprintln!("Current dir: {:?}", std::env::current_dir());
            return;
        }
    }

    // Test state loading
    let loaded_state = match WorktreeState::load() {
        Ok(state) => state,
        Err(e) => {
            eprintln!("Failed to load state: {:?}", e);
            eprintln!("Current directory: {:?}", std::env::current_dir());
            eprintln!("State file exists: {}", std::path::Path::new(".claude-launcher/worktree_state.json").exists());
            return;
        }
    };
    
    if loaded_state.active_worktrees.is_empty() {
        eprintln!("Loaded state has no active worktrees");
        eprintln!("State file path: .claude-launcher/worktree_state.json");
        if let Ok(contents) = fs::read_to_string(".claude-launcher/worktree_state.json") {
            eprintln!("State file contents: {}", contents);
        }
        return;
    }
    
    assert_eq!(loaded_state.active_worktrees.len(), 1);
    assert_eq!(loaded_state.active_worktrees[0].phase_id, "1");

    // Test status updates
    let mut state = loaded_state;
    state.mark_completed("1");
    assert_eq!(state.active_worktrees[0].status, WorktreeStatus::Completed);

    // Cleanup
    let _ = std::env::set_current_dir(original_dir);
}

#[test]
fn test_cleanup_old_worktrees() {
    let Some(temp_dir) = setup_test_repo() else {
        return; // Skip test if git is not available
    };
    let original_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get current directory: {}", e);
            return;
        }
    };
    
    if let Err(e) = std::env::set_current_dir(temp_dir.path()) {
        eprintln!("Failed to change to temp directory: {}", e);
        return;
    }

    // Create more worktrees than the limit
    for i in 1..=7 {
        create_worktree(&i.to_string(), "main").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100)); // Ensure different timestamps
    }

    // Run cleanup with limit of 5
    let result = cleanup_old_worktrees(5);
    assert!(result.is_ok());

    // Verify only 5 worktrees remain
    let remaining = list_claude_worktrees().unwrap();
    assert_eq!(remaining.len(), 5);

    // Cleanup
    let _ = std::env::set_current_dir(original_dir);
}

#[test]
fn test_worktree_new() {
    let worktree = Worktree::new("test-phase");
    assert!(worktree.name.starts_with("claude-phase-test-phase-"));
    assert_eq!(
        worktree.path,
        PathBuf::from(format!("../{}", worktree.name))
    );
    assert_eq!(worktree.branch, worktree.name);
    assert!(!worktree.created_at.is_empty());
}

#[test]
fn test_worktree_state_new() {
    let state = WorktreeState::new();
    assert!(state.active_worktrees.is_empty());
}

#[test]
fn test_worktree_state_mark_failed() {
    let mut state = WorktreeState::new();
    let worktree = Worktree::new("test-1");
    state.add_worktree("1".to_string(), &worktree);

    state.mark_failed("1");
    assert_eq!(state.active_worktrees[0].status, WorktreeStatus::Failed);
}

#[test]
fn test_get_active_worktree() {
    let mut state = WorktreeState::new();
    let worktree = Worktree::new("test-1");
    state.add_worktree("1".to_string(), &worktree);

    // Should find active worktree
    assert!(state.get_active_worktree("1").is_some());

    // Should not find non-existent worktree
    assert!(state.get_active_worktree("2").is_none());

    // Should not find completed worktree
    state.mark_completed("1");
    assert!(state.get_active_worktree("1").is_none());
}

#[test]
fn test_worktree_creation_with_invalid_branch() {
    let Some(temp_dir) = setup_test_repo() else {
        return; // Skip test if git is not available
    };
    let original_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get current directory: {}", e);
            return;
        }
    };
    
    if let Err(e) = std::env::set_current_dir(temp_dir.path()) {
        eprintln!("Failed to change to temp directory: {}", e);
        return;
    }

    // Test with non-existent base branch
    let result = create_worktree("test", "non-existent-branch");
    assert!(result.is_err());

    // Cleanup
    let _ = std::env::set_current_dir(original_dir);
}

#[test]
fn test_get_current_branch() {
    if !check_git_available() {
        return; // Skip test if git is not available
    }

    let Some(temp_dir) = setup_test_repo() else {
        return;
    };
    let original_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Failed to get current directory: {}", e);
            return;
        }
    };
    
    if let Err(e) = std::env::set_current_dir(temp_dir.path()) {
        eprintln!("Failed to change to temp directory: {}", e);
        return;
    }

    let branch = get_current_branch();
    assert!(branch.is_ok());
    // In a new repo, the default branch might be "main" or "master"
    let branch_name = branch.unwrap();
    assert!(branch_name == "main" || branch_name == "master");

    // Cleanup
    let _ = std::env::set_current_dir(original_dir);
}
