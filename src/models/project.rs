use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub workspace_id: i64,
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
            id: 1,
            workspace_id: 1,
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
