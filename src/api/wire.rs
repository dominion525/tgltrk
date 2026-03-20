use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{
    Client, ClientId, Project, ProjectId, Tag, TagId, TaskId, TimeEntry, TimeEntryId, User,
    WorkspaceId,
};

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
            default_workspace_id: WorkspaceId(w.default_workspace_id),
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
            id: TimeEntryId(w.id),
            workspace_id: WorkspaceId(w.workspace_id),
            description: w.description,
            start: w.start,
            stop: w.stop,
            duration: w.duration,
            project_id: w.project_id,
            task_id: w.task_id.map(TaskId),
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
    pub client_id: Option<i64>,
}

impl From<WireProject> for Project {
    fn from(w: WireProject) -> Self {
        Project {
            id: ProjectId(w.id),
            workspace_id: WorkspaceId(w.workspace_id),
            name: w.name,
            active: w.active,
            color: w.color,
            billable: w.billable,
            client_id: w.client_id.map(ClientId),
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
            id: TagId(w.id),
            workspace_id: WorkspaceId(w.workspace_id),
            name: w.name,
        }
    }
}

// --- Request types (domain → API) ---

#[derive(Debug, Serialize)]
pub struct CreateTimeEntryRequest {
    pub workspace_id: WorkspaceId,
    pub description: Option<String>,
    pub project_id: Option<i64>,
    pub task_id: Option<TaskId>,
    pub tags: Vec<String>,
    pub billable: bool,
    pub start: DateTime<Utc>,
    pub duration: i64,
    pub created_with: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<DateTime<Utc>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<i64>,
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

#[derive(Debug, Deserialize)]
pub struct WireClient {
    pub id: i64,
    pub wid: i64,
    pub name: String,
}

impl From<WireClient> for Client {
    fn from(w: WireClient) -> Self {
        Client {
            id: ClientId(w.id),
            workspace_id: WorkspaceId(w.wid),
            name: w.name,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CreateClientRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateClientRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CreateTagRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateTagRequest {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn wire_user_to_user() {
        let w = WireUser {
            email: "a@b.com".to_string(),
            fullname: "Alice".to_string(),
            default_workspace_id: 42,
            timezone: "UTC".to_string(),
        };
        let u: User = w.into();
        assert_eq!(u.email, "a@b.com");
        assert_eq!(u.fullname, "Alice");
        assert_eq!(u.default_workspace_id, WorkspaceId(42));
        assert_eq!(u.timezone, "UTC");
    }

    #[test]
    fn wire_time_entry_with_tags() {
        let now = Utc::now();
        let w = WireTimeEntry {
            id: 1,
            workspace_id: 2,
            description: Some("desc".to_string()),
            start: now,
            stop: Some(now),
            duration: 3600,
            project_id: Some(10),
            task_id: Some(20),
            tags: Some(vec!["a".to_string()]),
            billable: true,
        };
        let e: TimeEntry = w.into();
        assert_eq!(e.tags, vec!["a".to_string()]);
    }

    #[test]
    fn wire_time_entry_without_tags() {
        let now = Utc::now();
        let w = WireTimeEntry {
            id: 1,
            workspace_id: 2,
            description: None,
            start: now,
            stop: None,
            duration: -1,
            project_id: None,
            task_id: None,
            tags: None,
            billable: false,
        };
        let e: TimeEntry = w.into();
        assert!(e.tags.is_empty());
    }

    #[test]
    fn wire_time_entry_preserves_optionals() {
        let now = Utc::now();
        let w = WireTimeEntry {
            id: 1,
            workspace_id: 2,
            description: None,
            start: now,
            stop: None,
            duration: 100,
            project_id: None,
            task_id: None,
            tags: None,
            billable: false,
        };
        let e: TimeEntry = w.into();
        assert!(e.description.is_none());
        assert!(e.stop.is_none());
        assert!(e.project_id.is_none());
    }

    #[test]
    fn wire_project_to_project() {
        let w = WireProject {
            id: 5,
            workspace_id: 3,
            name: "Proj".to_string(),
            active: true,
            color: "#fff".to_string(),
            billable: Some(true),
            client_id: None,
        };
        let p: Project = w.into();
        assert_eq!(p.id, ProjectId(5));
        assert_eq!(p.workspace_id, WorkspaceId(3));
        assert_eq!(p.name, "Proj");
        assert!(p.active);
        assert_eq!(p.color, "#fff");
        assert_eq!(p.billable, Some(true));
    }

    #[test]
    fn wire_tag_to_tag() {
        let w = WireTag {
            id: 7,
            workspace_id: 4,
            name: "urgent".to_string(),
        };
        let t: Tag = w.into();
        assert_eq!(t.id, TagId(7));
        assert_eq!(t.workspace_id, WorkspaceId(4));
        assert_eq!(t.name, "urgent");
    }
}
