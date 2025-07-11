use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::process::Command;

use claude_launcher::generate_applescript;

mod git_worktree;

const VERSION: &str = "0.2.0";

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

    #[serde(default = "default_worktree_config")]
    worktree: WorktreeConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct AgentConfig {
    before_stop_commands: Vec<String>,
    
    #[serde(default = "default_commands")]
    commands: Vec<CommandConfig>,
    
    #[serde(default = "default_pre_tasks")]
    pre_tasks: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CommandConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    description: String,
    pattern: String,
    use_instead_of: String,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct WorktreeConfig {
    #[serde(default = "default_enabled")]
    enabled: bool,

    #[serde(default = "default_naming_pattern")]
    naming_pattern: String,

    #[serde(default = "default_max_worktrees")]
    max_worktrees: usize,

    #[serde(default = "default_base_branch")]
    base_branch: String,

    #[serde(default = "default_auto_cleanup")]
    auto_cleanup: bool,
}

// Default functions
fn default_worktree_config() -> WorktreeConfig {
    WorktreeConfig {
        enabled: false,
        naming_pattern: "claude-phase-{id}-{timestamp}".to_string(),
        max_worktrees: 5,
        base_branch: "main".to_string(),
        auto_cleanup: true,
    }
}

fn default_enabled() -> bool {
    false
}
fn default_naming_pattern() -> String {
    "claude-phase-{id}-{timestamp}".to_string()
}
fn default_max_worktrees() -> usize {
    5
}
fn default_base_branch() -> String {
    "main".to_string()
}
fn default_auto_cleanup() -> bool {
    true
}

fn default_commands() -> Vec<CommandConfig> {
    vec![]
}

fn default_pre_tasks() -> Vec<String> {
    vec![]
}

// Add cleanup handler for interrupted operations
fn setup_cleanup_handler() {
    ctrlc::set_handler(move || {
        eprintln!("\nInterrupted! Cleaning up...");

        // Try to save current state
        if let Ok(state) = git_worktree::WorktreeState::load() {
            let _ = state.save();
        }

        // Exit gracefully
        std::process::exit(130);
    })
    .expect("Error setting Ctrl-C handler");
}

fn main() {
    setup_cleanup_handler();

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
        println!("Claude Launcher v{}\n", VERSION);
        println!("Usage:");
        println!("  claude-launcher                    Auto-launch next TODO phase (parallel)");
        println!("  claude-launcher --step-by-step     Run tasks one at a time (sequential)");
        println!("  claude-launcher --worktree-per-phase Run phases in isolated git worktrees");
        println!("  claude-launcher --list-worktrees   List all active claude worktrees");
        println!("  claude-launcher --cleanup-worktrees Clean up completed worktrees");
        println!("  claude-launcher --init             Create .claude-launcher/ with empty config");
        println!(
            "  claude-launcher --init-lamdera     Create .claude-launcher/ with Lamdera preset"
        );
        println!(
            "  claude-launcher --smart-init       Analyze project and create appropriate config"
        );
        println!("  claude-launcher --create-task \"requirements\"  Generate task phases");
        println!("  claude-launcher --version          Show version information");
        println!("  claude-launcher \"task1\" \"task2\"    Launch specific tasks");
        std::process::exit(0);
    }

    // Check for special commands
    match args[1].as_str() {
        "--version" | "-v" => {
            println!("Claude Launcher v{}", VERSION);
            println!("A tool for managing parallel AI agent tasks");
            std::process::exit(0);
        }
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
        "--worktree-per-phase" => {
            handle_worktree_per_phase_mode(&current_dir);
            return;
        }
        "--list-worktrees" => {
            handle_list_worktrees(&current_dir);
            return;
        }
        "--cleanup-worktrees" => {
            handle_cleanup_worktrees(&current_dir);
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
    let config = load_config(current_dir);

    // Check if worktree mode is enabled in config
    if let Some(cfg) = &config {
        if cfg.worktree.enabled {
            println!("Worktree mode is enabled in config. Running with worktrees...");
            handle_worktree_per_phase_mode(current_dir);
            return;
        }
    }

    // Original auto mode logic continues here...
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
            let todo_steps: Vec<&Step> = phase
                .steps
                .iter()
                .filter(|step| step.status == "TODO")
                .collect();

            if todo_steps.is_empty() && phase.status == "TODO" {
                // All steps done but phase not complete - spawn CTO
                println!(
                    "🎯 All steps in Phase {} completed! Spawning Phase CTO...",
                    phase.id
                );

                // Check if phase is complete with worktree support
                let phase_complete = if let Some(cfg) = &config {
                    check_phase_completion(phase, cfg)
                } else {
                    phase.steps.iter().all(|s| s.status == "DONE")
                };

                if phase_complete {
                    // Phase is complete, may need to sync from worktree
                    if let Some(cfg) = &config {
                        if cfg.worktree.enabled {
                            if let Ok(state) = git_worktree::WorktreeState::load() {
                                if let Some(active_wt) =
                                    state.get_active_worktree(&phase.id.to_string())
                                {
                                    let worktree = git_worktree::Worktree {
                                        name: active_wt.worktree_name.clone(),
                                        path: active_wt.worktree_path.clone(),
                                        branch: active_wt.worktree_name.clone(),
                                        created_at: active_wt.created_at.clone(),
                                    };
                                    let _ = sync_worktree_changes(&worktree, &phase.id.to_string());
                                }
                            }
                        }
                    }
                }

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
            for (i, step) in todo_steps.iter().enumerate() {
                let prompt_file = if let Some(cfg) = &config {
                    if cfg.worktree.enabled {
                        // Use context-aware prompt generation for worktree mode
                        create_prompt_file_with_context(step, phase, cfg)
                    } else {
                        // Use regular prompt generation
                        let task_str =
                            format!("Phase {}, Step {}: {}", phase.id, step.id, step.name);
                        let prompt_file =
                            format!("{}/agent_prompt_task_{}.txt", current_dir, i + 1);
                        create_prompt_file(&prompt_file, &task_str, is_last_phase);
                        prompt_file
                    }
                } else {
                    // No config, use regular prompt generation
                    let task_str = format!("Phase {}, Step {}: {}", phase.id, step.id, step.name);
                    let prompt_file = format!("{}/agent_prompt_task_{}.txt", current_dir, i + 1);
                    create_prompt_file(&prompt_file, &task_str, is_last_phase);
                    prompt_file
                };

                let task_str = format!("Phase {}, Step {}: {}", phase.id, step.id, step.name);
                let applescript =
                    generate_applescript(&task_str, current_dir, &prompt_file, i == 0);
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
    
    let commands_section = if let Some(cfg) = &config {
        if !cfg.agent.commands.is_empty() {
            let commands_list = cfg.agent.commands
                .iter()
                .map(|cmd| {
                    format!("   - `{}`\n     Description: {}\n     Use instead of: {}", 
                        cmd.pattern, cmd.description, cmd.use_instead_of)
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            format!("\n\nAVAILABLE COMMANDS:\n{}\n\nIMPORTANT: When these commands are available, you MUST use them instead of directly editing files.\n", 
                commands_list
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let few_errors_max = config.as_ref().map(|c| c.cto.few_errors_max).unwrap_or(5);

    let pre_tasks_section = if let Some(cfg) = &config {
        if !cfg.agent.pre_tasks.is_empty() {
            let pre_tasks_list = cfg.agent.pre_tasks
                .iter()
                .enumerate()
                .map(|(i, cmd)| format!("{}. {}", i + 1, cmd))
                .collect::<Vec<_>>()
                .join("\n");
            format!("PRE-TASKS: Before reading prior work, execute these commands:\n{}\n\n", pre_tasks_list)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let prompt_content = format!(
        "{}FIRST: Read .claude-launcher/todos.json and analyze:\n\
        1. Comments from all completed steps in the current phase to understand what has been done\n\
        2. Comments from prior phases to understand the project context\n\
        3. Pay special attention to any issues or fixes mentioned\n{}\n\
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
        pre_tasks_section, commands_section, task, validation_commands, few_errors_max, few_errors_max + 1,
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
    
    let commands_section = if let Some(cfg) = &config {
        if !cfg.agent.commands.is_empty() {
            let commands_list = cfg.agent.commands
                .iter()
                .map(|cmd| {
                    format!("   - `{}`\n     Description: {}\n     Use instead of: {}", 
                        cmd.pattern, cmd.description, cmd.use_instead_of)
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            format!("\n\nAVAILABLE COMMANDS:\n{}\n\nIMPORTANT: When these commands are available, you MUST use them instead of directly editing files.\n", 
                commands_list
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let few_errors_max = config.as_ref().map(|c| c.cto.few_errors_max).unwrap_or(5);

    let pre_tasks_section = if let Some(cfg) = &config {
        if !cfg.agent.pre_tasks.is_empty() {
            let pre_tasks_list = cfg.agent.pre_tasks
                .iter()
                .enumerate()
                .map(|(i, cmd)| format!("{}. {}", i + 1, cmd))
                .collect::<Vec<_>>()
                .join("\n");
            format!("PRE-TASKS: Before reading prior work, execute these commands:\n{}\n\n", pre_tasks_list)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let prompt_content = format!(
        "{}FIRST: Read .claude-launcher/todos.json and analyze:\n\
        1. Comments from all completed steps in the current phase to understand what has been done\n\
        2. Comments from prior phases to understand the project context\n\
        3. Pay special attention to any issues or fixes mentioned\n{}\n\
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
        pre_tasks_section, commands_section, task, validation_commands, few_errors_max, few_errors_max + 1,
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
        let mut config: Config = serde_json::from_str(&contents).unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to parse config.json: {}. Using defaults.",
                e
            );
            Config {
                name: "Project".to_string(),
                agent: AgentConfig {
                    before_stop_commands: vec![],
                    commands: vec![],
                    pre_tasks: vec![],
                },
                cto: CtoConfig {
                    validation_commands: vec![],
                    few_errors_max: 5,
                },
                worktree: default_worktree_config(),
            }
        });

        // Ensure worktree config has defaults if missing
        if config.worktree.naming_pattern.is_empty() {
            config.worktree.naming_pattern = default_naming_pattern();
        }

        Some(config)
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

    let commands_section = if let Some(cfg) = &config {
        if !cfg.agent.commands.is_empty() {
            let commands_list = cfg.agent.commands
                .iter()
                .map(|cmd| {
                    if let Some(name) = &cmd.name {
                        format!("   - {}: {} (use instead of {})", name, cmd.description, cmd.use_instead_of)
                    } else {
                        format!("   - {} (use instead of {})", cmd.description, cmd.use_instead_of)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!("\n\nAVAILABLE COMMANDS:\n{}\n\nIMPORTANT: When these commands are available, you MUST use them instead of directly editing files.\n", 
                commands_list
            )
        } else {
            String::new()
        }
    } else {
        String::new()
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
        {}{}4. Based on the results:\n\
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
        phase.id, phase.id, validation_section, commands_section, launcher_command, few_errors_max, launcher_command, few_errors_max + 1, launcher_command, phase.id, ultimate_section
    );

    fs::write(file_path, prompt_content).expect("Failed to write CTO prompt file");
}

fn handle_init_command(current_dir: &str) {
    let launcher_dir = format!("{}/.claude-launcher", current_dir);
    let todos_path = format!("{}/todos.json", launcher_dir);
    let config_path = format!("{}/config.json", launcher_dir);
    let gitignore_path = format!("{}/.gitignore", launcher_dir);
    let claude_md_path = format!("{}/CLAUDE.md", launcher_dir);

    // Create .claude-launcher directory if it doesn't exist
    if !std::path::Path::new(&launcher_dir).exists() {
        fs::create_dir(&launcher_dir).expect("Failed to create .claude-launcher directory");
        println!("✅ Created .claude-launcher/ directory");
    }

    // Create todos.json if it doesn't exist
    if !std::path::Path::new(&todos_path).exists() {
        let empty_todos = TodosFile { phases: vec![] };
        let json = serde_json::to_string_pretty(&empty_todos).expect("Failed to serialize todos structure");
        fs::write(&todos_path, json).expect("Failed to create todos.json");
        println!("✅ Created .claude-launcher/todos.json");
    } else {
        println!("⏭️  Skipped .claude-launcher/todos.json (already exists)");
    }

    // Create config.json if it doesn't exist
    if !std::path::Path::new(&config_path).exists() {
        // Create empty config.json
        let empty_config = r#"{
  "name": "Project",
  "agent": {
    "before_stop_commands": [],
    "commands": [],
    "pre_tasks": []
  },
  "cto": {
    "validation_commands": [],
    "few_errors_max": 5
  },
  "worktree": {
    "enabled": false,
    "naming_pattern": "claude-phase-{id}-{timestamp}",
    "max_worktrees": 5,
    "base_branch": "main",
    "auto_cleanup": true
  }
}"#;

        fs::write(&config_path, empty_config).expect("Failed to create config.json");
        println!("✅ Created .claude-launcher/config.json");
    } else {
        println!("⏭️  Skipped .claude-launcher/config.json (already exists)");
    }
    
    // Create .gitignore if it doesn't exist
    if !std::path::Path::new(&gitignore_path).exists() {
        let gitignore_content = "# Temporary files\n*.tmp\n*.log\nworktree_state.json\n";
        fs::write(&gitignore_path, gitignore_content).expect("Failed to create .gitignore");
        println!("✅ Created .claude-launcher/.gitignore");
    } else {
        println!("⏭️  Skipped .claude-launcher/.gitignore (already exists)");
    }
    
    // Create CLAUDE.md if it doesn't exist
    if !std::path::Path::new(&claude_md_path).exists() {
        let claude_md_content = "# Project Instructions for Claude\n\n\
            ## Overview\n\
            Add project-specific instructions here that all Claude agents should follow.\n\n\
            ## Conventions\n\
            - Code style guidelines\n\
            - Naming conventions\n\
            - Architecture decisions\n\n\
            ## Important Notes\n\
            - Any project-specific quirks or requirements\n";
        fs::write(&claude_md_path, claude_md_content).expect("Failed to create CLAUDE.md");
        println!("✅ Created .claude-launcher/CLAUDE.md");
    } else {
        println!("⏭️  Skipped .claude-launcher/CLAUDE.md (already exists)");
    }

    println!("\n📝 Next step: Run 'claude-launcher --create-task \"your requirements\"' to generate task phases");
    println!("💡 Or run 'claude-launcher --init-lamdera' to create a Lamdera project setup");
}

fn handle_init_lamdera_command(current_dir: &str) {
    let launcher_dir = format!("{}/.claude-launcher", current_dir);
    let todos_path = format!("{}/todos.json", launcher_dir);
    let config_path = format!("{}/config.json", launcher_dir);
    let gitignore_path = format!("{}/.gitignore", launcher_dir);
    let claude_md_path = format!("{}/CLAUDE.md", launcher_dir);

    // Create .claude-launcher directory if it doesn't exist
    if !std::path::Path::new(&launcher_dir).exists() {
        fs::create_dir(&launcher_dir).expect("Failed to create .claude-launcher directory");
        println!("✅ Created .claude-launcher/ directory");
    }

    // Create todos.json if it doesn't exist
    if !std::path::Path::new(&todos_path).exists() {
        let empty_todos = TodosFile { phases: vec![] };
        let json = serde_json::to_string_pretty(&empty_todos).expect("Failed to serialize todos structure");
        fs::write(&todos_path, json).expect("Failed to create todos.json");
        println!("✅ Created .claude-launcher/todos.json");
    } else {
        println!("⏭️  Skipped .claude-launcher/todos.json (already exists)");
    }

    // Create Lamdera config.json if it doesn't exist
    if !std::path::Path::new(&config_path).exists() {
        // Create Lamdera config.json
        let lamdera_config = r#"{
  "name": "Lamdera Project",
  "agent": {
    "before_stop_commands": [],
    "commands": [
      {
        "description": "Add internationalization keys",
        "pattern": "elm-i18n add --fr \"French text\" --en \"English text\" KEY_NAME",
        "use_instead_of": "editing src/I18n.elm directly"
      },
      {
        "description": "Add function-based translations",
        "pattern": "elm-i18n add-fn --type-sig \"Int -> String\" --en \"\\\\n -> ...\" --fr \"\\\\n -> ...\" functionName",
        "use_instead_of": "editing src/I18n.elm for parameterized translations"
      }
    ],
    "pre_tasks": [
      "lamdera make src/Frontend.elm src/Backend.elm",
      "elm-test-rs --compiler lamdera"
    ]
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
  },
  "worktree": {
    "enabled": false,
    "naming_pattern": "claude-phase-{id}-{timestamp}",
    "max_worktrees": 5,
    "base_branch": "main",
    "auto_cleanup": true
  }
}"#;

        fs::write(&config_path, lamdera_config).expect("Failed to create config.json");
        println!("✅ Created .claude-launcher/config.json (Lamdera preset)");
    } else {
        println!("⏭️  Skipped .claude-launcher/config.json (already exists)");
    }
    
    // Create .gitignore if it doesn't exist
    if !std::path::Path::new(&gitignore_path).exists() {
        let gitignore_content = "# Temporary files\n*.tmp\n*.log\nworktree_state.json\n\n# Lamdera\n.lamdera/\n";
        fs::write(&gitignore_path, gitignore_content).expect("Failed to create .gitignore");
        println!("✅ Created .claude-launcher/.gitignore (with Lamdera patterns)");
    } else {
        println!("⏭️  Skipped .claude-launcher/.gitignore (already exists)");
    }
    
    // Create CLAUDE.md if it doesn't exist
    if !std::path::Path::new(&claude_md_path).exists() {
        let claude_md_content = "# Lamdera Project Instructions for Claude\n\n\
            ## Overview\n\
            This is a Lamdera project. Key differences from regular Elm:\n\
            - Frontend and Backend modules\n\
            - Shared Types module\n\n\
            ## Testing\n\
            - Use lamdera-program-test for TDD\n\
            - Run tests with: elm-test-rs --compiler lamdera\n\n\
            ## Commands\n\
            - Compile: lamdera make src/Frontend.elm src/Backend.elm\n\n\
            ## Important Notes\n\
            - Always use elm-i18n commands for translations (don't edit I18n.elm directly)\n\
            - Follow the existing architecture patterns\n";
        fs::write(&claude_md_path, claude_md_content).expect("Failed to create CLAUDE.md");
        println!("✅ Created .claude-launcher/CLAUDE.md (Lamdera template)");
    } else {
        println!("⏭️  Skipped .claude-launcher/CLAUDE.md (already exists)");
    }

    println!("\n🔧 Lamdera configuration includes:");
    println!("   - lamdera make and elm-test-rs validation commands");
    println!("   - elm-i18n commands for internationalization");
    println!("\n📝 Next step: Run 'claude-launcher --create-task \"your requirements\"' to generate task phases");
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
  },
  "worktree": {
    "enabled": false,
    "naming_pattern": "claude-phase-{id}-{timestamp}",
    "max_worktrees": 5,
    "base_branch": "main",
    "auto_cleanup": true
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
        r#"You are a project planning expert specialized in MAXIMIZING PARALLELIZATION.

REQUIREMENTS: {}

Create a detailed implementation plan that MAXIMIZES PARALLELIZATION and MINIMIZES PHASES.

CRITICAL PARALLELIZATION RULES:
1. ONE FILE PER AGENT: Each agent should modify exactly ONE file (maximum parallelization)
2. If a feature needs 10 files modified = 10 parallel agents in ONE phase
3. If one file needs extensive work = ONE agent across multiple phases
4. Pack as many parallel tasks as possible into each phase (aim for 10-30 tasks)
5. Only create a new phase when tasks have TRUE dependencies on previous phase outputs
6. Every agent prompt must end with "IMPORTANT: Complete ONLY this specific task. Once finished, STOP."

THE GOLDEN RULE: Number of files to modify = Number of parallel agents = ONE PHASE

GOOD EXAMPLE - Feature requiring 20 file changes:
Phase 1: 20 parallel tasks (each agent modifies ONE file)
- 1A: Create src/User.elm
- 1B: Create src/Product.elm  
- 1C: Update src/Types.elm (add User type)
- 1D: Update src/Types.elm (add Product type) <- WRONG! Same file!
Instead: 1C: Update src/Types.elm (add User AND Product types) <- One agent updates the file

BAD EXAMPLE - Same feature split unnecessarily:
Phase 1: Create models (5 agents on 5 files)
Phase 2: Create views (5 agents on 5 files)
Phase 3: Create controllers (5 agents on 5 files)
Phase 4: Create tests (5 agents on 5 files)
(All 20 files are independent - should be ONE phase with 20 agents!)

WHEN TO USE MULTIPLE PHASES:
- Complex logic in ONE file that builds incrementally
- Integration work that depends on multiple components existing
- Tests that need the implementation to compile first
- Refactoring that must happen in sequence

CONCRETE EXAMPLE - E-commerce Feature (Orders, Products, Users):
BEST APPROACH - One phase, many agents:
Phase 1: Create entire feature - 15 parallel tasks
- 1A: Create src/Models/User.elm
- 1B: Create src/Models/Product.elm 
- 1C: Create src/Models/Order.elm
- 1D: Create src/Views/UserList.elm
- 1E: Create src/Views/ProductList.elm
- 1F: Create src/Views/OrderList.elm
- 1G: Update src/Types.elm (add ALL new types: User, Product, Order)
- 1H: Update src/Frontend.elm (add ALL new messages and routing)
- 1I: Update src/Backend.elm (add ALL new handlers)
... etc (each file touched ONCE by ONE agent)

WORST APPROACH - Many phases, few agents:
Phase 1: Create User feature (3 tasks)
Phase 2: Create Product feature (3 tasks)
Phase 3: Create Order feature (3 tasks)
Phase 4: Integration (3 tasks)
(This creates artificial dependencies and slows everything down!)

TASK PROMPT REQUIREMENTS:
1. Include exact file paths and function names
2. Provide complete code examples (not just descriptions)
3. Specify imports and dependencies explicitly
4. Each step id should be like "1A", "1B", "1C"... "1Z", "1AA", "1AB", etc.
5. End EVERY prompt with: "IMPORTANT: Complete ONLY this specific task. Once finished, STOP."

The JSON structure should be:
{{
  "phases": [
    {{
      "id": 1,
      "name": "Phase Name - X Parallel Tasks",
      "steps": [
        {{
          "id": "1A",
          "name": "Task Name",
          "prompt": "Detailed instructions with complete code examples...\n\nIMPORTANT: Complete ONLY this specific task. Once finished, STOP.",
          "status": "TODO",
          "comment": ""
        }}
      ],
      "status": "TODO",
      "comment": ""
    }}
  ]
}}

CRITICAL: Replace the entire .claude-launcher/todos.json file with your new implementation plan that MAXIMIZES PARALLELIZATION."#,
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

// Add worktree support to phase completion detection
fn check_phase_completion(phase: &Phase, config: &Config) -> bool {
    let all_done = phase.steps.iter().all(|s| s.status == "DONE");

    if all_done && config.worktree.enabled {
        // Mark worktree as completed
        if let Ok(mut state) = git_worktree::WorktreeState::load() {
            state.mark_completed(&phase.id.to_string());
            let _ = state.save();

            // Trigger cleanup if auto_cleanup is enabled
            if config.worktree.auto_cleanup {
                let _ = state.cleanup_completed(&config.worktree);
            }
        }
    }

    all_done
}

// Update prompt generation to include worktree context
fn create_prompt_file_with_context(step: &Step, phase: &Phase, config: &Config) -> String {
    let prompt_file = format!("/tmp/claude_prompt_{}_{}.md", phase.id, step.id);

    let mut prompt_content = format!("# Task: {}\n\n## Phase: {}\n\n", step.name, phase.name);

    // Add worktree context if enabled
    if config.worktree.enabled {
        if let Ok(state) = git_worktree::WorktreeState::load() {
            if let Some(active_wt) = state.get_active_worktree(&phase.id.to_string()) {
                prompt_content.push_str(&format!(
                    "## Worktree Context\n\
                    You are working in an isolated git worktree:\n\
                    - Worktree: {}\n\
                    - Path: {}\n\
                    - Branch: {}\n\n",
                    active_wt.worktree_name,
                    active_wt.worktree_path.display(),
                    active_wt.worktree_name
                ));
            }
        }
    }

    // Add the main prompt
    prompt_content.push_str(&format!("## Instructions\n\n{}\n\n", step.prompt));

    // Add update instructions
    prompt_content.push_str(
        "## Important\n\
        1. When you complete this task, update the status to 'DONE' in .claude-launcher/todos.json\n\
        2. Add a comment describing what you accomplished\n\
        3. Only work on this specific task - do not start other tasks\n"
    );

    std::fs::write(&prompt_file, prompt_content).expect("Failed to write prompt file");

    prompt_file
}

// Add helper to sync changes back from worktree
fn sync_worktree_changes(worktree: &git_worktree::Worktree, phase_id: &str) -> std::io::Result<()> {
    // Copy updated todos.json back to main repo
    let worktree_todos = worktree.path.join(".claude-launcher/todos.json");
    if worktree_todos.exists() {
        std::fs::copy(&worktree_todos, ".claude-launcher/todos.json")?;
        println!("Synced todos.json from worktree {}", worktree.name);
    }

    // Create a commit in the worktree if there are changes
    let output = std::process::Command::new("git")
        .current_dir(&worktree.path)
        .args(["add", "-A"])
        .output()?;

    if output.status.success() {
        let commit_msg = format!(
            "Phase {} implementation from worktree {}",
            phase_id, worktree.name
        );
        std::process::Command::new("git")
            .current_dir(&worktree.path)
            .args(["commit", "-m", &commit_msg])
            .output()?;
    }

    Ok(())
}

// Add merge helper for completed worktrees
#[allow(dead_code)]
fn merge_worktree_branch(
    worktree: &git_worktree::Worktree,
    base_branch: &str,
) -> std::io::Result<()> {
    println!(
        "Merging worktree branch {} into {}",
        worktree.branch, base_branch
    );

    // Switch to base branch in main repo
    std::process::Command::new("git")
        .args(["checkout", base_branch])
        .output()?;

    // Merge the worktree branch
    let output = std::process::Command::new("git")
        .args([
            "merge",
            "--no-ff",
            "-m",
            &format!("Merge phase implementation from {}", worktree.branch),
            &worktree.branch,
        ])
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Failed to merge: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    println!(
        "Successfully merged {} into {}",
        worktree.branch, base_branch
    );
    Ok(())
}

// Implement the handler function
fn handle_worktree_per_phase_mode(current_dir: &str) {
    println!("Running in worktree-per-phase mode...");

    let config = load_config(current_dir).unwrap_or_else(|| {
        eprintln!("Error: Failed to load config. Run 'claude-launcher --init' first");
        std::process::exit(1);
    });
    let todos = load_todos(current_dir);

    // Enable worktree mode in config temporarily
    let mut worktree_config = config.worktree.clone();
    worktree_config.enabled = true;

    // Find next TODO phase
    if let Some(phase) = todos
        .phases
        .iter()
        .find(|p| p.status == "TODO" || p.steps.iter().any(|s| s.status == "TODO"))
    {
        let phase_id = phase.id.to_string();
        println!(
            "Starting phase {} in worktree mode: {}",
            phase_id, phase.name
        );

        // Load or create worktree state
        let mut state = git_worktree::WorktreeState::load()
            .unwrap_or_else(|_| git_worktree::WorktreeState::new());

        // Check if phase already has an active worktree
        let worktree = if let Some(active_wt) = state.get_active_worktree(&phase_id) {
            println!("Resuming in existing worktree: {}", active_wt.worktree_name);
            git_worktree::Worktree {
                name: active_wt.worktree_name.clone(),
                path: active_wt.worktree_path.clone(),
                branch: active_wt.worktree_name.clone(),
                created_at: active_wt.created_at.clone(),
            }
        } else {
            // Create new worktree for this phase
            println!("Creating new worktree for phase {}...", phase_id);
            let base_branch = worktree_config.base_branch.clone();

            match git_worktree::create_worktree(&phase_id, &base_branch) {
                Ok(wt) => {
                    state.add_worktree(phase_id.clone(), &wt);
                    state.save().expect("Failed to save worktree state");
                    println!("Created worktree: {} at {}", wt.name, wt.path.display());
                    wt
                }
                Err(git_worktree::WorktreeError::WorktreeExists(name)) => {
                    eprintln!("Worktree {} already exists. Attempting recovery...", name);

                    // Try to recover existing worktree
                    if let Ok(worktrees) = git_worktree::list_claude_worktrees() {
                        if let Some(existing) = worktrees.into_iter().find(|w| w.name == name) {
                            println!("Found existing worktree, resuming...");
                            existing
                        } else {
                            eprintln!(
                                "Could not recover worktree. Falling back to regular execution."
                            );
                            handle_auto_mode(current_dir);
                            return;
                        }
                    } else {
                        eprintln!("Could not list worktrees. Falling back to regular execution.");
                        handle_auto_mode(current_dir);
                        return;
                    }
                }
                Err(git_worktree::WorktreeError::NotInGitRepo) => {
                    eprintln!("Error: Not in a git repository. Please initialize git first.");
                    eprintln!("Run: git init");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Failed to create worktree: {}", e);
                    eprintln!("Falling back to regular execution.");
                    handle_auto_mode(current_dir);
                    return;
                }
            }
        };

        // Execute phase in worktree
        execute_phase_in_worktree(phase, &worktree, &config, current_dir);
    } else {
        println!("No TODO phases found.");
    }
}

// Add helper function to execute phase in worktree
fn execute_phase_in_worktree(
    phase: &Phase,
    worktree: &git_worktree::Worktree,
    _config: &Config,
    current_dir: &str,
) {
    // Copy necessary files to worktree
    let worktree_launcher_dir = worktree.path.join(".claude-launcher");

    // Ensure .claude-launcher directory exists in worktree
    std::fs::create_dir_all(&worktree_launcher_dir)
        .expect("Failed to create .claude-launcher in worktree");

    // Copy todos.json to worktree
    std::fs::copy(
        format!("{}/.claude-launcher/todos.json", current_dir),
        worktree_launcher_dir.join("todos.json"),
    )
    .expect("Failed to copy todos.json to worktree");

    // Copy config.json but disable worktree mode for the copy in the worktree
    let config_content = std::fs::read_to_string(format!("{}/.claude-launcher/config.json", current_dir))
        .expect("Failed to read config.json");
    
    // Parse and modify config to disable worktree mode
    let mut config_json: serde_json::Value = serde_json::from_str(&config_content)
        .expect("Failed to parse config.json");
    
    if let Some(worktree) = config_json.get_mut("worktree") {
        if let Some(obj) = worktree.as_object_mut() {
            obj.insert("enabled".to_string(), serde_json::Value::Bool(false));
        }
    }
    
    std::fs::write(
        worktree_launcher_dir.join("config.json"),
        serde_json::to_string_pretty(&config_json).expect("Failed to serialize config.json"),
    )
    .expect("Failed to write config.json to worktree");

    // Copy CLAUDE.md if it exists
    let claude_md_path = format!("{}/.claude-launcher/CLAUDE.md", current_dir);
    if std::path::Path::new(&claude_md_path).exists() {
        std::fs::copy(
            &claude_md_path,
            worktree_launcher_dir.join("CLAUDE.md"),
        )
        .expect("Failed to copy CLAUDE.md to worktree");
    }

    // Get absolute path for worktree
    let worktree_abs_path = if worktree.path.is_absolute() {
        worktree.path.clone()
    } else {
        std::env::current_dir()
            .expect("Failed to get current directory")
            .join(&worktree.path)
            .canonicalize()
            .unwrap_or_else(|_| {
                // If canonicalize fails (worktree doesn't exist yet), construct the path manually
                std::env::current_dir()
                    .expect("Failed to get current directory")
                    .join(&worktree.path)
            })
    };

    // Generate phase execution script
    let script_content = format!(
        r#"#!/bin/bash
cd "{}"
echo "Executing phase {} in worktree: {}"

# Run claude-launcher in the worktree
/Users/charles-andreassus/.local/bin/claude-launcher
"#,
        worktree_abs_path.display(),
        phase.id,
        worktree.name
    );

    let script_path = format!("/tmp/claude_worktree_phase_{}.sh", phase.id);
    std::fs::write(&script_path, script_content).expect("Failed to write worktree script");

    // Make script executable
    std::process::Command::new("chmod")
        .args(["+x", &script_path])
        .output()
        .expect("Failed to make script executable");

    // Generate AppleScript to run in new iTerm tab
    let applescript = generate_applescript_for_worktree(&script_path, &worktree.name);

    // Execute AppleScript
    let mut child = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&applescript)
        .spawn()
        .expect("Failed to execute AppleScript");

    child.wait().expect("Failed to wait for AppleScript");
}

// Add AppleScript generator for worktree execution
fn generate_applescript_for_worktree(script_path: &str, worktree_name: &str) -> String {
    format!(
        r#"tell application "iTerm"
    activate
    tell current window
        create tab with default profile
        tell current session
            write text "echo 'Starting worktree execution: {}'"
            write text "{}"
        end tell
    end tell
end tell"#,
        worktree_name, script_path
    )
}

// Helper function to load todos
fn load_todos(current_dir: &str) -> TodosFile {
    let todos_path = format!("{}/.claude-launcher/todos.json", current_dir);

    if !std::path::Path::new(&todos_path).exists() {
        eprintln!(
            "Error: .claude-launcher/todos.json does not exist. Run 'claude-launcher --init' first"
        );
        std::process::exit(1);
    }

    let contents = fs::read_to_string(&todos_path).expect("Failed to read todos.json");
    serde_json::from_str(&contents).expect("Failed to parse todos.json")
}

// Implementation for listing worktrees
fn handle_list_worktrees(current_dir: &str) {
    println!("Claude Launcher Active Worktrees");
    println!("================================\n");

    // List git worktrees
    match git_worktree::list_claude_worktrees() {
        Ok(worktrees) => {
            if worktrees.is_empty() {
                println!("No active claude-launcher worktrees found.");
            } else {
                // Load worktree state to get additional info
                let state = git_worktree::WorktreeState::load()
                    .unwrap_or_else(|_| git_worktree::WorktreeState::new());

                println!("Found {} worktree(s):\n", worktrees.len());

                for (idx, worktree) in worktrees.iter().enumerate() {
                    println!("{}. {}", idx + 1, worktree.name);
                    println!("   Path: {}", worktree.path.display());
                    println!("   Branch: {}", worktree.branch);
                    println!("   Created: {}", worktree.created_at);

                    // Find phase info from state
                    if let Some(active_wt) = state
                        .active_worktrees
                        .iter()
                        .find(|w| w.worktree_name == worktree.name)
                    {
                        println!("   Phase ID: {}", active_wt.phase_id);
                        println!("   Status: {:?}", active_wt.status);

                        // Check if phase has any TODO items
                        if let Ok(wt_todos_path) = worktree
                            .path
                            .join(".claude-launcher/todos.json")
                            .canonicalize()
                        {
                            if wt_todos_path.exists() {
                                if let Ok(contents) = std::fs::read_to_string(&wt_todos_path) {
                                    if let Ok(todos) = serde_json::from_str::<TodosFile>(&contents)
                                    {
                                        let phase_id: u32 = active_wt.phase_id.parse().unwrap_or(0);
                                        if let Some(phase) =
                                            todos.phases.iter().find(|p| p.id == phase_id)
                                        {
                                            let todo_count = phase
                                                .steps
                                                .iter()
                                                .filter(|s| s.status == "TODO")
                                                .count();
                                            let in_progress_count = phase
                                                .steps
                                                .iter()
                                                .filter(|s| s.status == "IN PROGRESS")
                                                .count();
                                            let done_count = phase
                                                .steps
                                                .iter()
                                                .filter(|s| s.status == "DONE")
                                                .count();

                                            println!("   Phase: {}", phase.name);
                                            println!(
                                                "   Progress: {} TODO, {} IN PROGRESS, {} DONE",
                                                todo_count, in_progress_count, done_count
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }

                    println!();
                }

                // Show cleanup info
                let config = load_config(current_dir);
                if let Some(cfg) = config {
                    if cfg.worktree.auto_cleanup {
                        println!(
                            "Auto-cleanup: Enabled (max {} worktrees)",
                            cfg.worktree.max_worktrees
                        );
                    } else {
                        println!("Auto-cleanup: Disabled");
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error listing worktrees: {}", e);
        }
    }

    // Show worktree state summary
    println!("\nWorktree State Summary:");
    println!("-----------------------");

    if let Ok(state) = git_worktree::WorktreeState::load() {
        let active_count = state
            .active_worktrees
            .iter()
            .filter(|w| w.status == git_worktree::WorktreeStatus::Active)
            .count();
        let completed_count = state
            .active_worktrees
            .iter()
            .filter(|w| w.status == git_worktree::WorktreeStatus::Completed)
            .count();
        let failed_count = state
            .active_worktrees
            .iter()
            .filter(|w| w.status == git_worktree::WorktreeStatus::Failed)
            .count();

        println!("Active: {}", active_count);
        println!("Completed: {}", completed_count);
        println!("Failed: {}", failed_count);
        println!("Total tracked: {}", state.active_worktrees.len());
    } else {
        println!("No worktree state file found.");
    }

    // Suggest cleanup command if needed
    match git_worktree::list_claude_worktrees() {
        Ok(worktrees) if worktrees.len() > 3 => {
            println!(
                "\nTip: You have {} worktrees. Consider running cleanup to remove old ones.",
                worktrees.len()
            );
            println!("     Use: claude-launcher --cleanup-worktrees");
        }
        _ => {}
    }
}

// Add a cleanup command as well
fn handle_cleanup_worktrees(current_dir: &str) {
    println!("Cleaning up completed worktrees...");

    let config = load_config(current_dir).unwrap_or_else(|| {
        eprintln!("Error: Failed to load config. Using defaults.");
        Config {
            name: "Project".to_string(),
            agent: AgentConfig {
                before_stop_commands: vec![],
                commands: vec![],
                pre_tasks: vec![],
            },
            cto: CtoConfig {
                validation_commands: vec![],
                few_errors_max: 5,
            },
            worktree: default_worktree_config(),
        }
    });

    let mut state =
        git_worktree::WorktreeState::load().unwrap_or_else(|_| git_worktree::WorktreeState::new());

    match state.cleanup_completed(&config.worktree) {
        Ok(_) => {
            println!("Cleanup completed successfully.");

            // Show remaining worktrees
            if let Ok(worktrees) = git_worktree::list_claude_worktrees() {
                println!("Remaining worktrees: {}", worktrees.len());
            }
        }
        Err(e) => {
            eprintln!("Error during cleanup: {}", e);
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_worktree_config_loading() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // Create config with worktree settings
        fs::create_dir(".claude-launcher").unwrap();
        let config_json = r#"{
            "name": "Test Project",
            "agent": {
                "before_stop_commands": [],
                "commands": []
            },
            "cto": {
                "validation_commands": [],
                "few_errors_max": 3
            },
            "worktree": {
                "enabled": true,
                "naming_pattern": "test-{id}-{timestamp}",
                "max_worktrees": 10,
                "base_branch": "develop",
                "auto_cleanup": false
            }
        }"#;

        fs::write(".claude-launcher/config.json", config_json).unwrap();

        let config = load_config(temp_dir.path().to_str().unwrap()).expect("Failed to load config");
        assert!(config.worktree.enabled);
        assert_eq!(config.worktree.naming_pattern, "test-{id}-{timestamp}");
        assert_eq!(config.worktree.max_worktrees, 10);
        assert_eq!(config.worktree.base_branch, "develop");
        assert!(!config.worktree.auto_cleanup);

        // Cleanup
        let _ = std::env::set_current_dir(original_dir);
    }

    #[test]
    fn test_worktree_config_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        
        // Ensure we can change to temp directory
        if let Err(e) = std::env::set_current_dir(temp_dir.path()) {
            eprintln!("Failed to change to temp dir: {}", e);
            return;
        }

        // Test with missing worktree config
        fs::create_dir(".claude-launcher").unwrap();
        let config_json = r#"{
            "name": "Test Project",
            "agent": {
                "before_stop_commands": [],
                "commands": []
            },
            "cto": {
                "validation_commands": [],
                "few_errors_max": 3
            }
        }"#;

        fs::write(".claude-launcher/config.json", config_json).unwrap();

        let config = load_config(temp_dir.path().to_str().unwrap()).expect("Failed to load config");
        assert!(!config.worktree.enabled);
        assert_eq!(
            config.worktree.naming_pattern,
            "claude-phase-{id}-{timestamp}"
        );
        assert_eq!(config.worktree.max_worktrees, 5);
        assert_eq!(config.worktree.base_branch, "main");
        assert!(config.worktree.auto_cleanup);

        // Cleanup
        let _ = std::env::set_current_dir(original_dir);
    }
}
