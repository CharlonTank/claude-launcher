pub fn generate_applescript(_task: &str, current_dir: &str, prompt_file: &str, is_first: bool) -> String {
    // Read the prompt file content and pass as argument
    let shell_command = format!(
        "cd {} && claude --dangerously-skip-permissions \"$(cat {})\" && rm {}",
        current_dir, prompt_file, prompt_file
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
        let script = generate_applescript("test task", "/test/dir", "/test/dir/agent_prompt_task_1.txt", true);
        
        assert!(script.contains("tell application \"iTerm\""));
        assert!(script.contains("activate"));
        assert!(script.contains("create tab with default profile"));
        assert!(script.contains("cd /test/dir"));
        assert!(script.contains("claude --dangerously-skip-permissions \"$(cat /test/dir/agent_prompt_task_1.txt)\""));
        assert!(script.contains("rm /test/dir/agent_prompt_task_1.txt"));
    }

    #[test]
    fn test_generate_applescript_additional_tab() {
        let script = generate_applescript("another task", "/test/dir", "/test/dir/agent_prompt_task_2.txt", false);
        
        assert!(script.contains("tell application \"iTerm\""));
        assert!(!script.contains("activate")); // Not first tab
        assert!(script.contains("create tab with default profile"));
        assert!(script.contains("claude --dangerously-skip-permissions \"$(cat /test/dir/agent_prompt_task_2.txt)\""));
    }

    #[test]
    fn test_command_structure() {
        let script = generate_applescript("test", "/work/dir", "/work/dir/agent_prompt_task_1.txt", true);
        
        assert!(script.contains("cd /work/dir && claude --dangerously-skip-permissions \"$(cat /work/dir/agent_prompt_task_1.txt)\" && rm /work/dir/agent_prompt_task_1.txt"));
    }
}