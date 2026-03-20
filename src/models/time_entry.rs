use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::WorkspaceId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeEntryId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskId(pub i64);

impl fmt::Display for TimeEntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: TimeEntryId,
    pub workspace_id: WorkspaceId,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub stop: Option<DateTime<Utc>>,
    pub duration: i64,
    pub project_id: Option<i64>,
    pub task_id: Option<TaskId>,
    pub tags: Vec<String>,
    pub billable: bool,
}

impl TimeEntry {
    pub fn is_running(&self) -> bool {
        self.duration < 0
    }

    pub fn display_duration(&self) -> String {
        let secs = if self.is_running() {
            Utc::now()
                .signed_duration_since(self.start)
                .num_seconds()
                .max(0)
        } else {
            self.duration
        };
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        format!("{h:02}:{m:02}:{s:02}")
    }
}

impl fmt::Display for TimeEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let desc = self.description.as_deref().unwrap_or("(no description)");
        let status = if self.is_running() { " [running]" } else { "" };
        write!(
            f,
            "#{} {} {}{}",
            self.id,
            desc,
            self.display_duration(),
            status
        )?;
        if !self.tags.is_empty() {
            write!(f, " [{}]", self.tags.join(", "))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(duration: i64, tags: Vec<String>) -> TimeEntry {
        TimeEntry {
            id: TimeEntryId(1),
            workspace_id: WorkspaceId(1),
            description: Some("Test".to_string()),
            start: Utc::now(),
            stop: if duration >= 0 {
                Some(Utc::now())
            } else {
                None
            },
            duration,
            project_id: None,
            task_id: None,
            tags,
            billable: false,
        }
    }

    #[test]
    fn is_running_negative_duration() {
        let e = make_entry(-1000, vec![]);
        assert!(e.is_running());
    }

    #[test]
    fn is_running_positive_duration() {
        let e = make_entry(3600, vec![]);
        assert!(!e.is_running());
    }

    #[test]
    fn is_running_zero_duration() {
        let e = make_entry(0, vec![]);
        assert!(!e.is_running());
    }

    #[test]
    fn display_duration_stopped() {
        let e = make_entry(3661, vec![]);
        assert_eq!(e.display_duration(), "01:01:01");
    }

    #[test]
    fn display_format_with_tags() {
        let e = make_entry(3600, vec!["a".to_string(), "b".to_string()]);
        let output = format!("{e}");
        assert!(output.contains("[a, b]"), "got: {output}");
    }
}
