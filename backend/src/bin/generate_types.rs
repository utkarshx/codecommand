use std::{env, fs, path::Path};

use ts_rs::TS; // in [build-dependencies]

fn generate_constants() -> String {
    r#"// Generated constants
export const EXECUTOR_TYPES: string[] = [
    "echo",
    "claude",
    "amp",
    "gemini",
    "opencode"
];

export const EDITOR_TYPES: EditorType[] = [
    "vscode",
    "cursor", 
    "windsurf",
    "intellij",
    "zed",
    "custom"
];

export const EXECUTOR_LABELS: Record<string, string> = {
    "echo": "Echo (Test Mode)",
    "claude": "Claude",
    "amp": "Amp",
    "gemini": "Gemini",
    "opencode": "OpenCode"
};

export const EDITOR_LABELS: Record<string, string> = {
    "vscode": "VS Code",
    "cursor": "Cursor",
    "windsurf": "Windsurf",
    "intellij": "IntelliJ IDEA",
    "zed": "Zed",
    "custom": "Custom"
};

export const SOUND_FILES: SoundFile[] = [
    "abstract-sound1",
    "abstract-sound2",
    "abstract-sound3",
    "abstract-sound4",
    "cow-mooing",
    "phone-vibration",
    "rooster"
];

export const SOUND_LABELS: Record<string, string> = {
    "abstract-sound1": "Gentle Chime",
    "abstract-sound2": "Soft Bell",
    "abstract-sound3": "Digital Tone",
    "abstract-sound4": "Subtle Alert",
    "cow-mooing": "Cow Mooing",
    "phone-vibration": "Phone Vibration",
    "rooster": "Rooster Call"
};"#
    .to_string()
}

fn main() {
    // 1. Make sure ../shared exists
    let shared_path = Path::new("../shared");
    fs::create_dir_all(shared_path).expect("cannot create ../shared");

    println!("Generating TypeScript types…");

    // 2. Let ts-rs write its per-type files here (handy for debugging)
    env::set_var("TS_RS_EXPORT_DIR", shared_path.to_str().unwrap());

    // 3. Grab every Rust type you want on the TS side
    let decls = [
        codecommand::models::ApiResponse::<()>::decl(),
        codecommand::models::config::Config::decl(),
        codecommand::models::config::ThemeMode::decl(),
        codecommand::models::config::EditorConfig::decl(),
        codecommand::models::config::GitHubConfig::decl(),
        codecommand::models::config::EditorType::decl(),
        codecommand::models::config::EditorConstants::decl(),
        codecommand::models::config::SoundFile::decl(),
        codecommand::models::config::SoundConstants::decl(),
        codecommand::routes::config::ConfigConstants::decl(),
        codecommand::executor::ExecutorConfig::decl(),
        codecommand::executor::ExecutorConstants::decl(),
        codecommand::models::project::CreateProject::decl(),
        codecommand::models::project::Project::decl(),
        codecommand::models::project::ProjectWithBranch::decl(),
        codecommand::models::project::UpdateProject::decl(),
        codecommand::models::project::SearchResult::decl(),
        codecommand::models::project::SearchMatchType::decl(),
        codecommand::models::project::GitBranch::decl(),
        codecommand::models::project::CreateBranch::decl(),
        codecommand::models::task::CreateTask::decl(),
        codecommand::models::task::CreateTaskAndStart::decl(),
        codecommand::models::task::TaskStatus::decl(),
        codecommand::models::task::Task::decl(),
        codecommand::models::task::TaskWithAttemptStatus::decl(),
        codecommand::models::task::UpdateTask::decl(),
        codecommand::models::task_attempt::TaskAttemptStatus::decl(),
        codecommand::models::task_attempt::TaskAttempt::decl(),
        codecommand::models::task_attempt::CreateTaskAttempt::decl(),
        codecommand::models::task_attempt::UpdateTaskAttempt::decl(),
        codecommand::models::task_attempt::CreateFollowUpAttempt::decl(),
        codecommand::models::task_attempt_activity::TaskAttemptActivity::decl(),
        codecommand::models::task_attempt_activity::TaskAttemptActivityWithPrompt::decl(),
        codecommand::models::task_attempt_activity::CreateTaskAttemptActivity::decl(),
        codecommand::routes::filesystem::DirectoryEntry::decl(),
        codecommand::models::task_attempt::DiffChunkType::decl(),
        codecommand::models::task_attempt::DiffChunk::decl(),
        codecommand::models::task_attempt::FileDiff::decl(),
        codecommand::models::task_attempt::WorktreeDiff::decl(),
        codecommand::models::task_attempt::BranchStatus::decl(),
        codecommand::models::task_attempt::ExecutionState::decl(),
        codecommand::models::task_attempt::TaskAttemptState::decl(),
        codecommand::models::execution_process::ExecutionProcess::decl(),
        codecommand::models::execution_process::ExecutionProcessSummary::decl(),
        codecommand::models::execution_process::ExecutionProcessStatus::decl(),
        codecommand::models::execution_process::ExecutionProcessType::decl(),
        codecommand::models::execution_process::CreateExecutionProcess::decl(),
        codecommand::models::execution_process::UpdateExecutionProcess::decl(),
        codecommand::models::executor_session::ExecutorSession::decl(),
        codecommand::models::executor_session::CreateExecutorSession::decl(),
        codecommand::models::executor_session::UpdateExecutorSession::decl(),
        codecommand::executor::NormalizedConversation::decl(),
        codecommand::executor::NormalizedEntry::decl(),
        codecommand::executor::NormalizedEntryType::decl(),
        codecommand::executor::ActionType::decl(),
    ];

    // 4. Friendly banner
    const HEADER: &str =
        "// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs).\n\
         // Do not edit this file manually.\n\
         // Auto-generated from Rust backend types using ts-rs\n\n";

    // 5. Add `export` if it’s missing, then join
    let body = decls
        .into_iter()
        .map(|d| {
            let trimmed = d.trim_start();
            if trimmed.starts_with("export") {
                d
            } else {
                format!("export {trimmed}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // 6. Add constants
    let constants = generate_constants();

    // 7. Write the consolidated types.ts
    fs::write(
        shared_path.join("types.ts"),
        format!("{HEADER}{body}\n\n{constants}"),
    )
    .expect("unable to write types.ts");

    println!("✅ TypeScript types generated in ../shared/");
}
