//! Minimal ntfy.sh push client.
//!
//! Sends an HTTP POST to https://ntfy.sh/{topic} with the reminder title
//! and body. The topic is treated as a secret (long random string) and
//! is NEVER logged or included in any telemetry.

use anyhow::Result;

pub async fn push(topic: &str, title: &str, body: &str) -> Result<()> {
    let url = format!("https://ntfy.sh/{}", topic);
    reqwest::Client::new()
        .post(&url)
        .header("Title", title)
        .body(body.to_owned())
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
