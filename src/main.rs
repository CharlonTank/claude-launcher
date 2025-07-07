use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: claude-launcher \"task1\" [\"task2\" ...]");
        std::process::exit(1);
    }
    
    if args.len() > 11 {
        eprintln!("Error: Maximum of 10 tasks allowed");
        std::process::exit(1);
    }
    
    let current_dir = env::current_dir()
        .expect("Failed to get current directory")
        .to_string_lossy()
        .to_string();
    
    let tasks: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
    
    for (i, task) in tasks.iter().enumerate() {
        let applescript = generate_applescript(task, &current_dir, i == 0);
        execute_applescript(&applescript);
    }
}

fn generate_applescript(task: &str, current_dir: &str, is_first: bool) -> String {
    let escaped_task = task.replace("\"", "\\\"");
    let escaped_dir = current_dir.replace("\"", "\\\"");
    
    if is_first {
        format!(
            r#"
            tell application "iTerm"
                activate
                
                tell current window
                    create tab with default profile
                    tell current session
                        write text "cd \"{}\" && claude --dangerously-skip-permissions \"look in todos.md, {}, ONCE YOUR DONE, update todos.md to mark your task as done AND ADD A COMMENT about what you did, any issues encountered, or important notes. IMPORTANT: If you encounter a file that has been modified when you try to modify it, use 'sleep 120' (wait 2 minutes) and try again. CRITICAL: If you're the LAST ONE to mark your todo as complete in the current phase, you TRANSFORM INTO THE PHASE CTO. As the Phase CTO, you must: 1) Review all completed tasks in the phase, 2) Ensure the phase is properly coded and tested, 3) Add a comprehensive comment in the PHASE SECTION of todos.md summarizing what has been done, any issues encountered, and important notes. ULTIMATE: If after marking your phase as complete, ALL PHASES are now marked as DONE, you TRANSFORM INTO THE FINAL CTO. As the Final CTO, you must: 1) Review all phase summaries, 2) Ensure the entire project is properly integrated and tested, 3) Create a final project summary in todos.md with overall status, key achievements, and any remaining considerations. After completing your CTO duties, YOU STOP HERE.\""
                    end tell
                end tell
            end tell
            "#,
            escaped_dir, escaped_task
        )
    } else {
        format!(
            r#"
            tell application "iTerm"
                tell current window
                    create tab with default profile
                    tell current session
                        write text "cd \"{}\" && claude --dangerously-skip-permissions \"look in todos.md, {}, ONCE YOUR DONE, update todos.md to mark your task as done AND ADD A COMMENT about what you did, any issues encountered, or important notes. IMPORTANT: If you encounter a file that has been modified when you try to modify it, use 'sleep 120' (wait 2 minutes) and try again. CRITICAL: If you're the LAST ONE to mark your todo as complete in the current phase, you TRANSFORM INTO THE PHASE CTO. As the Phase CTO, you must: 1) Review all completed tasks in the phase, 2) Ensure the phase is properly coded and tested, 3) Add a comprehensive comment in the PHASE SECTION of todos.md summarizing what has been done, any issues encountered, and important notes. ULTIMATE: If after marking your phase as complete, ALL PHASES are now marked as DONE, you TRANSFORM INTO THE FINAL CTO. As the Final CTO, you must: 1) Review all phase summaries, 2) Ensure the entire project is properly integrated and tested, 3) Create a final project summary in todos.md with overall status, key achievements, and any remaining considerations. After completing your CTO duties, YOU STOP HERE.\""
                    end tell
                end tell
            end tell
            "#,
            escaped_dir, escaped_task
        )
    }
}

fn execute_applescript(script: &str) {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .expect("Failed to execute AppleScript");
    
    if !output.status.success() {
        eprintln!("AppleScript error: {}", String::from_utf8_lossy(&output.stderr));
    }
}
