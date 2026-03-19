use base64::{Engine as _, engine::general_purpose};
use reqwest::{Client, RequestBuilder, header};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::constants::API_BASE_URL;
use crate::error::{AppError, Result};
use crate::models::{Project, Tag, TimeEntry, User};

use super::wire::{
    CreateProjectRequest, CreateTagRequest, CreateTimeEntryRequest, UpdateProjectRequest,
    UpdateTagRequest, UpdateTimeEntryRequest, WireProject, WireTag, WireTimeEntry, WireUser,
};

#[cfg(test)]
use mockall::automock;

// --- Param types ---

pub struct CreateTimeEntryParams {
    pub description: Option<String>,
    pub project_id: Option<i64>,
    pub task_id: Option<i64>,
    pub tags: Vec<String>,
    pub billable: bool,
}

pub struct UpdateTimeEntryParams {
    pub description: Option<String>,
    pub project_id: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub billable: Option<bool>,
}

pub struct CreateProjectParams {
    pub name: String,
}

pub struct UpdateProjectParams {
    pub name: Option<String>,
}

#[cfg_attr(test, automock)]
pub trait ApiClient {
    async fn get_me(&self) -> Result<User>;
    async fn get_current_timer(&self) -> Result<Option<TimeEntry>>;
    async fn get_time_entries(
        &self,
        since: Option<String>,
        until: Option<String>,
    ) -> Result<Vec<TimeEntry>>;
    async fn get_time_entry(&self, entry_id: i64) -> Result<TimeEntry>;
    async fn create_time_entry(
        &self,
        workspace_id: i64,
        params: &CreateTimeEntryParams,
    ) -> Result<TimeEntry>;
    async fn update_time_entry(
        &self,
        workspace_id: i64,
        entry_id: i64,
        params: &UpdateTimeEntryParams,
    ) -> Result<TimeEntry>;
    async fn delete_time_entry(&self, workspace_id: i64, entry_id: i64) -> Result<()>;
    async fn stop_time_entry(&self, workspace_id: i64, entry_id: i64) -> Result<TimeEntry>;
    async fn list_projects(&self, workspace_id: i64) -> Result<Vec<Project>>;
    async fn get_project(&self, workspace_id: i64, project_id: i64) -> Result<Project>;
    async fn create_project(
        &self,
        workspace_id: i64,
        params: &CreateProjectParams,
    ) -> Result<Project>;
    async fn update_project(
        &self,
        workspace_id: i64,
        project_id: i64,
        params: &UpdateProjectParams,
    ) -> Result<Project>;
    async fn delete_project(&self, workspace_id: i64, project_id: i64) -> Result<()>;
    async fn list_tags(&self, workspace_id: i64) -> Result<Vec<Tag>>;
    async fn create_tag(&self, workspace_id: i64, name: &str) -> Result<Tag>;
    async fn update_tag(&self, workspace_id: i64, tag_id: i64, name: &str) -> Result<Tag>;
    async fn delete_tag(&self, workspace_id: i64, tag_id: i64) -> Result<()>;
}

pub struct TogglClient {
    http: Client,
    base_url: String,
}

impl TogglClient {
    pub fn new(api_token: &str) -> Result<Self> {
        let auth = format!("{api_token}:api_token");
        let encoded = general_purpose::STANDARD.encode(auth);
        let header_value = header::HeaderValue::from_str(&format!("Basic {encoded}"))
            .map_err(|e| AppError::Auth(format!("Invalid token: {e}")))?;

        let mut headers = header::HeaderMap::new();
        headers.insert(header::AUTHORIZATION, header_value);
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let http = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| AppError::Api(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self {
            http,
            base_url: API_BASE_URL.to_string(),
        })
    }

    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        self.send(self.http.get(url)).await
    }

    async fn post<T: DeserializeOwned, B: Serialize>(&self, url: &str, body: &B) -> Result<T> {
        self.send(self.http.post(url).json(body)).await
    }

    async fn put<T: DeserializeOwned, B: Serialize>(&self, url: &str, body: &B) -> Result<T> {
        self.send(self.http.put(url).json(body)).await
    }

    async fn patch<T: DeserializeOwned, B: Serialize>(&self, url: &str, body: &B) -> Result<T> {
        self.send(self.http.patch(url).json(body)).await
    }

    async fn send<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T> {
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }
        let parsed = response.json::<T>().await?;
        Ok(parsed)
    }

    async fn delete_request(&self, url: &str) -> Result<()> {
        let response = self.http.delete(url).send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }
        Ok(())
    }
}

impl ApiClient for TogglClient {
    async fn get_me(&self) -> Result<User> {
        let url = format!("{}/me", self.base_url);
        let wire: WireUser = self.get(&url).await?;
        Ok(wire.into())
    }

    async fn get_current_timer(&self) -> Result<Option<TimeEntry>> {
        let url = format!("{}/me/time_entries/current", self.base_url);
        let response = self.http.get(&url).send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }
        let text = response.text().await?;
        if text == "null" || text.is_empty() {
            return Ok(None);
        }
        let wire: WireTimeEntry = serde_json::from_str(&text)?;
        Ok(Some(wire.into()))
    }

    async fn get_time_entries(
        &self,
        since: Option<String>,
        until: Option<String>,
    ) -> Result<Vec<TimeEntry>> {
        let mut url = format!("{}/me/time_entries", self.base_url);
        let mut params = Vec::new();
        if let Some(s) = &since {
            params.push(format!("start_date={s}"));
        }
        if let Some(u) = &until {
            params.push(format!("end_date={u}"));
        }
        if !params.is_empty() {
            url = format!("{url}?{}", params.join("&"));
        }
        let wire: Vec<WireTimeEntry> = self.get(&url).await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    async fn get_time_entry(&self, entry_id: i64) -> Result<TimeEntry> {
        let url = format!("{}/me/time_entries/{entry_id}", self.base_url);
        let wire: WireTimeEntry = self.get(&url).await?;
        Ok(wire.into())
    }

    async fn create_time_entry(
        &self,
        workspace_id: i64,
        params: &CreateTimeEntryParams,
    ) -> Result<TimeEntry> {
        let url = format!("{}/workspaces/{workspace_id}/time_entries", self.base_url);
        let now = chrono::Utc::now();
        let body = CreateTimeEntryRequest {
            workspace_id,
            description: params.description.clone(),
            project_id: params.project_id,
            task_id: params.task_id,
            tags: params.tags.clone(),
            billable: params.billable,
            start: now,
            duration: -now.timestamp(),
            created_with: crate::constants::CLIENT_NAME.to_string(),
        };
        let wire: WireTimeEntry = self.post(&url, &body).await?;
        Ok(wire.into())
    }

    async fn update_time_entry(
        &self,
        workspace_id: i64,
        entry_id: i64,
        params: &UpdateTimeEntryParams,
    ) -> Result<TimeEntry> {
        let url = format!(
            "{}/workspaces/{workspace_id}/time_entries/{entry_id}",
            self.base_url
        );
        let body = UpdateTimeEntryRequest {
            description: params.description.clone(),
            project_id: params.project_id,
            tags: params.tags.clone(),
            billable: params.billable,
        };
        let wire: WireTimeEntry = self.put(&url, &body).await?;
        Ok(wire.into())
    }

    async fn delete_time_entry(&self, workspace_id: i64, entry_id: i64) -> Result<()> {
        let url = format!(
            "{}/workspaces/{workspace_id}/time_entries/{entry_id}",
            self.base_url
        );
        self.delete_request(&url).await
    }

    async fn stop_time_entry(&self, workspace_id: i64, entry_id: i64) -> Result<TimeEntry> {
        let url = format!(
            "{}/workspaces/{workspace_id}/time_entries/{entry_id}/stop",
            self.base_url
        );
        let wire: WireTimeEntry = self.patch(&url, &serde_json::json!({})).await?;
        Ok(wire.into())
    }

    async fn list_projects(&self, workspace_id: i64) -> Result<Vec<Project>> {
        let url = format!("{}/workspaces/{workspace_id}/projects", self.base_url);
        let wire: Vec<WireProject> = self.get(&url).await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    async fn get_project(&self, workspace_id: i64, project_id: i64) -> Result<Project> {
        let url = format!(
            "{}/workspaces/{workspace_id}/projects/{project_id}",
            self.base_url
        );
        let wire: WireProject = self.get(&url).await?;
        Ok(wire.into())
    }

    async fn create_project(
        &self,
        workspace_id: i64,
        params: &CreateProjectParams,
    ) -> Result<Project> {
        let url = format!("{}/workspaces/{workspace_id}/projects", self.base_url);
        let body = CreateProjectRequest {
            name: params.name.clone(),
            active: true,
        };
        let wire: WireProject = self.post(&url, &body).await?;
        Ok(wire.into())
    }

    async fn update_project(
        &self,
        workspace_id: i64,
        project_id: i64,
        params: &UpdateProjectParams,
    ) -> Result<Project> {
        let url = format!(
            "{}/workspaces/{workspace_id}/projects/{project_id}",
            self.base_url
        );
        let body = UpdateProjectRequest {
            name: params.name.clone(),
        };
        let wire: WireProject = self.put(&url, &body).await?;
        Ok(wire.into())
    }

    async fn delete_project(&self, workspace_id: i64, project_id: i64) -> Result<()> {
        let url = format!(
            "{}/workspaces/{workspace_id}/projects/{project_id}",
            self.base_url
        );
        self.delete_request(&url).await
    }

    async fn list_tags(&self, workspace_id: i64) -> Result<Vec<Tag>> {
        let url = format!("{}/workspaces/{workspace_id}/tags", self.base_url);
        let wire: Vec<WireTag> = self.get(&url).await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    async fn create_tag(&self, workspace_id: i64, name: &str) -> Result<Tag> {
        let url = format!("{}/workspaces/{workspace_id}/tags", self.base_url);
        let body = CreateTagRequest {
            name: name.to_string(),
        };
        let wire: WireTag = self.post(&url, &body).await?;
        Ok(wire.into())
    }

    async fn update_tag(&self, workspace_id: i64, tag_id: i64, name: &str) -> Result<Tag> {
        let url = format!("{}/workspaces/{workspace_id}/tags/{tag_id}", self.base_url);
        let body = UpdateTagRequest {
            name: name.to_string(),
        };
        let wire: WireTag = self.put(&url, &body).await?;
        Ok(wire.into())
    }

    async fn delete_tag(&self, workspace_id: i64, tag_id: i64) -> Result<()> {
        let url = format!("{}/workspaces/{workspace_id}/tags/{tag_id}", self.base_url);
        self.delete_request(&url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_valid_token_succeeds() {
        assert!(TogglClient::new("valid_token").is_ok());
    }

    #[test]
    fn new_with_empty_token_succeeds() {
        assert!(TogglClient::new("").is_ok());
    }
}
