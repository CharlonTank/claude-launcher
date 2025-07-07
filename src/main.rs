use std::env;
use std::fs;
use std::process::Command;

use claude_launcher::generate_applescript;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  claude-launcher --init                     Create todos.md template");
        eprintln!("  claude-launcher --create-task \"requirements\"  Generate task phases");
        eprintln!("  claude-launcher [--recursive] \"task1\" ...  Launch agents");
        eprintln!("Options:");
        eprintln!("  --recursive: Enable recursive mode where Phase CTOs can spawn new phases");
        std::process::exit(1);
    }
    
    let current_dir = env::current_dir()
        .expect("Failed to get current directory")
        .to_string_lossy()
        .to_string();
    
    // Check for special commands
    match args[1].as_str() {
        "--init" => {
            handle_init_command(&current_dir);
            return;
        }
        "--create-task" => {
            if args.len() < 3 {
                eprintln!("Error: --create-task requires requirements");
                eprintln!("Usage: claude-launcher --create-task \"what you want to build\"");
                std::process::exit(1);
            }
            handle_create_task_command(&current_dir, &args[2]);
            return;
        }
        _ => {}
    }
    
    // Normal execution mode
    let recursive_mode = args[1] == "--recursive";
    let task_start_index = if recursive_mode { 2 } else { 1 };
    
    if args.len() <= task_start_index {
        eprintln!("Error: No tasks provided");
        eprintln!("Usage: claude-launcher [--recursive] \"task1\" [\"task2\" ...]");
        std::process::exit(1);
    }
    
    if args.len() - task_start_index > 10 {
        eprintln!("Error: Maximum of 10 tasks allowed");
        std::process::exit(1);
    }
    
    let tasks: Vec<&str> = args[task_start_index..].iter().map(|s| s.as_str()).collect();
    
    for (i, task) in tasks.iter().enumerate() {
        // Create prompt file first
        let prompt_file = format!("{}/agent_prompt_task_{}.txt", &current_dir, i + 1);
        create_prompt_file(&prompt_file, task, recursive_mode);
        
        let applescript = generate_applescript(task, &current_dir, &prompt_file, i == 0);
        execute_applescript(&applescript);
    }
}

fn create_prompt_file(file_path: &str, task: &str, recursive_mode: bool) {
    // Write ONLY the prompt content, not the flags
    let recursive_instructions = if recursive_mode {
        "RECURSIVE MODE ENABLED: As the Phase CTO, after completing your phase review, you MUST check if there are more phases to execute. If there are, spawn new agents for the next phase by running: claude-launcher --recursive \"Next Phase Task 1\" \"Next Phase Task 2\" etc. The FINAL CTO should check if any phases were missed or if additional work is needed. If so, create a new phase and spawn agents using claude-launcher --recursive."
    } else {
        ""
    };
    
    let prompt_content = format!(
        "look in todos.md, {}, ONCE YOUR DONE, update todos.md to mark your task as done AND ADD A COMMENT about what you did, any issues encountered, or important notes. IMPORTANT: If you encounter a file that has been modified when you try to modify it, use sleep 120 (wait 2 minutes) and try again. CRITICAL: If you are the LAST ONE to mark your todo as complete in the current phase, you TRANSFORM INTO THE PHASE CTO. As the Phase CTO, you must: 1) Review all completed tasks in the phase, 2) Ensure the phase is properly coded and tested, 3) Add a comprehensive comment in the PHASE SECTION of todos.md summarizing what has been done, any issues encountered, and important notes. {} ULTIMATE: If after marking your phase as complete, ALL PHASES are now marked as DONE, you TRANSFORM INTO THE FINAL CTO. As the Final CTO, you must: 1) Review all phase summaries, 2) Ensure the entire project is properly integrated and tested, 3) Create a final project summary in todos.md with overall status, key achievements, and any remaining considerations. After completing your CTO duties, YOU STOP HERE.",
        task,
        recursive_instructions
    );
    
    fs::write(file_path, prompt_content).expect("Failed to write prompt file");
}


fn handle_init_command(current_dir: &str) {
    let todos_path = format!("{}/todos.md", current_dir);
    
    // Check if todos.md already exists
    if std::path::Path::new(&todos_path).exists() {
        eprintln!("Error: todos.md already exists in this directory");
        eprintln!("Remove it first if you want to create a new one");
        std::process::exit(1);
    }
    
    // Create todos.md with the template
    let template = r#"# Parallel Agent Planning Guidelines

## Overview

When creating plans for AI agent execution, format each plan with parallel-executable steps that multiple agents can work on simultaneously without conflicts.

## Required Components for Each Step

1. **Step description** - Clear explanation of what needs to be accomplished
2. **Agent prompt** - Detailed instructions including explicit STOP command
3. **Code examples** - Concrete implementation snippets or pseudocode
4. **Dependencies** - Which steps can run in parallel vs. sequential requirements
5. **Expected outputs** - Define successful completion criteria
6. **Isolation boundaries** - Files, functions, or resources to prevent conflicts

## Conflict Prevention Strategy

- Assign each agent to separate files/modules
- Define clear ownership of functions, classes, or components
- Use unique naming conventions per agent (e.g., Agent1: `user_*`, Agent2: `auth_*`)
- Specify exact file paths ensuring no overlapping modifications
- Create clear interfaces between components

## Task Status Legend

- **[TODO]**: Task is available to be picked up
- **[IN PROGRESS]**: A Claude is currently working on this task
- **[DONE]**: Task has been completed

## Template Format

```
Step 1A [TODO] (Parallel): [Task Name]
- Prompt: "[Detailed instructions]
  IMPORTANT: Complete ONLY this specific task. Once finished, STOP.
  Do not proceed to other tasks or create additional features."
- Owns files: [list of files this agent will create/modify]
- Code: ```language
  // File: path/to/file.ext
  // Implementation example
  ```

- Output: [Expected deliverable]
- No conflicts: [Isolation guarantee]

Step 1B [TODO] (Parallel): [Task Name]

- Prompt: "[Detailed instructions]
  IMPORTANT: Complete ONLY this specific task. Once finished, STOP."
- Owns files: [list of files]
- Code: ```language
  // Implementation

  ```
- Output: [Expected deliverable]
- No conflicts: [Isolation guarantee]

Step 2 [TODO] (Depends on 1A & 1B): [Task Name]

- [Same structure as above]

```

## Key Principles

1. **Be extensive and explicit** - More detail enables better autonomous execution
2. **Enforce task boundaries** - Each prompt must include STOP instruction
3. **Prevent conflicts** - Clear file/resource ownership per agent
4. **Enable parallelism** - Identify truly independent tasks
5. **Define interfaces** - Clear contracts between components
6. **Use bash wait for dependencies** - When agents need to wait for prior tasks to complete, they must use the bash `wait` function to ensure proper synchronization

## Important: Synchronization Between Agents

When an agent depends on another agent's work, include in their prompt:
```bash
# Wait for previous agent to complete their task
wait
# Then proceed with dependent work
```

This ensures proper task ordering and prevents race conditions when parallel agents have dependencies.

This approach maximizes efficiency through parallel execution while maintaining code quality and preventing merge conflicts.

# Project Implementation Plan

<!-- Your project phases will be added here by claude-launcher --create-task -->
"#;
    
    fs::write(&todos_path, template).expect("Failed to create todos.md");
    println!("✅ Created todos.md template in {}", current_dir);
    println!("📝 Next step: Run 'claude-launcher --create-task \"your requirements\"' to generate task phases");
}

fn handle_create_task_command(current_dir: &str, requirements: &str) {
    let todos_path = format!("{}/todos.md", current_dir);
    
    // Check if todos.md exists
    if !std::path::Path::new(&todos_path).exists() {
        eprintln!("Error: todos.md does not exist. Run 'claude-launcher --init' first");
        std::process::exit(1);
    }
    
    // Create prompt for Claude to analyze requirements and generate phases
    let prompt_file = format!("{}/task_planning_prompt.txt", current_dir);
    let prompt = format!(
        r#"You are a project planning expert. Your task is to analyze the given requirements and create a detailed implementation plan following the Parallel Agent Planning Guidelines format.

REQUIREMENTS: {}

Read the existing todos.md file and append a detailed implementation plan with multiple phases. Each phase should contain parallel tasks that can be executed simultaneously by different agents.

IMPORTANT GUIDELINES:
1. Create phases that build upon each other (Phase 1 foundations, Phase 2 features, etc.)
2. Each phase should have 2-5 parallel tasks that don't conflict
3. Be extremely detailed in prompts - include exact file names, function names, and code examples
4. Ensure no two parallel tasks modify the same files
5. Use the exact format shown in the template with Step 1A, 1B, 2A, etc.
6. Include dependencies between phases
7. Add specific code examples and expected outputs
8. End each agent prompt with "IMPORTANT: Complete ONLY this specific task. Once finished, STOP."

After generating the plan, append it to todos.md under the Project Implementation Plan section.

CRITICAL: Only modify todos.md by appending your implementation plan. Do not change the template section."#,
        requirements
    );
    
    fs::write(&prompt_file, prompt).expect("Failed to write prompt file");
    
    // Launch Claude to create the task plan
    let applescript = generate_applescript("Task Planning", current_dir, &prompt_file, true);
    execute_applescript(&applescript);
    
    println!("🚀 Launching Claude to analyze requirements and create task phases...");
    println!("📋 Claude will update todos.md with a detailed implementation plan");
    println!("⏳ Once complete, run 'claude-launcher --recursive' with the generated tasks");
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
