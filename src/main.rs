use std::env;
use std::fs;
use std::process::Command;
use serde::{Deserialize, Serialize};

use claude_launcher::generate_applescript;

#[derive(Serialize, Deserialize, Debug)]
struct TodosFile {
    phases: Vec<Phase>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Phase {
    id: u32,
    name: String,
    steps: Vec<Step>,
    status: String,
    comment: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Step {
    id: String,
    name: String,
    prompt: String,
    status: String,
    comment: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let current_dir = env::current_dir()
        .expect("Failed to get current directory")
        .to_string_lossy()
        .to_string();
    
    // No arguments - auto-detect next tasks
    if args.len() == 1 {
        handle_auto_mode(&current_dir);
        return;
    }
    
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
    
    // Normal execution mode with explicit tasks
    let tasks: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
    
    if tasks.len() > 10 {
        eprintln!("Error: Maximum of 10 tasks allowed");
        std::process::exit(1);
    }
    
    for (i, task) in tasks.iter().enumerate() {
        // Create prompt file first
        let prompt_file = format!("{}/agent_prompt_task_{}.txt", &current_dir, i + 1);
        create_prompt_file(&prompt_file, task);
        
        let applescript = generate_applescript(task, &current_dir, &prompt_file, i == 0);
        execute_applescript(&applescript);
    }
}

fn handle_auto_mode(current_dir: &str) {
    let todos_path = format!("{}/todos.json", current_dir);
    
    // Check if todos.json exists
    if !std::path::Path::new(&todos_path).exists() {
        eprintln!("Error: todos.json does not exist. Run 'claude-launcher --init' first");
        std::process::exit(1);
    }
    
    // Read and parse todos.json
    let contents = fs::read_to_string(&todos_path)
        .expect("Failed to read todos.json");
    
    let todos: TodosFile = serde_json::from_str(&contents)
        .expect("Failed to parse todos.json");
    
    // Find first phase with TODO status
    let todo_phase = todos.phases.iter().find(|phase| phase.status == "TODO");
    
    match todo_phase {
        Some(phase) => {
            // Get all TODO steps in this phase
            let todo_steps: Vec<String> = phase.steps.iter()
                .filter(|step| step.status == "TODO")
                .map(|step| format!("Phase {}, Step {}: {}", phase.id, step.id, step.name))
                .collect();
            
            if todo_steps.is_empty() && phase.status == "TODO" {
                // All steps done but phase not complete - spawn CTO
                println!("🎯 All steps in Phase {} completed! Spawning Phase CTO...", phase.id);
                
                let cto_task = format!("Phase {} CTO: Review and Complete {}", phase.id, phase.name);
                let prompt_file = format!("{}/agent_prompt_cto_phase_{}.txt", current_dir, phase.id);
                create_cto_prompt_file(&prompt_file, phase);
                
                let applescript = generate_applescript(&cto_task, current_dir, &prompt_file, true);
                execute_applescript(&applescript);
                return;
            }
            
            if todo_steps.is_empty() {
                println!("Phase {} is already completed!", phase.id);
                return;
            }
            
            println!("🚀 Auto-launching Phase {}: {}", phase.id, phase.name);
            println!("📋 Running {} tasks in parallel", todo_steps.len());
            
            // Launch the tasks
            for (i, task) in todo_steps.iter().enumerate() {
                let prompt_file = format!("{}/agent_prompt_task_{}.txt", current_dir, i + 1);
                create_prompt_file(&prompt_file, task);
                
                let applescript = generate_applescript(task, current_dir, &prompt_file, i == 0);
                execute_applescript(&applescript);
            }
        }
        None => {
            println!("✅ All phases completed! No TODO tasks found.");
        }
    }
}

fn create_prompt_file(file_path: &str, task: &str) {
    let prompt_content = format!(
        "look in todos.json, {}, ONCE YOUR DONE, update todos.json to mark your task as done (status: \"DONE\") AND ADD A COMMENT in the comment field about what you did, any issues encountered, or important notes. IMPORTANT: If you encounter a file that has been modified when you try to modify it, use sleep 120 (wait 2 minutes) and try again. After completing your task, YOU STOP HERE.",
        task
    );
    
    fs::write(file_path, prompt_content).expect("Failed to write prompt file");
}

fn create_cto_prompt_file(file_path: &str, phase: &Phase) {
    let prompt_content = format!(
        "You are the Phase {} CTO. All tasks in this phase have been completed. Your responsibilities:\n\n\
        1. Review todos.json and verify all steps in Phase {} are properly completed\n\
        2. Check the comments for each step to understand what was done\n\
        3. Run validation commands:\n\
           - First run: `lamdera make src/Frontend.elm src/Backend.elm`\n\
           - Then run: `elm-test-rs --compiler /opt/homebrew/bin/lamdera`\n\
        4. Based on the results:\n\
           - **No errors**: Mark phase status as \"DONE\", add summary comment, call `claude-launcher`, STOP\n\
           - **Few errors (1-5)**: Fix the errors, then mark phase as \"DONE\", add summary, call `claude-launcher`, STOP\n\
           - **Many errors (6+)**: Analyze root cause, create a new remediation phase in todos.json with specific fix tasks, \
             mark current phase as \"DONE\" with comment explaining issues, call `claude-launcher`, STOP\n\
        5. Phase summary comment should include:\n\
           - What was accomplished\n\
           - Any issues encountered and how they were resolved\n\
           - Test results\n\
           - Key achievements\n\n\
        IMPORTANT: You are ONLY reviewing Phase {}. Do not modify other phases or steps.\n\n\
        ULTIMATE: If after marking your phase as complete, ALL PHASES are now marked as DONE, you TRANSFORM INTO THE FINAL CTO. \
        As the Final CTO: Run both validation commands again, ensure everything passes, then create a final project summary. \
        After completing your duties, YOU STOP HERE.",
        phase.id, phase.id, phase.id
    );
    
    fs::write(file_path, prompt_content).expect("Failed to write CTO prompt file");
}

fn handle_init_command(current_dir: &str) {
    let todos_path = format!("{}/todos.json", current_dir);
    
    // Check if todos.json already exists
    if std::path::Path::new(&todos_path).exists() {
        eprintln!("Error: todos.json already exists in this directory");
        eprintln!("Remove it first if you want to create a new one");
        std::process::exit(1);
    }
    
    // Create empty todos.json
    let empty_todos = TodosFile {
        phases: vec![]
    };
    
    let json = serde_json::to_string_pretty(&empty_todos)
        .expect("Failed to serialize todos structure");
    
    fs::write(&todos_path, json).expect("Failed to create todos.json");
    println!("✅ Created todos.json in {}", current_dir);
    println!("📝 Next step: Run 'claude-launcher --create-task \"your requirements\"' to generate task phases");
}

fn handle_create_task_command(current_dir: &str, requirements: &str) {
    let todos_path = format!("{}/todos.json", current_dir);
    
    // Check if todos.json exists
    if !std::path::Path::new(&todos_path).exists() {
        eprintln!("Error: todos.json does not exist. Run 'claude-launcher --init' first");
        std::process::exit(1);
    }
    
    // Create prompt for Claude to analyze requirements and generate phases
    let prompt_file = format!("{}/task_planning_prompt.txt", current_dir);
    let prompt = format!(
        r#"You are a project planning expert. Your task is to analyze the given requirements and create a detailed implementation plan in JSON format.

REQUIREMENTS: {}

Read the existing todos.json file and replace it with a detailed implementation plan with multiple phases. Each phase should contain parallel tasks that can be executed simultaneously by different agents.

IMPORTANT GUIDELINES:
1. Create phases that build upon each other (Phase 1 foundations, Phase 2 features, etc.)
2. Each phase should have 2-5 parallel tasks that don't conflict
3. Be extremely detailed in prompts - include exact file names, function names, and code examples
4. Ensure no two parallel tasks modify the same files
5. Each step id should be like "1A", "1B", "2A", etc.
6. All phases and steps should have status: "TODO" and comment: ""
7. Add specific code examples and expected outputs in the prompt field
8. End each agent prompt with "IMPORTANT: Complete ONLY this specific task. Once finished, STOP."

The JSON structure should be:
{{
  "phases": [
    {{
      "id": 1,
      "name": "Phase Name",
      "steps": [
        {{
          "id": "1A",
          "name": "Task Name",
          "prompt": "Detailed instructions including code examples...",
          "status": "TODO",
          "comment": ""
        }}
      ],
      "status": "TODO",
      "comment": ""
    }}
  ]
}}

CRITICAL: Replace the entire todos.json file with your new implementation plan."#,
        requirements
    );
    
    fs::write(&prompt_file, prompt).expect("Failed to write prompt file");
    
    // Launch Claude to create the task plan
    let applescript = generate_applescript("Task Planning", current_dir, &prompt_file, true);
    execute_applescript(&applescript);
    
    println!("🚀 Launching Claude to analyze requirements and create task phases...");
    println!("📋 Claude will update todos.json with a detailed implementation plan");
    println!("⏳ Once complete, run 'claude-launcher' (no arguments) to start execution");
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