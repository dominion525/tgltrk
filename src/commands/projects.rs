use colored::Colorize;

use crate::api::client::{ApiClient, CreateProjectParams, TogglClient, UpdateProjectParams};
use crate::cli::ProjectsAction;
use crate::commands::{cache_get, cache_set, invalidate_cache, resolve_workspace_id};
use crate::credentials;
use crate::error::Result;
use crate::output;

pub async fn execute(action: ProjectsAction, json: bool, workspace: Option<i64>) -> Result<()> {
    let store = credentials::get_store();
    let cred = store.read()?;
    let client = TogglClient::new(&cred.api_token)?;
    run(action, json, workspace, &client).await
}

async fn run(
    action: ProjectsAction,
    json: bool,
    workspace: Option<i64>,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let wid = resolve_workspace_id(client, workspace).await?;
    match action {
        ProjectsAction::List => list(json, wid, client).await,
        ProjectsAction::Get { id } => get(json, wid, id, client).await,
        ProjectsAction::Create { name } => create(json, wid, &name, client).await,
        ProjectsAction::Update { id, name } => update(json, wid, id, name, client).await,
        ProjectsAction::Delete { id } => delete(wid, id, client).await,
    }
}

async fn list(json: bool, wid: i64, client: &(impl ApiClient + ?Sized)) -> Result<()> {
    if let Some(cached) = cache_get::<Vec<crate::models::Project>>("projects") {
        return output::print_list(&cached, json);
    }
    let projects = client.list_projects(wid).await?;
    cache_set("projects", &projects);
    output::print_list(&projects, json)
}

async fn get(json: bool, wid: i64, id: i64, client: &(impl ApiClient + ?Sized)) -> Result<()> {
    let project = client.get_project(wid, id).await?;
    output::print_result(&project, json)
}

async fn create(
    json: bool,
    wid: i64,
    name: &str,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let params = CreateProjectParams {
        name: name.to_string(),
    };
    let project = client.create_project(wid, &params).await?;
    invalidate_cache("projects");
    if json {
        output::print_result(&project, true)?;
    } else {
        println!("{} Project created", "✓".green().bold());
        println!("{project}");
    }
    Ok(())
}

async fn update(
    json: bool,
    wid: i64,
    id: i64,
    name: Option<String>,
    client: &(impl ApiClient + ?Sized),
) -> Result<()> {
    let params = UpdateProjectParams { name };
    let project = client.update_project(wid, id, &params).await?;
    invalidate_cache("projects");
    if json {
        output::print_result(&project, true)?;
    } else {
        println!("{} Project updated", "✓".green().bold());
        println!("{project}");
    }
    Ok(())
}

async fn delete(wid: i64, id: i64, client: &(impl ApiClient + ?Sized)) -> Result<()> {
    client.delete_project(wid, id).await?;
    invalidate_cache("projects");
    println!("{} Project #{id} deleted", "✓".green().bold());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MockApiClient;
    use crate::models::Project;

    fn make_project(id: i64, name: &str) -> Project {
        Project {
            id,
            workspace_id: 1,
            name: name.to_string(),
            active: true,
            color: "#06aaf5".to_string(),
            billable: None,
        }
    }

    #[tokio::test]
    async fn list_projects_displays_all() {
        let mut mock = MockApiClient::new();
        mock.expect_list_projects().returning(|_| {
            Ok(vec![
                make_project(1, "Project A"),
                make_project(2, "Project B"),
            ])
        });
        let result = run(ProjectsAction::List, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn delete_project_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_delete_project()
            .withf(|wid, pid| *wid == 1 && *pid == 5)
            .returning(|_, _| Ok(()));
        let result = run(ProjectsAction::Delete { id: 5 }, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_project_by_id() {
        let mut mock = MockApiClient::new();
        mock.expect_get_project()
            .withf(|wid, pid| *wid == 1 && *pid == 10)
            .returning(|_, _| Ok(make_project(10, "My Project")));
        let result = run(ProjectsAction::Get { id: 10 }, false, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_project_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_get_project()
            .returning(|_, _| Ok(make_project(10, "My Project")));
        let result = run(ProjectsAction::Get { id: 10 }, true, Some(1), &mock).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_project_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_create_project()
            .returning(|_, _| Ok(make_project(11, "New")));
        let result = run(
            ProjectsAction::Create {
                name: "New".to_string(),
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn create_project_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_create_project()
            .returning(|_, _| Ok(make_project(11, "New")));
        let result = run(
            ProjectsAction::Create {
                name: "New".to_string(),
            },
            true,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_project_calls_api() {
        let mut mock = MockApiClient::new();
        mock.expect_update_project()
            .withf(|wid, pid, _| *wid == 1 && *pid == 10)
            .returning(|_, _, _| Ok(make_project(10, "Renamed")));
        let result = run(
            ProjectsAction::Update {
                id: 10,
                name: Some("Renamed".to_string()),
            },
            false,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_project_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_update_project()
            .returning(|_, _, _| Ok(make_project(10, "Renamed")));
        let result = run(
            ProjectsAction::Update {
                id: 10,
                name: Some("Renamed".to_string()),
            },
            true,
            Some(1),
            &mock,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_projects_json_output() {
        let mut mock = MockApiClient::new();
        mock.expect_list_projects()
            .returning(|_| Ok(vec![make_project(1, "A"), make_project(2, "B")]));
        let result = run(ProjectsAction::List, true, Some(1), &mock).await;
        assert!(result.is_ok());
    }
}
