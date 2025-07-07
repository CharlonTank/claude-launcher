pub fn generate_applescript(task: &str, current_dir: &str, is_first: bool) -> String {
    // Escape single quotes for the bash command
    let escaped_task = task.replace("'", "'\\''");
    let escaped_dir = current_dir.replace("'", "'\\''");
    
    // Use a simpler approach with single quotes to avoid complex escaping
    let shell_command = format!(
        "cd '{}' && claude --dangerously-skip-permissions 'look in todos.md, {}, ONCE YOUR DONE, update todos.md to mark your task as done AND ADD A COMMENT about what you did, any issues encountered, or important notes. IMPORTANT: If you encounter a file that has been modified when you try to modify it, use sleep 120 (wait 2 minutes) and try again. CRITICAL: If you are the LAST ONE to mark your todo as complete in the current phase, you TRANSFORM INTO THE PHASE CTO. As the Phase CTO, you must: 1) Review all completed tasks in the phase, 2) Ensure the phase is properly coded and tested, 3) Add a comprehensive comment in the PHASE SECTION of todos.md summarizing what has been done, any issues encountered, and important notes. ULTIMATE: If after marking your phase as complete, ALL PHASES are now marked as DONE, you TRANSFORM INTO THE FINAL CTO. As the Final CTO, you must: 1) Review all phase summaries, 2) Ensure the entire project is properly integrated and tested, 3) Create a final project summary in todos.md with overall status, key achievements, and any remaining considerations. After completing your CTO duties, YOU STOP HERE.'",
        escaped_dir, escaped_task
    );
    
    if is_first {
        format!(
            r#"
            tell application "iTerm"
                activate
                
                tell current window
                    create tab with default profile
                    tell current session
                        write text "{}"
                    end tell
                end tell
            end tell
            "#,
            shell_command
        )
    } else {
        format!(
            r#"
            tell application "iTerm"
                tell current window
                    create tab with default profile
                    tell current session
                        write text "{}"
                    end tell
                end tell
            end tell
            "#,
            shell_command
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_applescript_first_tab() {
        let script = generate_applescript("test task", "/test/dir", true);
        
        assert!(script.contains("tell application \"iTerm\""));
        assert!(script.contains("activate"));
        assert!(script.contains("create tab with default profile"));
        assert!(script.contains("cd '/test/dir'"));
        assert!(script.contains("test task"));
        assert!(script.contains("look in todos.md"));
    }

    #[test]
    fn test_generate_applescript_additional_tab() {
        let script = generate_applescript("another task", "/test/dir", false);
        
        assert!(script.contains("tell application \"iTerm\""));
        assert!(!script.contains("activate")); // Not first tab
        assert!(script.contains("create tab with default profile"));
        assert!(script.contains("another task"));
    }

    #[test]
    fn test_escape_single_quotes_in_task() {
        let script = generate_applescript("task with 'quotes'", "/test/dir", true);
        
        // Single quotes are escaped as '\''
        assert!(script.contains("task with '\\''quotes'\\''"));
    }

    #[test]
    fn test_escape_single_quotes_in_directory() {
        let script = generate_applescript("test task", "/path/with'quote", true);
        
        // The path will be escaped properly for single quotes
        assert!(script.contains("/path/with'\\''quote"));
    }

    #[test]
    fn test_cto_instructions_included() {
        let script = generate_applescript("test", "/test", true);
        
        assert!(script.contains("TRANSFORM INTO THE PHASE CTO"));
        assert!(script.contains("TRANSFORM INTO THE FINAL CTO"));
        assert!(script.contains("Review all completed tasks"));
        assert!(script.contains("Create a final project summary"));
    }

    #[test]
    fn test_file_conflict_instructions() {
        let script = generate_applescript("test", "/test", true);
        
        assert!(script.contains("sleep 120"));
        assert!(script.contains("wait 2 minutes"));
    }
}