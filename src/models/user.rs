use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub email: String,
    pub fullname: String,
    pub default_workspace_id: i64,
    pub timezone: String,
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Name:      {}", self.fullname)?;
        writeln!(f, "Email:     {}", self.email)?;
        writeln!(f, "Workspace: {}", self.default_workspace_id)?;
        write!(f, "Timezone:  {}", self.timezone)
    }
}
