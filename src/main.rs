use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::process::Command;

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

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    name: String,
    agent: AgentConfig,
    cto: CtoConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct AgentConfig {
    before_stop_commands: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CtoConfig {
    validation_commands: Vec<ValidationCommand>,
    few_errors_max: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct ValidationCommand {
    command: String,
    description: String,
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

    // Show help if requested
    if args[1] == "--help" || args[1] == "-h" {
        println!("Usage:");
        println!("  claude-launcher                    Auto-launch next TODO phase (parallel)");
        println!("  claude-launcher --step-by-step     Run tasks one at a time (sequential)");
        println!("  claude-launcher --init             Create .claude-launcher/ with empty config");
        println!(
            "  claude-launcher --init-lamdera     Create .claude-launcher/ with Lamdera preset"
        );
        println!(
            "  claude-launcher --smart-init       Analyze project and create appropriate config"
        );
        println!("  claude-launcher --create-task \"requirements\"  Generate task phases");
        println!("  claude-launcher \"task1\" \"task2\"    Launch specific tasks");
        std::process::exit(0);
    }

    // Check for special commands
    match args[1].as_str() {
        "--init" => {
            handle_init_command(&current_dir);
            return;
        }
        "--init-lamdera" => {
            handle_init_lamdera_command(&current_dir);
            return;
        }
        "--smart-init" => {
            handle_smart_init_command(&current_dir);
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
        "--step-by-step" => {
            handle_step_by_step_mode(&current_dir);
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
        // For manual task launching, we don't know the phase context, so assume not last phase
        create_prompt_file(&prompt_file, task, false);

        let applescript = generate_applescript(task, &current_dir, &prompt_file, i == 0);
        execute_applescript(&applescript);
    }
}

fn handle_auto_mode(current_dir: &str) {
    let todos_path = format!("{}/.claude-launcher/todos.json", current_dir);

    // Check if todos.json exists
    if !std::path::Path::new(&todos_path).exists() {
        eprintln!(
            "Error: .claude-launcher/todos.json does not exist. Run 'claude-launcher --init' first"
        );
        std::process::exit(1);
    }

    // Read and parse todos.json
    let contents = fs::read_to_string(&todos_path).expect("Failed to read todos.json");

    let todos: TodosFile = serde_json::from_str(&contents).expect("Failed to parse todos.json");

    // Find first phase with TODO status
    let todo_phase = todos.phases.iter().find(|phase| phase.status == "TODO");

    match todo_phase {
        Some(phase) => {
            // Get all TODO steps in this phase
            let todo_steps: Vec<String> = phase
                .steps
                .iter()
                .filter(|step| step.status == "TODO")
                .map(|step| format!("Phase {}, Step {}: {}", phase.id, step.id, step.name))
                .collect();

            if todo_steps.is_empty() && phase.status == "TODO" {
                // All steps done but phase not complete - spawn CTO
                println!(
                    "🎯 All steps in Phase {} completed! Spawning Phase CTO...",
                    phase.id
                );

                let cto_task =
                    format!("Phase {} CTO: Review and Complete {}", phase.id, phase.name);
                let prompt_file =
                    format!("{}/agent_prompt_cto_phase_{}.txt", current_dir, phase.id);
                // Check if this is the last TODO phase
                let is_last_phase = todos.phases.iter().filter(|p| p.status == "TODO").count() == 1;
                create_cto_prompt_file(&prompt_file, phase, false, is_last_phase); // false = not step-by-step mode

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

            // Check if this is the last TODO phase
            let is_last_phase = todos.phases.iter().filter(|p| p.status == "TODO").count() == 1;

            // Launch the tasks
            for (i, task) in todo_steps.iter().enumerate() {
                let prompt_file = format!("{}/agent_prompt_task_{}.txt", current_dir, i + 1);
                create_prompt_file(&prompt_file, task, is_last_phase);

                let applescript = generate_applescript(task, current_dir, &prompt_file, i == 0);
                execute_applescript(&applescript);
            }
        }
        None => {
            println!("✅ All phases completed! No TODO tasks found.");
        }
    }
}

fn handle_step_by_step_mode(current_dir: &str) {
    let todos_path = format!("{}/.claude-launcher/todos.json", current_dir);

    // Check if todos.json exists
    if !std::path::Path::new(&todos_path).exists() {
        eprintln!(
            "Error: .claude-launcher/todos.json does not exist. Run 'claude-launcher --init' first"
        );
        std::process::exit(1);
    }

    // Read and parse todos.json
    let contents = fs::read_to_string(&todos_path).expect("Failed to read todos.json");

    let todos: TodosFile = serde_json::from_str(&contents).expect("Failed to parse todos.json");

    // Find first phase with TODO status
    let todo_phase = todos.phases.iter().find(|phase| phase.status == "TODO");

    match todo_phase {
        Some(phase) => {
            // Get first TODO step in this phase
            let first_todo_step = phase
                .steps
                .iter()
                .find(|step| step.status == "TODO")
                .map(|step| format!("Phase {}, Step {}: {}", phase.id, step.id, step.name));

            match first_todo_step {
                Some(task) => {
                    println!("🚶 Step-by-step mode: Phase {}: {}", phase.id, phase.name);
                    println!("📋 Running next task: {}", task);

                    // Check if this is the last TODO phase
                    let is_last_phase =
                        todos.phases.iter().filter(|p| p.status == "TODO").count() == 1;

                    // Launch just the first task
                    let prompt_file = format!("{}/agent_prompt_task_step.txt", current_dir);
                    create_step_by_step_prompt_file(&prompt_file, &task, is_last_phase);

                    let applescript = generate_applescript(&task, current_dir, &prompt_file, true);
                    execute_applescript(&applescript);
                }
                None => {
                    // All steps done but phase not complete - spawn CTO
                    println!(
                        "🎯 All steps in Phase {} completed! Spawning Phase CTO...",
                        phase.id
                    );

                    let cto_task =
                        format!("Phase {} CTO: Review and Complete {}", phase.id, phase.name);
                    let prompt_file =
                        format!("{}/agent_prompt_cto_phase_{}.txt", current_dir, phase.id);
                    // Check if this is the last TODO phase
                    let is_last_phase =
                        todos.phases.iter().filter(|p| p.status == "TODO").count() == 1;
                    create_cto_prompt_file(&prompt_file, phase, true, is_last_phase); // true = step-by-step mode

                    let applescript =
                        generate_applescript(&cto_task, current_dir, &prompt_file, true);
                    execute_applescript(&applescript);
                }
            }
        }
        None => {
            println!("✅ All phases completed! No TODO tasks found.");
        }
    }
}

fn create_prompt_file(file_path: &str, task: &str, is_last_phase: bool) {
    // Load config to get validation commands
    let current_dir = env::current_dir()
        .expect("Failed to get current directory")
        .to_string_lossy()
        .to_string();

    let config = load_config(&current_dir);

    let validation_commands = if let Some(cfg) = &config {
        if cfg.cto.validation_commands.is_empty() {
            String::from("validation commands configured in .claude-launcher/config.json")
        } else {
            cfg.cto
                .validation_commands
                .iter()
                .map(|cmd| format!("`{}`", cmd.command))
                .collect::<Vec<_>>()
                .join(" and ")
        }
    } else {
        String::from("`lamdera make src/Frontend.elm src/Backend.elm` and `elm-test-rs --compiler /opt/homebrew/bin/lamdera`")
    };

    let few_errors_max = config.as_ref().map(|c| c.cto.few_errors_max).unwrap_or(5);

    let prompt_content = format!(
        "FIRST: Read .claude-launcher/todos.json and analyze:\n\
        1. Comments from all completed steps in the current phase to understand what has been done\n\
        2. Comments from prior phases to understand the project context\n\
        3. Pay special attention to any issues or fixes mentioned\n\n\
        THEN: Complete your task: {}\n\n\
        ONCE YOUR DONE: Update .claude-launcher/todos.json to mark your task as done (status: \"DONE\") AND ADD A COMMENT in the comment field about what you did, any issues encountered, or important notes.\n\n\
        IMPORTANT: If you encounter a file that has been modified when you try to modify it, use sleep 120 (wait 2 minutes) and try again.\n\n\
        CRITICAL: If you are the LAST ONE to mark your todo as complete in the current phase, you TRANSFORM INTO THE PHASE CTO. As the Phase CTO, you must:\n\
        1) Review all completed tasks in the phase\n\
        2) Run validation commands: {}\n\
        3) Based on results:\n\
           - No errors: Mark phase as \"DONE\", add summary, call `claude-launcher`\n\
           - Few errors (1-{}): Fix them, mark phase as \"DONE\", call `claude-launcher`\n\
           - Many errors ({}+): Create remediation phase, mark current phase \"DONE\", call `claude-launcher`\n\
        4) Add comprehensive phase comment{}",
        task, validation_commands, few_errors_max, few_errors_max + 1,
        if is_last_phase {
            "\n\n\
        ULTIMATE: If after marking your phase as complete, ALL PHASES are now marked as DONE, you TRANSFORM INTO THE FINAL CTO. As the Final CTO: \
        Run validation commands, ensure everything passes, create final project summary. After completing your duties, YOU STOP HERE."
        } else {
            ""
        }
    );

    fs::write(file_path, prompt_content).expect("Failed to write prompt file");
}

fn create_step_by_step_prompt_file(file_path: &str, task: &str, is_last_phase: bool) {
    // Load config to get validation commands
    let current_dir = env::current_dir()
        .expect("Failed to get current directory")
        .to_string_lossy()
        .to_string();

    let config = load_config(&current_dir);

    let validation_commands = if let Some(cfg) = &config {
        if cfg.cto.validation_commands.is_empty() {
            String::from("validation commands configured in .claude-launcher/config.json")
        } else {
            cfg.cto
                .validation_commands
                .iter()
                .map(|cmd| format!("`{}`", cmd.command))
                .collect::<Vec<_>>()
                .join(" and ")
        }
    } else {
        String::from("`lamdera make src/Frontend.elm src/Backend.elm` and `elm-test-rs --compiler /opt/homebrew/bin/lamdera`")
    };

    let few_errors_max = config.as_ref().map(|c| c.cto.few_errors_max).unwrap_or(5);

    let prompt_content = format!(
        "FIRST: Read .claude-launcher/todos.json and analyze:\n\
        1. Comments from all completed steps in the current phase to understand what has been done\n\
        2. Comments from prior phases to understand the project context\n\
        3. Pay special attention to any issues or fixes mentioned\n\n\
        THEN: Complete your task: {}\n\n\
        ONCE YOUR DONE: Update .claude-launcher/todos.json to mark your task as done (status: \"DONE\") AND ADD A COMMENT in the comment field about what you did, any issues encountered, or important notes.\n\n\
        IMPORTANT: If you encounter a file that has been modified when you try to modify it, use sleep 120 (wait 2 minutes) and try again.\n\n\
        CRITICAL: If you are the LAST ONE to mark your todo as complete in the current phase, you TRANSFORM INTO THE PHASE CTO. As the Phase CTO:\n\
        1) Review all completed tasks in the phase\n\
        2) Run validation commands: {}\n\
        3) Based on results:\n\
           - No errors: Mark phase as \"DONE\", add summary, call `claude-launcher --step-by-step`\n\
           - Few errors (1-{}): Fix them, mark phase as \"DONE\", call `claude-launcher --step-by-step`\n\
           - Many errors ({}+): Create remediation phase, mark current phase \"DONE\", call `claude-launcher --step-by-step`\n\
        4) Add comprehensive phase comment\n\n\
        OTHERWISE: If NOT the last task, call `claude-launcher --step-by-step` to continue with the next task.{}",
        task, validation_commands, few_errors_max, few_errors_max + 1,
        if is_last_phase {
            "\n\n\
        ULTIMATE: If after marking your phase as complete, ALL PHASES are now marked as DONE, you TRANSFORM INTO THE FINAL CTO. As the Final CTO: \
        Run validation commands, ensure everything passes, create final project summary. After completing your duties, YOU STOP HERE."
        } else {
            ""
        }
    );

    fs::write(file_path, prompt_content).expect("Failed to write step-by-step prompt file");
}

fn load_config(current_dir: &str) -> Option<Config> {
    let config_path = format!("{}/.claude-launcher/config.json", current_dir);

    if let Ok(contents) = fs::read_to_string(&config_path) {
        serde_json::from_str(&contents).ok()
    } else {
        None
    }
}

fn create_cto_prompt_file(
    file_path: &str,
    phase: &Phase,
    step_by_step_mode: bool,
    is_last_phase: bool,
) {
    let launcher_command = if step_by_step_mode {
        "claude-launcher --step-by-step"
    } else {
        "claude-launcher"
    };

    // Load config to get validation commands
    let current_dir = env::current_dir()
        .expect("Failed to get current directory")
        .to_string_lossy()
        .to_string();

    let config = load_config(&current_dir);

    let validation_section = if let Some(cfg) = &config {
        if cfg.cto.validation_commands.is_empty() {
            String::from("3. No validation commands configured\n")
        } else {
            let commands = cfg
                .cto
                .validation_commands
                .iter()
                .map(|cmd| format!("           - {}: `{}`", cmd.description, cmd.command))
                .collect::<Vec<_>>()
                .join("\n");
            format!("3. Run validation commands:\n{}\n", commands)
        }
    } else {
        String::from(
            "3. Run validation commands:\n\
           - First run: `lamdera make src/Frontend.elm src/Backend.elm`\n\
           - Then run: `elm-test-rs --compiler /opt/homebrew/bin/lamdera`\n",
        )
    };

    let few_errors_max = config.as_ref().map(|c| c.cto.few_errors_max).unwrap_or(5);

    let ultimate_section = if is_last_phase {
        "\n\n\
        ULTIMATE: If after marking your phase as complete, ALL PHASES are now marked as DONE, you TRANSFORM INTO THE FINAL CTO. \
        As the Final CTO: Run validation commands again, ensure everything passes, then create a final project summary. \
        After completing your duties, YOU STOP HERE."
    } else {
        ""
    };

    let prompt_content = format!(
        "You are the Phase {} CTO. All tasks in this phase have been completed. Your responsibilities:\n\n\
        1. Review .claude-launcher/todos.json and verify all steps in Phase {} are properly completed\n\
        2. Check the comments for each step to understand what was done\n\
        {}4. Based on the results:\n\
           - **No errors**: Mark phase status as \"DONE\", add summary comment, call `{}`, STOP\n\
           - **Few errors (1-{})**: Fix the errors, then mark phase as \"DONE\", add summary, call `{}`, STOP\n\
           - **Many errors ({}+)**: Analyze root cause, create a new remediation phase in .claude-launcher/todos.json with specific fix tasks, \
             mark current phase as \"DONE\" with comment explaining issues, call `{}`, STOP\n\
        5. Phase summary comment should include:\n\
           - What was accomplished\n\
           - Any issues encountered and how they were resolved\n\
           - Test results\n\
           - Key achievements\n\n\
        IMPORTANT: You are ONLY reviewing Phase {}. Do not modify other phases or steps.{}",
        phase.id, phase.id, validation_section, launcher_command, few_errors_max, launcher_command, few_errors_max + 1, launcher_command, phase.id, ultimate_section
    );

    fs::write(file_path, prompt_content).expect("Failed to write CTO prompt file");
}

fn handle_init_command(current_dir: &str) {
    let launcher_dir = format!("{}/.claude-launcher", current_dir);
    let todos_path = format!("{}/todos.json", launcher_dir);
    let config_path = format!("{}/config.json", launcher_dir);

    // Create .claude-launcher directory if it doesn't exist
    if !std::path::Path::new(&launcher_dir).exists() {
        fs::create_dir(&launcher_dir).expect("Failed to create .claude-launcher directory");
    }

    // Check if todos.json already exists
    if std::path::Path::new(&todos_path).exists() {
        eprintln!("Error: .claude-launcher/todos.json already exists in this directory");
        eprintln!("Remove it first if you want to create a new one");
        std::process::exit(1);
    }

    // Create empty todos.json
    let empty_todos = TodosFile { phases: vec![] };

    let json =
        serde_json::to_string_pretty(&empty_todos).expect("Failed to serialize todos structure");

    fs::write(&todos_path, json).expect("Failed to create todos.json");

    // Create empty config.json
    let empty_config = r#"{
  "name": "Project",
  "agent": {
    "before_stop_commands": []
  },
  "cto": {
    "validation_commands": [],
    "few_errors_max": 5
  }
}"#;

    fs::write(&config_path, empty_config).expect("Failed to create config.json");

    println!("✅ Created .claude-launcher/ directory with todos.json and config.json");
    println!("📝 Next step: Run 'claude-launcher --create-task \"your requirements\"' to generate task phases");
    println!("💡 Or run 'claude-launcher --init-lamdera' to create a Lamdera project setup");
}

fn handle_init_lamdera_command(current_dir: &str) {
    let launcher_dir = format!("{}/.claude-launcher", current_dir);
    let todos_path = format!("{}/todos.json", launcher_dir);
    let config_path = format!("{}/config.json", launcher_dir);

    // Create .claude-launcher directory if it doesn't exist
    if !std::path::Path::new(&launcher_dir).exists() {
        fs::create_dir(&launcher_dir).expect("Failed to create .claude-launcher directory");
    }

    // Check if todos.json already exists
    if std::path::Path::new(&todos_path).exists() {
        eprintln!("Error: .claude-launcher/todos.json already exists in this directory");
        eprintln!("Remove it first if you want to create a new one");
        std::process::exit(1);
    }

    // Create empty todos.json
    let empty_todos = TodosFile { phases: vec![] };

    let json =
        serde_json::to_string_pretty(&empty_todos).expect("Failed to serialize todos structure");

    fs::write(&todos_path, json).expect("Failed to create todos.json");

    // Create Lamdera config.json
    let lamdera_config = r#"{
  "name": "Lamdera Project",
  "agent": {
    "before_stop_commands": []
  },
  "cto": {
    "validation_commands": [
      {
        "command": "lamdera make src/Frontend.elm src/Backend.elm",
        "description": "Compile Lamdera project"
      },
      {
        "command": "elm-test-rs --compiler /opt/homebrew/bin/lamdera",
        "description": "Run tests with Lamdera compiler"
      }
    ],
    "few_errors_max": 5
  }
}"#;

    fs::write(&config_path, lamdera_config).expect("Failed to create config.json");

    println!("✅ Created .claude-launcher/ directory with Lamdera preset");
    println!("🔧 Config includes lamdera make and elm-test-rs validation commands");
    println!("📝 Next step: Run 'claude-launcher --create-task \"your requirements\"' to generate task phases");
}

fn handle_smart_init_command(current_dir: &str) {
    let launcher_dir = format!("{}/.claude-launcher", current_dir);
    let todos_path = format!("{}/todos.json", launcher_dir);

    // Create .claude-launcher directory if it doesn't exist
    if !std::path::Path::new(&launcher_dir).exists() {
        fs::create_dir(&launcher_dir).expect("Failed to create .claude-launcher directory");
    }

    // Create empty todos.json only if it doesn't exist
    if !std::path::Path::new(&todos_path).exists() {
        let empty_todos = TodosFile { phases: vec![] };

        let json = serde_json::to_string_pretty(&empty_todos)
            .expect("Failed to serialize todos structure");

        fs::write(&todos_path, json).expect("Failed to create todos.json");
    }

    // Create prompt for Claude to analyze project and generate appropriate config
    let prompt_file = format!("{}/smart_init_prompt.txt", current_dir);
    let prompt = r#"Analyze the current project directory and create an appropriate config.json for claude-launcher.

TASK: 
1. Look for common project files (package.json, Cargo.toml, elm.json, requirements.txt, etc.)
2. Identify the primary programming language(s) and frameworks
3. Detect build tools, test frameworks, and common commands
4. Check for existing scripts in package.json, Makefile, or similar
5. Create a .claude-launcher/config.json with appropriate validation commands

IMPORTANT: The config.json should have this structure:
{
  "name": "Project Name",
  "agent": {
    "before_stop_commands": []
  },
  "cto": {
    "validation_commands": [
      {
        "command": "actual command to run",
        "description": "What this command does"
      }
    ],
    "few_errors_max": 5
  }
}

Common patterns to look for:
- Node.js: npm test, npm run lint, npm run typecheck, npm run build
- Python: pytest, mypy, flake8, black --check
- Rust: cargo test, cargo clippy, cargo fmt -- --check
- Elm/Lamdera: lamdera make, elm-test-rs
- Ruby: rspec, rubocop

CRITICAL: Write the config to .claude-launcher/config.json

After creating the config, output a summary of what was detected and configured."#;

    fs::write(&prompt_file, prompt).expect("Failed to write prompt file");

    // Launch Claude to analyze project and create config
    let applescript = generate_applescript("Smart Init", current_dir, &prompt_file, true);
    execute_applescript(&applescript);

    println!("🔍 Launching Claude to analyze your project...");
    println!("📋 Claude will create an appropriate .claude-launcher/config.json");
    println!("⏳ Once complete, run 'claude-launcher --create-task \"your requirements\"'");
}

fn handle_create_task_command(current_dir: &str, requirements: &str) {
    let todos_path = format!("{}/.claude-launcher/todos.json", current_dir);

    // Check if todos.json exists
    if !std::path::Path::new(&todos_path).exists() {
        eprintln!(
            "Error: .claude-launcher/todos.json does not exist. Run 'claude-launcher --init' first"
        );
        std::process::exit(1);
    }

    // Create prompt for Claude to analyze requirements and generate phases
    let prompt_file = format!("{}/task_planning_prompt.txt", current_dir);
    let prompt = format!(
        r#"You are a project planning expert. Your task is to analyze the given requirements and create a detailed implementation plan in JSON format.

REQUIREMENTS: {}

Read the existing .claude-launcher/todos.json file and replace it with a detailed implementation plan with multiple phases. Each phase should contain parallel tasks that can be executed simultaneously by different agents.

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

CRITICAL: Replace the entire .claude-launcher/todos.json file with your new implementation plan."#,
        requirements
    );

    fs::write(&prompt_file, prompt).expect("Failed to write prompt file");

    // Launch Claude to create the task plan
    let applescript = generate_applescript("Task Planning", current_dir, &prompt_file, true);
    execute_applescript(&applescript);

    println!("🚀 Launching Claude to analyze requirements and create task phases...");
    println!(
        "📋 Claude will update .claude-launcher/todos.json with a detailed implementation plan"
    );
    println!("⏳ Once complete, run 'claude-launcher' (no arguments) to start execution");
}

fn execute_applescript(script: &str) {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .expect("Failed to execute AppleScript");

    if !output.status.success() {
        eprintln!(
            "AppleScript error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
