use crate::error::{AppError, AppResult};
use reqwest::blocking::Client;
use serde_json::json;
use std::time::Duration;

pub fn generate(prompt: &str, model: &str, api_key: &str, timeout_secs: u64) -> AppResult<String> {
    if api_key.trim().is_empty() {
        return Err(AppError::message(
            "Missing GEMINI_API_KEY for cloud AI provider.",
        ));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .build()?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }]
        }))
        .send()?;

    let status = response.status();
    let json_response = response
        .json::<serde_json::Value>()
        .unwrap_or_else(|_| json!({}));

    if !status.is_success() {
        let message = json_response
            .get("error")
            .and_then(|value| value.get("message"))
            .and_then(|value| value.as_str())
            .unwrap_or("Gemini request failed");
        return Err(AppError::message(format!("AI request failed: {message}")));
    }

    extract_text(&json_response).ok_or_else(|| {
        AppError::message("AI response did not include explanation text (missing candidate text).")
    })
}

fn extract_text(response: &serde_json::Value) -> Option<String> {
    response
        .get("candidates")
        .and_then(|value| value.get(0))
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("parts"))
        .and_then(|value| value.get(0))
        .and_then(|value| value.get("text"))
        .and_then(|value| value.as_str())
        .map(|text| text.to_string())
}
