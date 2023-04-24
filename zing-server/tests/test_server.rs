use anyhow::Result;
use reqwest::StatusCode;
use serde_json::json;


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
