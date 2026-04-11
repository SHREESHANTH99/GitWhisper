use crate::error::{AppError, AppResult};
use reqwest::blocking::Client;
use serde_json::json;
use std::time::Duration;

pub fn is_available(ollama_url: &str, timeout_secs: u64) -> bool {
    let base = ollama_url.trim_end_matches('/');
    if base.is_empty() {
        return false;
    }

    let client = match Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };

    let url = format!("{base}/api/tags");
    match client.get(url).send() {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

pub fn generate(
    prompt: &str,
    model: &str,
    ollama_url: &str,
    timeout_secs: u64,
) -> AppResult<String> {
    let base = ollama_url.trim_end_matches('/');
    if base.is_empty() {
        return Err(AppError::message(
            "Missing Ollama URL for local AI provider.",
        ));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .build()?;

    let url = format!("{base}/api/generate");
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        }))
        .send()?;

    let status = response.status();
    let json_response = response
        .json::<serde_json::Value>()
        .unwrap_or_else(|_| json!({}));

    if !status.is_success() {
        let message = json_response
            .get("error")
            .and_then(|value| value.as_str())
            .unwrap_or("Ollama request failed");
        return Err(AppError::message(format!(
            "Local AI request failed: {message}"
        )));
    }

    if let Some(error) = json_response.get("error").and_then(|value| value.as_str()) {
        return Err(AppError::message(format!(
            "Local AI request failed: {error}"
        )));
    }

    json_response
        .get("response")
        .and_then(|value| value.as_str())
        .map(|text| text.to_string())
        .ok_or_else(|| AppError::message("Local AI response did not include `response` text."))
}
