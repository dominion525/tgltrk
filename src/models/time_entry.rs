use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: i64,
    pub workspace_id: i64,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub stop: Option<DateTime<Utc>>,
    pub duration: i64,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub tags: Vec<String>,
    pub billable: bool,
}

impl TimeEntry {
    pub fn is_running(&self) -> bool {
        self.duration < 0
    }

    pub fn display_duration(&self) -> String {
        let secs = if self.is_running() {
            Utc::now().signed_duration_since(self.start).num_seconds()
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
