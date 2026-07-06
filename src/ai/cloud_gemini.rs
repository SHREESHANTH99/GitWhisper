use crate::error::{AppError, AppResult};
use reqwest::blocking::Client;
use serde_json::json;
use std::sync::OnceLock;
use std::time::Duration;

// ── Shared HTTP client ────────────────────────────────────────────────────────
//
// `Client` is cheap to clone (it holds an `Arc` internally) but expensive to
// *build* — each call to `Client::builder().build()` performs TLS
// initialisation and spawns a connection-pool thread.
//
// We keep a single instance alive for the process lifetime via `OnceLock`.
// The timeout is fixed at first initialisation.  If callers ever need
// different timeouts they can build their own `Client` and bypass this helper.
//
// Note: The `reqwest::blocking::Client` is used here, which blocks the OS thread.
// Since the current server architecture is single-threaded or spawns OS threads
// per connection, this is acceptable for now. If moving to async Rust, switch
// to the async `reqwest::Client`.
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn get_client(timeout_secs: u64) -> Client {
    HTTP_CLIENT
        .get_or_init(|| {
            Client::builder()
                .timeout(Duration::from_secs(timeout_secs.max(1)))
                .build()
                .expect("Failed to build shared HTTP client for Gemini")
        })
        .clone()
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn generate(prompt: &str, model: &str, api_key: &str, timeout_secs: u64) -> AppResult<String> {
    if api_key.trim().is_empty() {
        return Err(AppError::MissingApiKey);
    }

    let client = get_client(timeout_secs);

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

    // Read the raw body text first so that non-JSON responses (HTML error
    // pages, Cloudflare 502s, rate-limit splash pages, …) are preserved and
    // can be surfaced to the user rather than silently becoming `{}`.
    let body_text = response.text()?;

    let json_response: serde_json::Value = serde_json::from_str(&body_text).unwrap_or_else(|_| {
        // Not valid JSON — embed up to 500 chars of the raw body so the
        // error path below can include it in its message.
        let preview = &body_text[..body_text.len().min(500)];
        json!({ "__raw_error__": preview })
    });

    if !status.is_success() {
        // Gemini surfaces errors as {"error": {"message": "…", "code": N}}.
        // If the body was not JSON we fall back to the raw preview we stored.
        let message = json_response
            .get("error")
            .and_then(|v| v.get("message"))
            .and_then(|v| v.as_str())
            .or_else(|| json_response.get("__raw_error__").and_then(|v| v.as_str()))
            .unwrap_or("Gemini request failed with no error message");

        return Err(AppError::message(format!(
            "Gemini API error (HTTP {status}): {message}"
        )));
    }

    extract_text(&json_response).ok_or_else(|| {
        AppError::message("AI response did not include explanation text (missing candidate text).")
    })
}

// ── Response parsing ──────────────────────────────────────────────────────────

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
