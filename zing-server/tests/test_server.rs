use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde_json::{json, Value};

#[tokio::test]
async fn test_login_logout() -> Result<()> {
    let client = reqwest::Client::builder().cookie_store(true).build()?;

    let login_response = client
        .post("http://localhost:3000/login")
        .json(&json!({ "name": "John Doe" }))
        .send()
        .await?;
    assert_eq!(login_response.status(), StatusCode::OK);

    let status_response = client.get("http://localhost:3000/login").send().await?;
    assert_eq!(status_response.status(), StatusCode::OK);
    // TODO: check JSON output?

    let status_response = client.delete("http://localhost:3000/login").send().await?;
    assert_eq!(status_response.status(), StatusCode::OK);

    let status_response = client.get("http://localhost:3000/login").send().await?;
    assert_eq!(status_response.status(), StatusCode::UNAUTHORIZED);

    let status_response = client.delete("http://localhost:3000/login").send().await?;
    assert_eq!(status_response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

#[tokio::test]
async fn test_create_table() -> Result<()> {
    let client = reqwest::Client::builder().cookie_store(true).build()?;

    let login_response = client
        .post("http://localhost:3000/login")
        .json(&json!({ "name": "Jane Doe" }))
        .send()
        .await?;
    assert_eq!(login_response.status(), StatusCode::OK);

    let table_status = client
        .get("http://localhost:3000/table")
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(
        table_status
            .as_array()
            .context("expected array of tables")?
            .len(),
        0
    );

    let create_response = client
        .post("http://localhost:3000/table")
        .send()
        .await?
        .json::<Value>()
        .await?;
    let table_id = create_response["id"]
        .as_str()
        .context("table status should have id")?;

    let table_status = client
        .get(format!("http://localhost:3000/table/{}", table_id))
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(table_status["id"], table_id);

    Ok(())
}
