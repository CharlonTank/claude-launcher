pub fn generate_applescript(task: &str, current_dir: &str, is_first: bool) -> String {
    // Build the full prompt
    let prompt = format!(
        "look in todos.md, {}, ONCE YOUR DONE, update todos.md to mark your task as done AND ADD A COMMENT about what you did, any issues encountered, or important notes. IMPORTANT: If you encounter a file that has been modified when you try to modify it, use 'sleep 120' (wait 2 minutes) and try again. CRITICAL: If you are the LAST ONE to mark your todo as complete in the current phase, you TRANSFORM INTO THE PHASE CTO. As the Phase CTO, you must: 1) Review all completed tasks in the phase, 2) Ensure the phase is properly coded and tested, 3) Add a comprehensive comment in the PHASE SECTION of todos.md summarizing what has been done, any issues encountered, and important notes. ULTIMATE: If after marking your phase as complete, ALL PHASES are now marked as DONE, you TRANSFORM INTO THE FINAL CTO. As the Final CTO, you must: 1) Review all phase summaries, 2) Ensure the entire project is properly integrated and tested, 3) Create a final project summary in todos.md with overall status, key achievements, and any remaining considerations. After completing your CTO duties, YOU STOP HERE.",
        task
    );
    
    // Escape the prompt for shell - replace single quotes with '\''
    let escaped_prompt = prompt.replace("'", "'\\''");
    
    // Build simple command
    let shell_command = format!(
        "cd '{}' && claude --dangerously-skip-permissions '{}'",
        current_dir.replace("'", "'\\''"), 
        escaped_prompt
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
        assert!(script.contains("claude --dangerously-skip-permissions"));
        assert!(script.contains("test task"));
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
    fn test_escape_single_quotes() {
        let script = generate_applescript("task with 'quotes'", "/test/dir", true);
        
        assert!(script.contains("task with '\\''quotes'\\''"));
    }
}