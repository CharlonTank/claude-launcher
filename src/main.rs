use std::env;
use std::fs;
use std::process::Command;

use claude_launcher::generate_applescript;

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
        // Create the prompt file first
        let prompt_file = format!("{}/agent_prompt_task_{}.txt", &current_dir, i + 1);
        create_prompt_file(&prompt_file, task);
        
        let applescript = generate_applescript(task, &current_dir, &prompt_file, i == 0);
        execute_applescript(&applescript);
    }
}

fn create_prompt_file(file_path: &str, task: &str) {
    let prompt_content = format!(
        "look in todos.md, {}, ONCE YOUR DONE, update todos.md to mark your task as done AND ADD A COMMENT about what you did, any issues encountered, or important notes. IMPORTANT: If you encounter a file that has been modified when you try to modify it, use 'sleep 120' (wait 2 minutes) and try again. CRITICAL: If you are the LAST ONE to mark your todo as complete in the current phase, you TRANSFORM INTO THE PHASE CTO. As the Phase CTO, you must: 1) Review all completed tasks in the phase, 2) Ensure the phase is properly coded and tested, 3) Add a comprehensive comment in the PHASE SECTION of todos.md summarizing what has been done, any issues encountered, and important notes. ULTIMATE: If after marking your phase as complete, ALL PHASES are now marked as DONE, you TRANSFORM INTO THE FINAL CTO. As the Final CTO, you must: 1) Review all phase summaries, 2) Ensure the entire project is properly integrated and tested, 3) Create a final project summary in todos.md with overall status, key achievements, and any remaining considerations. After completing your CTO duties, YOU STOP HERE.",
        task
    );
    
    fs::write(file_path, prompt_content).expect("Failed to write prompt file");
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
