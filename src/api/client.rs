use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use reqwest::{RequestBuilder, header};
use serde::Serialize;
use serde::de::DeserializeOwned;

use std::time::Duration;

use crate::constants::{API_BASE_URL, API_TIMEOUT_SECS};
use crate::error::{AppError, Result};
use crate::models::{
    ClientId, Project, ProjectId, Tag, TagId, TaskId, TimeEntry, TimeEntryId, User, WorkspaceId,
};

use super::wire::{
    CreateClientRequest, CreateProjectRequest, CreateTagRequest, CreateTimeEntryRequest,
    UpdateClientRequest, UpdateProjectRequest, UpdateTagRequest, UpdateTimeEntryRequest,
    WireClient, WireProject, WireTag, WireTimeEntry, WireUser, WireWorkspace,
};

#[cfg(test)]
use mockall::automock;

// --- Param types ---

pub struct CreateTimeEntryParams {
    pub description: Option<String>,
    pub project_id: Option<i64>,
    pub task_id: Option<TaskId>,
    pub tags: Vec<String>,
    pub billable: bool,
    pub start: Option<DateTime<Utc>>,
    pub stop: Option<DateTime<Utc>>,
    pub duration: Option<i64>,
}

impl From<&TimeEntry> for CreateTimeEntryParams {
    fn from(entry: &TimeEntry) -> Self {
        Self {
            description: entry.description.clone(),
            project_id: entry.project_id,
            task_id: entry.task_id,
            tags: entry.tags.clone(),
            billable: entry.billable,
            start: None,
            stop: None,
            duration: None,
        }
    }
}

pub struct UpdateTimeEntryParams {
    pub description: Option<String>,
    pub project_id: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub billable: Option<bool>,
    pub start: Option<DateTime<Utc>>,
    pub stop: Option<DateTime<Utc>>,
    pub duration: Option<i64>,
}

pub struct CreateProjectParams {
    pub name: String,
    pub client_id: Option<i64>,
}

pub struct UpdateProjectParams {
    pub name: Option<String>,
    pub client_id: Option<i64>,
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
    async fn get_time_entry(&self, entry_id: TimeEntryId) -> Result<TimeEntry>;
    async fn create_time_entry(
        &self,
        workspace_id: WorkspaceId,
        params: &CreateTimeEntryParams,
    ) -> Result<TimeEntry>;
    async fn update_time_entry(
        &self,
        workspace_id: WorkspaceId,
        entry_id: TimeEntryId,
        params: &UpdateTimeEntryParams,
    ) -> Result<TimeEntry>;
    async fn delete_time_entry(
        &self,
        workspace_id: WorkspaceId,
        entry_id: TimeEntryId,
    ) -> Result<()>;
    async fn stop_time_entry(
        &self,
        workspace_id: WorkspaceId,
        entry_id: TimeEntryId,
    ) -> Result<TimeEntry>;
    async fn list_projects(&self, workspace_id: WorkspaceId) -> Result<Vec<Project>>;
    async fn get_project(
        &self,
        workspace_id: WorkspaceId,
        project_id: ProjectId,
    ) -> Result<Project>;
    async fn create_project(
        &self,
        workspace_id: WorkspaceId,
        params: &CreateProjectParams,
    ) -> Result<Project>;
    async fn update_project(
        &self,
        workspace_id: WorkspaceId,
        project_id: ProjectId,
        params: &UpdateProjectParams,
    ) -> Result<Project>;
    async fn delete_project(&self, workspace_id: WorkspaceId, project_id: ProjectId) -> Result<()>;
    async fn list_tags(&self, workspace_id: WorkspaceId) -> Result<Vec<Tag>>;
    async fn create_tag(&self, workspace_id: WorkspaceId, name: &str) -> Result<Tag>;
    async fn update_tag(&self, workspace_id: WorkspaceId, tag_id: TagId, name: &str)
    -> Result<Tag>;
    async fn delete_tag(&self, workspace_id: WorkspaceId, tag_id: TagId) -> Result<()>;
    async fn list_clients(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<crate::models::Client>>;
    async fn create_client(
        &self,
        workspace_id: WorkspaceId,
        name: &str,
    ) -> Result<crate::models::Client>;
    async fn update_client(
        &self,
        workspace_id: WorkspaceId,
        client_id: ClientId,
        name: &str,
    ) -> Result<crate::models::Client>;
    async fn delete_client(
        &self,
        workspace_id: WorkspaceId,
        client_id: ClientId,
    ) -> Result<()>;
    async fn get_client(
        &self,
        workspace_id: WorkspaceId,
        client_id: ClientId,
    ) -> Result<crate::models::Client>;
    async fn list_workspaces(&self) -> Result<Vec<crate::models::Workspace>>;
    async fn get_workspace(&self, workspace_id: WorkspaceId) -> Result<crate::models::Workspace>;
}

pub struct TogglClient {
    http: reqwest::Client,
    base_url: String,
}

impl TogglClient {
    fn build(api_token: &str, base_url: String) -> Result<Self> {
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

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(API_TIMEOUT_SECS))
            .user_agent(format!("tgltrk/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| AppError::Api(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self { http, base_url })
    }

    pub fn new(api_token: &str) -> Result<Self> {
        Self::build(api_token, API_BASE_URL.to_string())
    }

    pub fn new_with_base_url(api_token: &str, base_url: &str) -> Result<Self> {
        Self::build(api_token, base_url.to_string())
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

    async fn check_response(&self, response: reqwest::Response) -> Result<reqwest::Response> {
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }
        Ok(response)
    }

    async fn send<T: DeserializeOwned>(&self, request: RequestBuilder) -> Result<T> {
        let response = self.check_response(request.send().await?).await?;
        let parsed = response.json::<T>().await?;
        Ok(parsed)
    }

    async fn delete_request(&self, url: &str) -> Result<()> {
        self.check_response(self.http.delete(url).send().await?)
            .await?;
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
        let text = text.trim();
        if text == "null" || text.is_empty() {
            return Ok(None);
        }
        let wire: WireTimeEntry = serde_json::from_str(text)?;
        Ok(Some(wire.into()))
    }

    async fn get_time_entries(
        &self,
        since: Option<String>,
        until: Option<String>,
    ) -> Result<Vec<TimeEntry>> {
        let url = format!("{}/me/time_entries", self.base_url);
        let mut request = self.http.get(&url);
        if let Some(s) = &since {
            request = request.query(&[("start_date", s)]);
        }
        if let Some(u) = &until {
            request = request.query(&[("end_date", u)]);
        }
        let response = self.check_response(request.send().await?).await?;
        let wire: Vec<WireTimeEntry> = response.json().await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    async fn get_time_entry(&self, entry_id: TimeEntryId) -> Result<TimeEntry> {
        let url = format!("{}/me/time_entries/{entry_id}", self.base_url);
        let wire: WireTimeEntry = self.get(&url).await?;
        Ok(wire.into())
    }

    async fn create_time_entry(
        &self,
        workspace_id: WorkspaceId,
        params: &CreateTimeEntryParams,
    ) -> Result<TimeEntry> {
        let url = format!("{}/workspaces/{workspace_id}/time_entries", self.base_url);
        let now = Utc::now();
        let start = params.start.unwrap_or(now);
        let duration = params.duration.unwrap_or(-start.timestamp());
        let body = CreateTimeEntryRequest {
            workspace_id,
            description: params.description.clone(),
            project_id: params.project_id,
            task_id: params.task_id,
            tags: params.tags.clone(),
            billable: params.billable,
            start,
            duration,
            created_with: crate::constants::CLIENT_NAME.to_string(),
            stop: params.stop,
        };
        let wire: WireTimeEntry = self.post(&url, &body).await?;
        Ok(wire.into())
    }

    async fn update_time_entry(
        &self,
        workspace_id: WorkspaceId,
        entry_id: TimeEntryId,
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
            start: params.start,
            stop: params.stop,
            duration: params.duration,
        };
        let wire: WireTimeEntry = self.put(&url, &body).await?;
        Ok(wire.into())
    }

    async fn delete_time_entry(
        &self,
        workspace_id: WorkspaceId,
        entry_id: TimeEntryId,
    ) -> Result<()> {
        let url = format!(
            "{}/workspaces/{workspace_id}/time_entries/{entry_id}",
            self.base_url
        );
        self.delete_request(&url).await
    }

    async fn stop_time_entry(
        &self,
        workspace_id: WorkspaceId,
        entry_id: TimeEntryId,
    ) -> Result<TimeEntry> {
        let url = format!(
            "{}/workspaces/{workspace_id}/time_entries/{entry_id}/stop",
            self.base_url
        );
        let wire: WireTimeEntry = self.patch(&url, &serde_json::json!({})).await?;
        Ok(wire.into())
    }

    async fn list_projects(&self, workspace_id: WorkspaceId) -> Result<Vec<Project>> {
        let url = format!("{}/workspaces/{workspace_id}/projects", self.base_url);
        let wire: Vec<WireProject> = self.get(&url).await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    async fn get_project(
        &self,
        workspace_id: WorkspaceId,
        project_id: ProjectId,
    ) -> Result<Project> {
        let url = format!(
            "{}/workspaces/{workspace_id}/projects/{project_id}",
            self.base_url
        );
        let wire: WireProject = self.get(&url).await?;
        Ok(wire.into())
    }

    async fn create_project(
        &self,
        workspace_id: WorkspaceId,
        params: &CreateProjectParams,
    ) -> Result<Project> {
        let url = format!("{}/workspaces/{workspace_id}/projects", self.base_url);
        let body = CreateProjectRequest {
            name: params.name.clone(),
            active: true,
            client_id: params.client_id,
        };
        let wire: WireProject = self.post(&url, &body).await?;
        Ok(wire.into())
    }

    async fn update_project(
        &self,
        workspace_id: WorkspaceId,
        project_id: ProjectId,
        params: &UpdateProjectParams,
    ) -> Result<Project> {
        let url = format!(
            "{}/workspaces/{workspace_id}/projects/{project_id}",
            self.base_url
        );
        let body = UpdateProjectRequest {
            name: params.name.clone(),
            client_id: params.client_id,
        };
        let wire: WireProject = self.put(&url, &body).await?;
        Ok(wire.into())
    }

    async fn delete_project(&self, workspace_id: WorkspaceId, project_id: ProjectId) -> Result<()> {
        let url = format!(
            "{}/workspaces/{workspace_id}/projects/{project_id}",
            self.base_url
        );
        self.delete_request(&url).await
    }

    async fn list_tags(&self, workspace_id: WorkspaceId) -> Result<Vec<Tag>> {
        let url = format!("{}/workspaces/{workspace_id}/tags", self.base_url);
        let wire: Vec<WireTag> = self.get(&url).await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    async fn create_tag(&self, workspace_id: WorkspaceId, name: &str) -> Result<Tag> {
        let url = format!("{}/workspaces/{workspace_id}/tags", self.base_url);
        let body = CreateTagRequest {
            name: name.to_string(),
        };
        let wire: WireTag = self.post(&url, &body).await?;
        Ok(wire.into())
    }

    async fn update_tag(
        &self,
        workspace_id: WorkspaceId,
        tag_id: TagId,
        name: &str,
    ) -> Result<Tag> {
        let url = format!("{}/workspaces/{workspace_id}/tags/{tag_id}", self.base_url);
        let body = UpdateTagRequest {
            name: name.to_string(),
        };
        let wire: WireTag = self.put(&url, &body).await?;
        Ok(wire.into())
    }

    async fn delete_tag(&self, workspace_id: WorkspaceId, tag_id: TagId) -> Result<()> {
        let url = format!("{}/workspaces/{workspace_id}/tags/{tag_id}", self.base_url);
        self.delete_request(&url).await
    }

    async fn list_clients(
        &self,
        workspace_id: WorkspaceId,
    ) -> Result<Vec<crate::models::Client>> {
        let url = format!("{}/workspaces/{workspace_id}/clients", self.base_url);
        let wire: Vec<WireClient> = self.get(&url).await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    async fn create_client(
        &self,
        workspace_id: WorkspaceId,
        name: &str,
    ) -> Result<crate::models::Client> {
        let url = format!("{}/workspaces/{workspace_id}/clients", self.base_url);
        let body = CreateClientRequest {
            name: name.to_string(),
        };
        let wire: WireClient = self.post(&url, &body).await?;
        Ok(wire.into())
    }

    async fn update_client(
        &self,
        workspace_id: WorkspaceId,
        client_id: ClientId,
        name: &str,
    ) -> Result<crate::models::Client> {
        let url = format!(
            "{}/workspaces/{workspace_id}/clients/{client_id}",
            self.base_url
        );
        let body = UpdateClientRequest {
            name: name.to_string(),
        };
        let wire: WireClient = self.put(&url, &body).await?;
        Ok(wire.into())
    }

    async fn delete_client(
        &self,
        workspace_id: WorkspaceId,
        client_id: ClientId,
    ) -> Result<()> {
        let url = format!(
            "{}/workspaces/{workspace_id}/clients/{client_id}",
            self.base_url
        );
        self.delete_request(&url).await
    }

    async fn get_client(
        &self,
        workspace_id: WorkspaceId,
        client_id: ClientId,
    ) -> Result<crate::models::Client> {
        let url = format!(
            "{}/workspaces/{workspace_id}/clients/{client_id}",
            self.base_url
        );
        let wire: WireClient = self.get(&url).await?;
        Ok(wire.into())
    }

    async fn list_workspaces(&self) -> Result<Vec<crate::models::Workspace>> {
        let url = format!("{}/me/workspaces", self.base_url);
        let wire: Vec<WireWorkspace> = self.get(&url).await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    async fn get_workspace(&self, workspace_id: WorkspaceId) -> Result<crate::models::Workspace> {
        let url = format!("{}/workspaces/{workspace_id}", self.base_url);
        let wire: WireWorkspace = self.get(&url).await?;
        Ok(wire.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup() -> (MockServer, TogglClient) {
        let server = MockServer::start().await;
        let client = TogglClient::new_with_base_url("test_token", &server.uri()).unwrap();
        (server, client)
    }

    fn wire_user_json() -> serde_json::Value {
        serde_json::json!({
            "email": "test@example.com",
            "fullname": "Test User",
            "default_workspace_id": 1,
            "timezone": "UTC"
        })
    }

    fn wire_time_entry_json(id: i64, workspace_id: i64) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "workspace_id": workspace_id,
            "description": "Test entry",
            "start": "2024-01-01T00:00:00Z",
            "stop": "2024-01-01T01:00:00Z",
            "duration": 3600,
            "project_id": null,
            "task_id": null,
            "tags": [],
            "billable": false
        })
    }

    fn wire_project_json(id: i64, workspace_id: i64) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "workspace_id": workspace_id,
            "name": "Test Project",
            "active": true,
            "color": "#06aaf5",
            "billable": null
        })
    }

    fn wire_tag_json(id: i64, workspace_id: i64) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "workspace_id": workspace_id,
            "name": "Test Tag"
        })
    }

    #[test]
    fn new_with_valid_token_succeeds() {
        assert!(TogglClient::new("valid_token").is_ok());
    }

    #[test]
    fn new_with_empty_token_succeeds() {
        assert!(TogglClient::new("").is_ok());
    }

    // --- get_me ---

    #[tokio::test]
    async fn get_me_returns_user() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wire_user_json()))
            .mount(&server)
            .await;

        let user = client.get_me().await.unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.fullname, "Test User");
        assert_eq!(user.default_workspace_id, WorkspaceId(1));
    }

    // --- get_current_timer ---

    #[tokio::test]
    async fn get_current_timer_running() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/me/time_entries/current"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wire_time_entry_json(42, 1)))
            .mount(&server)
            .await;

        let result = client.get_current_timer().await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, TimeEntryId(42));
    }

    #[tokio::test]
    async fn get_current_timer_null() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/me/time_entries/current"))
            .respond_with(ResponseTemplate::new(200).set_body_string("null"))
            .mount(&server)
            .await;

        let result = client.get_current_timer().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_current_timer_empty() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/me/time_entries/current"))
            .respond_with(ResponseTemplate::new(200).set_body_string(""))
            .mount(&server)
            .await;

        let result = client.get_current_timer().await.unwrap();
        assert!(result.is_none());
    }

    // --- get_time_entries ---

    #[tokio::test]
    async fn get_time_entries_no_params() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/me/time_entries"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([wire_time_entry_json(1, 1)])),
            )
            .mount(&server)
            .await;

        let entries = client.get_time_entries(None, None).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, TimeEntryId(1));
    }

    #[tokio::test]
    async fn get_time_entries_with_since_until() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/me/time_entries"))
            .and(query_param("start_date", "2024-01-01"))
            .and(query_param("end_date", "2024-01-31"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([wire_time_entry_json(1, 1)])),
            )
            .mount(&server)
            .await;

        let entries = client
            .get_time_entries(
                Some("2024-01-01".to_string()),
                Some("2024-01-31".to_string()),
            )
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
    }

    // --- get_time_entry ---

    #[tokio::test]
    async fn get_time_entry_by_id() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/me/time_entries/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wire_time_entry_json(42, 1)))
            .mount(&server)
            .await;

        let entry = client.get_time_entry(TimeEntryId(42)).await.unwrap();
        assert_eq!(entry.id, TimeEntryId(42));
    }

    // --- create_time_entry ---

    #[tokio::test]
    async fn create_time_entry_sends_post() {
        let (server, client) = setup().await;
        Mock::given(method("POST"))
            .and(path("/workspaces/1/time_entries"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wire_time_entry_json(100, 1)))
            .mount(&server)
            .await;

        let params = CreateTimeEntryParams {
            description: Some("Test".to_string()),
            project_id: None,
            task_id: None,
            tags: vec![],
            billable: false,
            start: None,
            stop: None,
            duration: None,
        };
        let entry = client
            .create_time_entry(WorkspaceId(1), &params)
            .await
            .unwrap();
        assert_eq!(entry.id, TimeEntryId(100));
    }

    // --- update_time_entry ---

    #[tokio::test]
    async fn update_time_entry_sends_put() {
        let (server, client) = setup().await;
        Mock::given(method("PUT"))
            .and(path("/workspaces/1/time_entries/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wire_time_entry_json(42, 1)))
            .mount(&server)
            .await;

        let params = UpdateTimeEntryParams {
            description: Some("Updated".to_string()),
            project_id: None,
            tags: None,
            billable: None,
            start: None,
            stop: None,
            duration: None,
        };
        let entry = client
            .update_time_entry(WorkspaceId(1), TimeEntryId(42), &params)
            .await
            .unwrap();
        assert_eq!(entry.id, TimeEntryId(42));
    }

    // --- delete_time_entry ---

    #[tokio::test]
    async fn delete_time_entry_sends_delete() {
        let (server, client) = setup().await;
        Mock::given(method("DELETE"))
            .and(path("/workspaces/1/time_entries/42"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let result = client
            .delete_time_entry(WorkspaceId(1), TimeEntryId(42))
            .await;
        assert!(result.is_ok());
    }

    // --- stop_time_entry ---

    #[tokio::test]
    async fn stop_time_entry_sends_patch() {
        let (server, client) = setup().await;
        Mock::given(method("PATCH"))
            .and(path("/workspaces/1/time_entries/42/stop"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wire_time_entry_json(42, 1)))
            .mount(&server)
            .await;

        let entry = client
            .stop_time_entry(WorkspaceId(1), TimeEntryId(42))
            .await
            .unwrap();
        assert_eq!(entry.id, TimeEntryId(42));
    }

    // --- projects ---

    #[tokio::test]
    async fn list_projects_returns_vec() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/workspaces/1/projects"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!([wire_project_json(10, 1)])),
            )
            .mount(&server)
            .await;

        let projects = client.list_projects(WorkspaceId(1)).await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, ProjectId(10));
    }

    #[tokio::test]
    async fn get_project_by_id() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/workspaces/1/projects/10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wire_project_json(10, 1)))
            .mount(&server)
            .await;

        let project = client
            .get_project(WorkspaceId(1), ProjectId(10))
            .await
            .unwrap();
        assert_eq!(project.id, ProjectId(10));
    }

    #[tokio::test]
    async fn create_project_sends_post() {
        let (server, client) = setup().await;
        Mock::given(method("POST"))
            .and(path("/workspaces/1/projects"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wire_project_json(11, 1)))
            .mount(&server)
            .await;

        let params = CreateProjectParams {
            name: "New Project".to_string(),
            client_id: None,
        };
        let project = client
            .create_project(WorkspaceId(1), &params)
            .await
            .unwrap();
        assert_eq!(project.id, ProjectId(11));
    }

    #[tokio::test]
    async fn update_project_sends_put() {
        let (server, client) = setup().await;
        Mock::given(method("PUT"))
            .and(path("/workspaces/1/projects/10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wire_project_json(10, 1)))
            .mount(&server)
            .await;

        let params = UpdateProjectParams {
            name: Some("Renamed".to_string()),
            client_id: None,
        };
        let project = client
            .update_project(WorkspaceId(1), ProjectId(10), &params)
            .await
            .unwrap();
        assert_eq!(project.id, ProjectId(10));
    }

    #[tokio::test]
    async fn delete_project_sends_delete() {
        let (server, client) = setup().await;
        Mock::given(method("DELETE"))
            .and(path("/workspaces/1/projects/10"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let result = client.delete_project(WorkspaceId(1), ProjectId(10)).await;
        assert!(result.is_ok());
    }

    // --- tags ---

    #[tokio::test]
    async fn list_tags_returns_vec() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/workspaces/1/tags"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!([wire_tag_json(5, 1)])),
            )
            .mount(&server)
            .await;

        let tags = client.list_tags(WorkspaceId(1)).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].id, TagId(5));
    }

    #[tokio::test]
    async fn create_tag_sends_post() {
        let (server, client) = setup().await;
        Mock::given(method("POST"))
            .and(path("/workspaces/1/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wire_tag_json(6, 1)))
            .mount(&server)
            .await;

        let tag = client.create_tag(WorkspaceId(1), "New Tag").await.unwrap();
        assert_eq!(tag.id, TagId(6));
    }

    #[tokio::test]
    async fn update_tag_sends_put() {
        let (server, client) = setup().await;
        Mock::given(method("PUT"))
            .and(path("/workspaces/1/tags/5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(wire_tag_json(5, 1)))
            .mount(&server)
            .await;

        let tag = client
            .update_tag(WorkspaceId(1), TagId(5), "Renamed")
            .await
            .unwrap();
        assert_eq!(tag.id, TagId(5));
    }

    #[tokio::test]
    async fn delete_tag_sends_delete() {
        let (server, client) = setup().await;
        Mock::given(method("DELETE"))
            .and(path("/workspaces/1/tags/5"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let result = client.delete_tag(WorkspaceId(1), TagId(5)).await;
        assert!(result.is_ok());
    }

    // --- From<&TimeEntry> for CreateTimeEntryParams ---

    #[test]
    fn from_time_entry_to_create_params() {
        use crate::models::TimeEntry;
        use chrono::Utc;

        let now = Utc::now();
        let entry = TimeEntry {
            id: TimeEntryId(42),
            workspace_id: WorkspaceId(1),
            description: Some("My task".to_string()),
            start: now,
            stop: Some(now),
            duration: 3600,
            project_id: Some(10),
            task_id: Some(TaskId(20)),
            tags: vec!["a".to_string(), "b".to_string()],
            billable: true,
        };

        let params = CreateTimeEntryParams::from(&entry);

        assert_eq!(params.description, entry.description);
        assert_eq!(params.project_id, entry.project_id);
        assert_eq!(params.task_id, entry.task_id);
        assert_eq!(params.tags, entry.tags);
        assert_eq!(params.billable, entry.billable);
    }

    // --- error cases ---

    #[tokio::test]
    async fn send_http_401_returns_error() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/me"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let result = client.get_me().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::HttpStatus { status, body } => {
                assert_eq!(status, 401);
                assert_eq!(body, "Unauthorized");
            }
            other => panic!("expected HttpStatus, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_http_500_returns_error() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/me"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let result = client.get_me().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::HttpStatus { status, .. } => assert_eq!(status, 500),
            other => panic!("expected HttpStatus, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_current_timer_http_error() {
        let (server, client) = setup().await;
        Mock::given(method("GET"))
            .and(path("/me/time_entries/current"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&server)
            .await;

        let result = client.get_current_timer().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::HttpStatus { status, .. } => assert_eq!(status, 403),
            other => panic!("expected HttpStatus, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn delete_request_http_error() {
        let (server, client) = setup().await;
        Mock::given(method("DELETE"))
            .and(path("/workspaces/1/tags/1"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&server)
            .await;

        let result = client.delete_tag(WorkspaceId(1), TagId(1)).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::HttpStatus { status, .. } => assert_eq!(status, 404),
            other => panic!("expected HttpStatus, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn connection_refused_returns_api_error() {
        let client = TogglClient::new_with_base_url("test_token", "http://127.0.0.1:1").unwrap();
        let result = client.get_me().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Api(_) => {}
            other => panic!("expected Api error, got: {other:?}"),
        }
    }
}
