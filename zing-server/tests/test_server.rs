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

    let tables_status = client
        .get("http://localhost:3000/table")
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(
        tables_status
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

    let tables_status = client
        .get("http://localhost:3000/table")
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(
        tables_status
            .as_array()
            .context("expected array of tables")?
            .len(),
        1
    );

    let table_status = client
        .get(format!("http://localhost:3000/table/{}", table_id))
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(table_status["id"], table_id);

    let join_response = client
        .post(format!("http://localhost:3000/table/{}", table_id))
        .send()
        .await?;
    // must not be able to join table again
    assert_eq!(join_response.status(), StatusCode::CONFLICT);

    let tables_status = client
        .get("http://localhost:3000/table")
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(
        tables_status
            .as_array()
            .context("expected array of tables")?
            .len(),
        1 // number of tables must not have changed
    );

    Ok(())
}

#[tokio::test]
async fn test_game_starting() -> Result<()> {
    let client1 = reqwest::Client::builder().cookie_store(true).build()?;

    client1
        .post("http://localhost:3000/login")
        .json(&json!({ "name": "Player 1" }))
        .send()
        .await?;

    let create_response = client1
        .post("http://localhost:3000/table")
        .send()
        .await?
        .json::<Value>()
        .await?;
    let table_id = create_response["id"]
        .as_str()
        .context("table status should have id")?;

    let start_response = client1
        .post(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?;
    // must not be able to start game with single player at table
    assert_eq!(start_response.status(), StatusCode::CONFLICT);

    let client2 = reqwest::Client::builder().cookie_store(true).build()?;

    client2
        .post("http://localhost:3000/login")
        .json(&json!({ "name": "Player 2" }))
        .send()
        .await?;

    let join_response = client2
        .post(format!("http://localhost:3000/table/{}", table_id))
        .send()
        .await?;
    assert_eq!(join_response.status(), StatusCode::OK);

    let inactive_response = client2
        .get(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(
        inactive_response["active"]
            .as_bool()
            .context("game status should have active attribute")?,
        false
    );

    let start_response = client2
        .post(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?;
    assert_eq!(start_response.status(), StatusCode::OK);

    let active_response = client2
        .get(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(
        active_response["active"]
            .as_bool()
            .context("game status should have active attribute")?,
        true
    );

    let active_response = client1
        .get(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(
        active_response["active"]
            .as_bool()
            .context("game status should have active attribute")?,
        true
    );

    let start_response = client2
        .post(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?;
    assert_eq!(start_response.status(), StatusCode::CONFLICT);

    Ok(())
}

#[tokio::test]
async fn test_playing_cards() -> Result<()> {
    let client2 = reqwest::Client::builder().cookie_store(true).build()?;

    client2
        .post("http://localhost:3000/login")
        .json(&json!({ "name": "Player 2" }))
        .send()
        .await?;

    let client1 = reqwest::Client::builder().cookie_store(true).build()?;

    client1
        .post("http://localhost:3000/login")
        .json(&json!({ "name": "Player 1" }))
        .send()
        .await?;

    let create_response = client1
        .post("http://localhost:3000/table")
        .send()
        .await?
        .json::<Value>()
        .await?;
    let table_id = create_response["id"]
        .as_str()
        .context("table status should have id")?;

    client2
        .post(format!("http://localhost:3000/table/{}", table_id))
        .send()
        .await?;

    client1
        .post(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?;

    let play_response = client2
        .post(format!(
            "http://localhost:3000/table/{}/game/play",
            table_id
        ))
        .json(&json!({ "card_index": 2 }))
        .send()
        .await?;
    assert_eq!(play_response.status(), StatusCode::OK);

    let play_response = client2
        .post(format!(
            "http://localhost:3000/table/{}/game/play",
            table_id
        ))
        .json(&json!({ "card_index": 1 }))
        .send()
        .await?;
    // playing when not one's turn:
    assert_eq!(play_response.status(), StatusCode::CONFLICT);

    let play_response = client1
        .post(format!(
            "http://localhost:3000/table/{}/game/play",
            table_id
        ))
        .json(&json!({ "card_index": 3 }))
        .send()
        .await?;
    assert_eq!(play_response.status(), StatusCode::OK);

    let play_response = client1
        .post(format!(
            "http://localhost:3000/table/{}/game/play",
            table_id
        ))
        .json(&json!({ "card_index": 0 }))
        .send()
        .await?;
    // playing when not one's turn:
    assert_eq!(play_response.status(), StatusCode::CONFLICT);

    let play_response = client2
        .post(format!(
            "http://localhost:3000/table/{}/game/play",
            table_id
        ))
        .json(&json!({ "card_index": 3 }))
        .send()
        .await?;
    // too high card index
    assert_eq!(play_response.status(), StatusCode::CONFLICT);

    let ended_response = client2
        .get(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(
        ended_response["ended"]
            .as_bool()
            .context("game status should have ended attribute")?,
        false
    );

    // echo "players have 48 cards in total; 2 have been played already, 2*23 to go..."
    for _ in 0..23 {
        let play_response = client2
            .post(format!(
                "http://localhost:3000/table/{}/game/play",
                table_id
            ))
            .json(&json!({ "card_index": 0 }))
            .send()
            .await?;
        assert_eq!(play_response.status(), StatusCode::OK);

        let play_response = client1
            .post(format!(
                "http://localhost:3000/table/{}/game/play",
                table_id
            ))
            .json(&json!({ "card_index": 0 }))
            .send()
            .await?;
        assert_eq!(play_response.status(), StatusCode::OK);
    }

    let play_response = client2
        .post(format!(
            "http://localhost:3000/table/{}/game/play",
            table_id
        ))
        .json(&json!({ "card_index": 0 }))
        .send()
        .await?;
    assert_eq!(play_response.status(), StatusCode::CONFLICT);

    let play_response = client1
        .post(format!(
            "http://localhost:3000/table/{}/game/play",
            table_id
        ))
        .json(&json!({ "card_index": 0 }))
        .send()
        .await?;
    assert_eq!(play_response.status(), StatusCode::CONFLICT);

    let ended_response = client2
        .get(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(
        ended_response["ended"]
            .as_bool()
            .context("game status should have ended attribute")?,
        true
    );

    let active_response = client2
        .get(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(
        active_response["active"]
            .as_bool()
            .context("game status should have active attribute")?,
        true
    );

    let finish_response = client1
        .delete(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?;
    assert_eq!(finish_response.status(), StatusCode::OK);

    let inactive_response = client2
        .get(format!("http://localhost:3000/table/{}/game", table_id))
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert_eq!(
        inactive_response["active"]
            .as_bool()
            .context("game status should have active attribute")?,
        false
    );

    Ok(())
}
