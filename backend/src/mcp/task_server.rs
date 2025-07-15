use rmcp::{
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, Error as RmcpError, ServerHandler,
};
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::{
    project::Project,
    task::{CreateTask, Task, TaskStatus},
};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateTaskRequest {
    #[schemars(description = "The ID of the project to create the task in. This is required!")]
    pub project_id: String,
    #[schemars(description = "The title of the task")]
    pub title: String,
    #[schemars(description = "Optional description of the task")]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListProjectsRequest {
    // Empty for now, but we can add filtering options later
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct CreateTaskResponse {
    pub success: bool,
    pub task_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ProjectSummary {
    #[schemars(description = "The unique identifier of the project")]
    pub id: String,
    #[schemars(description = "The name of the project")]
    pub name: String,
    #[schemars(description = "The path to the git repository")]
    pub git_repo_path: String,
    #[schemars(description = "Optional setup script for the project")]
    pub setup_script: Option<String>,
    #[schemars(description = "Optional development script for the project")]
    pub dev_script: Option<String>,
    #[schemars(description = "Current git branch (if available)")]
    pub current_branch: Option<String>,
    #[schemars(description = "When the project was created")]
    pub created_at: String,
    #[schemars(description = "When the project was last updated")]
    pub updated_at: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListProjectsResponse {
    pub success: bool,
    pub projects: Vec<ProjectSummary>,
    pub count: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListTasksRequest {
    #[schemars(description = "The ID of the project to list tasks from")]
    pub project_id: String,
    #[schemars(
        description = "Optional status filter: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'"
    )]
    pub status: Option<String>,
    #[schemars(description = "Maximum number of tasks to return (default: 50)")]
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TaskSummary {
    #[schemars(description = "The unique identifier of the task")]
    pub id: String,
    #[schemars(description = "The title of the task")]
    pub title: String,
    #[schemars(description = "Optional description of the task")]
    pub description: Option<String>,
    #[schemars(description = "Current status of the task")]
    pub status: String,
    #[schemars(description = "When the task was created")]
    pub created_at: String,
    #[schemars(description = "When the task was last updated")]
    pub updated_at: String,
    #[schemars(description = "Whether the task has an in-progress execution attempt")]
    pub has_in_progress_attempt: Option<bool>,
    #[schemars(description = "Whether the task has a merged execution attempt")]
    pub has_merged_attempt: Option<bool>,
    #[schemars(description = "Whether the task has a failed execution attempt")]
    pub has_failed_attempt: Option<bool>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksResponse {
    pub success: bool,
    pub tasks: Vec<TaskSummary>,
    pub count: usize,
    pub project_id: String,
    pub project_name: Option<String>,
    pub applied_filters: ListTasksFilters,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ListTasksFilters {
    pub status: Option<String>,
    pub limit: i32,
}

fn parse_task_status(status_str: &str) -> Option<TaskStatus> {
    match status_str.to_lowercase().as_str() {
        "todo" => Some(TaskStatus::Todo),
        "inprogress" | "in-progress" | "in_progress" => Some(TaskStatus::InProgress),
        "inreview" | "in-review" | "in_review" => Some(TaskStatus::InReview),
        "done" | "completed" => Some(TaskStatus::Done),
        "cancelled" | "canceled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

fn task_status_to_string(status: &TaskStatus) -> String {
    match status {
        TaskStatus::Todo => "todo".to_string(),
        TaskStatus::InProgress => "in-progress".to_string(),
        TaskStatus::InReview => "in-review".to_string(),
        TaskStatus::Done => "done".to_string(),
        TaskStatus::Cancelled => "cancelled".to_string(),
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateTaskRequest {
    #[schemars(description = "The ID of the project containing the task")]
    pub project_id: String,
    #[schemars(description = "The ID of the task to update")]
    pub task_id: String,
    #[schemars(description = "New title for the task")]
    pub title: Option<String>,
    #[schemars(description = "New description for the task")]
    pub description: Option<String>,
    #[schemars(description = "New status: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'")]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct UpdateTaskResponse {
    pub success: bool,
    pub message: String,
    pub task: Option<TaskSummary>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteTaskRequest {
    #[schemars(description = "The ID of the project containing the task")]
    pub project_id: String,
    #[schemars(description = "The ID of the task to delete")]
    pub task_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DeleteTaskResponse {
    pub success: bool,
    pub message: String,
    pub deleted_task_id: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SimpleTaskResponse {
    pub success: bool,
    pub message: String,
    pub task_title: String,
    pub new_status: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTaskRequest {
    #[schemars(description = "The ID of the project containing the task")]
    pub project_id: String,
    #[schemars(description = "The ID of the task to retrieve")]
    pub task_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct GetTaskResponse {
    pub success: bool,
    pub task: Option<TaskSummary>,
    pub project_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskServer {
    pub pool: SqlitePool,
}

impl TaskServer {
    #[allow(dead_code)]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[tool(tool_box)]
impl TaskServer {
    #[tool(
        description = "Create a new task/ticket in a project. Always pass the `project_id` of the project you want to create the task in - it is required!"
    )]
    async fn create_task(
        &self,
        #[tool(aggr)] CreateTaskRequest {
            project_id,
            title,
            description,
        }: CreateTaskRequest,
    ) -> Result<CallToolResult, RmcpError> {
        // Parse project_id from string to UUID
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid project ID format. Must be a valid UUID.",
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Invalid project ID format".to_string()),
                )]));
            }
        };

        // Check if project exists
        match Project::exists(&self.pool, project_uuid).await {
            Ok(false) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Project not found",
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Project not found".to_string()),
                )]));
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to check project existence",
                    "details": e.to_string(),
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Database error".to_string()),
                )]));
            }
            Ok(true) => {}
        }

        let task_id = Uuid::new_v4();
        let create_task_data = CreateTask {
            project_id: project_uuid,
            title: title.clone(),
            description: description.clone(),
        };

        match Task::create(&self.pool, &create_task_data, task_id).await {
            Ok(_task) => {
                let success_response = CreateTaskResponse {
                    success: true,
                    task_id: task_id.to_string(),
                    message: "Task created successfully".to_string(),
                };
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&success_response)
                        .unwrap_or_else(|_| "Task created successfully".to_string()),
                )]))
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to create task",
                    "details": e.to_string(),
                    "project_id": project_id,
                    "title": title
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Failed to create task".to_string()),
                )]))
            }
        }
    }

    #[tool(description = "List all the available projects")]
    async fn list_projects(
        &self,
        #[tool(aggr)] _request: ListProjectsRequest,
    ) -> Result<CallToolResult, RmcpError> {
        match Project::find_all(&self.pool).await {
            Ok(projects) => {
                let count = projects.len();
                let project_summaries: Vec<ProjectSummary> = projects
                    .into_iter()
                    .map(|project| {
                        let project_with_branch = project.with_branch_info();
                        ProjectSummary {
                            id: project_with_branch.id.to_string(),
                            name: project_with_branch.name,
                            git_repo_path: project_with_branch.git_repo_path,
                            setup_script: project_with_branch.setup_script,
                            dev_script: project_with_branch.dev_script,
                            current_branch: project_with_branch.current_branch,
                            created_at: project_with_branch.created_at.to_rfc3339(),
                            updated_at: project_with_branch.updated_at.to_rfc3339(),
                        }
                    })
                    .collect();

                let response = ListProjectsResponse {
                    success: true,
                    projects: project_summaries,
                    count,
                };

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|_| "Failed to serialize projects".to_string()),
                )]))
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to retrieve projects",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Database error".to_string()),
                )]))
            }
        }
    }

    #[tool(
        description = "List all the task/tickets in a project with optional filtering and execution status. `project_id` is required!"
    )]
    async fn list_tasks(
        &self,
        #[tool(aggr)] ListTasksRequest {
            project_id,
            status,
            limit,
        }: ListTasksRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid project ID format. Must be a valid UUID.",
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Invalid project ID format".to_string()),
                )]));
            }
        };

        let status_filter = if let Some(ref status_str) = status {
            match parse_task_status(status_str) {
                Some(status) => Some(status),
                None => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": "Invalid status filter. Valid values: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'",
                        "provided_status": status_str
                    });
                    return Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&error_response)
                            .unwrap_or_else(|_| "Invalid status filter".to_string()),
                    )]));
                }
            }
        } else {
            None
        };

        let project = match Project::find_by_id(&self.pool, project_uuid).await {
            Ok(Some(project)) => project,
            Ok(None) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Project not found",
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Project not found".to_string()),
                )]));
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to check project existence",
                    "details": e.to_string(),
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Database error".to_string()),
                )]));
            }
        };

        let task_limit = limit.unwrap_or(50).clamp(1, 200); // Reasonable limits

        let tasks_result =
            Task::find_by_project_id_with_attempt_status(&self.pool, project_uuid).await;

        match tasks_result {
            Ok(tasks) => {
                let filtered_tasks: Vec<_> = tasks
                    .into_iter()
                    .filter(|task| {
                        if let Some(ref filter_status) = status_filter {
                            &task.status == filter_status
                        } else {
                            true
                        }
                    })
                    .take(task_limit as usize)
                    .collect();

                let task_summaries: Vec<TaskSummary> = filtered_tasks
                    .into_iter()
                    .map(|task| TaskSummary {
                        id: task.id.to_string(),
                        title: task.title,
                        description: task.description,
                        status: task_status_to_string(&task.status),
                        created_at: task.created_at.to_rfc3339(),
                        updated_at: task.updated_at.to_rfc3339(),
                        has_in_progress_attempt: Some(task.has_in_progress_attempt),
                        has_merged_attempt: Some(task.has_merged_attempt),
                        has_failed_attempt: Some(task.has_failed_attempt),
                    })
                    .collect();

                let count = task_summaries.len();
                let response = ListTasksResponse {
                    success: true,
                    tasks: task_summaries,
                    count,
                    project_id: project_id.clone(),
                    project_name: Some(project.name),
                    applied_filters: ListTasksFilters {
                        status: status.clone(),
                        limit: task_limit,
                    },
                };

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response)
                        .unwrap_or_else(|_| "Failed to serialize tasks".to_string()),
                )]))
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to retrieve tasks",
                    "details": e.to_string(),
                    "project_id": project_id
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Database error".to_string()),
                )]))
            }
        }
    }

    #[tool(
        description = "Update an existing task/ticket's title, description, or status. `project_id` and `task_id` are required! `title`, `description`, and `status` are optional."
    )]
    async fn update_task(
        &self,
        #[tool(aggr)] UpdateTaskRequest {
            project_id,
            task_id,
            title,
            description,
            status,
        }: UpdateTaskRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid project ID format. Must be a valid UUID.",
                    "project_id": project_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        let task_uuid = match Uuid::parse_str(&task_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid task ID format. Must be a valid UUID.",
                    "task_id": task_id
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        let status_enum = if let Some(ref status_str) = status {
            match parse_task_status(status_str) {
                Some(status) => Some(status),
                None => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": "Invalid status. Valid values: 'todo', 'inprogress', 'inreview', 'done', 'cancelled'",
                        "provided_status": status_str
                    });
                    return Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&error_response).unwrap(),
                    )]));
                }
            }
        } else {
            None
        };

        let current_task =
            match Task::find_by_id_and_project_id(&self.pool, task_uuid, project_uuid).await {
                Ok(Some(task)) => task,
                Ok(None) => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": "Task not found in the specified project",
                        "task_id": task_id,
                        "project_id": project_id
                    });
                    return Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&error_response).unwrap(),
                    )]));
                }
                Err(e) => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": "Failed to retrieve task",
                        "details": e.to_string()
                    });
                    return Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&error_response).unwrap(),
                    )]));
                }
            };

        let new_title = title.unwrap_or(current_task.title);
        let new_description = description.or(current_task.description);
        let new_status = status_enum.unwrap_or(current_task.status);

        match Task::update(
            &self.pool,
            task_uuid,
            project_uuid,
            new_title,
            new_description,
            new_status,
        )
        .await
        {
            Ok(updated_task) => {
                let task_summary = TaskSummary {
                    id: updated_task.id.to_string(),
                    title: updated_task.title,
                    description: updated_task.description,
                    status: task_status_to_string(&updated_task.status),
                    created_at: updated_task.created_at.to_rfc3339(),
                    updated_at: updated_task.updated_at.to_rfc3339(),
                    has_in_progress_attempt: None,
                    has_merged_attempt: None,
                    has_failed_attempt: None,
                };

                let response = UpdateTaskResponse {
                    success: true,
                    message: "Task updated successfully".to_string(),
                    task: Some(task_summary),
                };

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to update task",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]))
            }
        }
    }

    #[tool(
        description = "Delete a task/ticket from a project. `project_id` and `task_id` are required!"
    )]
    async fn delete_task(
        &self,
        #[tool(aggr)] DeleteTaskRequest {
            project_id,
            task_id,
        }: DeleteTaskRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid project ID format"
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        let task_uuid = match Uuid::parse_str(&task_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid task ID format"
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        match Task::exists(&self.pool, task_uuid, project_uuid).await {
            Ok(true) => {
                // Delete the task
                match Task::delete(&self.pool, task_uuid, project_uuid).await {
                    Ok(rows_affected) => {
                        if rows_affected > 0 {
                            let response = DeleteTaskResponse {
                                success: true,
                                message: "Task deleted successfully".to_string(),
                                deleted_task_id: Some(task_id),
                            };
                            Ok(CallToolResult::success(vec![Content::text(
                                serde_json::to_string_pretty(&response).unwrap(),
                            )]))
                        } else {
                            let error_response = serde_json::json!({
                                "success": false,
                                "error": "Task not found or already deleted"
                            });
                            Ok(CallToolResult::error(vec![Content::text(
                                serde_json::to_string_pretty(&error_response).unwrap(),
                            )]))
                        }
                    }
                    Err(e) => {
                        let error_response = serde_json::json!({
                            "success": false,
                            "error": "Failed to delete task",
                            "details": e.to_string()
                        });
                        Ok(CallToolResult::error(vec![Content::text(
                            serde_json::to_string_pretty(&error_response).unwrap(),
                        )]))
                    }
                }
            }
            Ok(false) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Task not found in the specified project"
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]))
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to check task existence",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]))
            }
        }
    }

    #[tool(
        description = "Get detailed information about a specific task/ticket. `project_id` and `task_id` are required!"
    )]
    async fn get_task(
        &self,
        #[tool(aggr)] GetTaskRequest {
            project_id,
            task_id,
        }: GetTaskRequest,
    ) -> Result<CallToolResult, RmcpError> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid project ID format"
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        let task_uuid = match Uuid::parse_str(&task_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Invalid task ID format"
                });
                return Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]));
            }
        };

        let task_result =
            Task::find_by_id_and_project_id(&self.pool, task_uuid, project_uuid).await;
        let project_result = Project::find_by_id(&self.pool, project_uuid).await;

        match (task_result, project_result) {
            (Ok(Some(task)), Ok(Some(project))) => {
                let task_summary = TaskSummary {
                    id: task.id.to_string(),
                    title: task.title,
                    description: task.description,
                    status: task_status_to_string(&task.status),
                    created_at: task.created_at.to_rfc3339(),
                    updated_at: task.updated_at.to_rfc3339(),
                    has_in_progress_attempt: None,
                    has_merged_attempt: None,
                    has_failed_attempt: None,
                };

                let response = GetTaskResponse {
                    success: true,
                    task: Some(task_summary),
                    project_name: Some(project.name),
                };

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            (Ok(None), _) | (_, Ok(None)) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Task or project not found"
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]))
            }
            (Err(e), _) | (_, Err(e)) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "Failed to retrieve task or project",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response).unwrap(),
                )]))
            }
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for TaskServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "codecommand".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("A task and project management server. If you need to create or update tickets or tasks then use these tools. Most of them absolutely require that you pass the `project_id` of the project that you are currently working on. This should be provided to you. TOOLS: 'list_projects', 'list_tasks', 'create_task', 'get_task', 'update_task', 'delete_task'. Make sure to pass `project_id` or `task_id` where required. You can use list tools to get the available ids.".to_string()),
        }
    }
}
