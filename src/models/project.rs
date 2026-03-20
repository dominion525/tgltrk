use serde::{Deserialize, Serialize};
use std::fmt;

use super::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectId(pub i64);

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub active: bool,
    pub color: String,
    pub billable: Option<bool>,
}

impl fmt::Display for Project {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.active { "" } else { " (archived)" };
        write!(f, "#{} {}{}", self.id, self.name, status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_project(active: bool) -> Project {
        Project {
            id: ProjectId(1),
            workspace_id: WorkspaceId(1),
            name: "Test".to_string(),
            active,
            color: "#fff".to_string(),
            billable: None,
        }
    }

    #[test]
    fn project_display_active() {
        let output = format!("{}", make_project(true));
        assert!(!output.contains("(archived)"), "got: {output}");
    }

    #[test]
    fn project_display_archived() {
        let output = format!("{}", make_project(false));
        assert!(output.contains("(archived)"), "got: {output}");
    }
}
