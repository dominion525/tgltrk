use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{Project, Tag, TimeEntry, User};

// --- Response types (API → domain) ---

#[derive(Debug, Deserialize)]
pub struct WireUser {
    pub email: String,
    pub fullname: String,
    pub default_workspace_id: i64,
    pub timezone: String,
}

impl From<WireUser> for User {
    fn from(w: WireUser) -> Self {
        User {
            email: w.email,
            fullname: w.fullname,
            default_workspace_id: w.default_workspace_id,
            timezone: w.timezone,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WireTimeEntry {
    pub id: i64,
    pub workspace_id: i64,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub stop: Option<DateTime<Utc>>,
    pub duration: i64,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub billable: bool,
}

impl From<WireTimeEntry> for TimeEntry {
    fn from(w: WireTimeEntry) -> Self {
        TimeEntry {
            id: w.id,
            workspace_id: w.workspace_id,
            description: w.description,
            start: w.start,
            stop: w.stop,
            duration: w.duration,
            project_id: w.project_id,
            task_id: w.task_id,
            tags: w.tags.unwrap_or_default(),
            billable: w.billable,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WireProject {
    pub id: i64,
    pub workspace_id: i64,
    pub name: String,
    pub active: bool,
    pub color: String,
    pub billable: Option<bool>,
}

impl From<WireProject> for Project {
    fn from(w: WireProject) -> Self {
        Project {
            id: w.id,
            workspace_id: w.workspace_id,
            name: w.name,
            active: w.active,
            color: w.color,
            billable: w.billable,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WireTag {
    pub id: i64,
    pub workspace_id: i64,
    pub name: String,
}

impl From<WireTag> for Tag {
    fn from(w: WireTag) -> Self {
        Tag {
            id: w.id,
            workspace_id: w.workspace_id,
            name: w.name,
        }
    }
}

// --- Request types (domain → API) ---

#[derive(Debug, Serialize)]
pub struct CreateTimeEntryRequest {
    pub workspace_id: i64,
    pub description: Option<String>,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub tags: Vec<String>,
    pub billable: bool,
    pub start: DateTime<Utc>,
    pub duration: i64,
    pub created_with: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateTimeEntryRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billable: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Serialize)]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTagRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateTagRequest {
    pub name: String,
}
