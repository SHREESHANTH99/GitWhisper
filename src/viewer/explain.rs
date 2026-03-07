use std::fs;
use serde::Deserialize;
use reqwest::blocking::Client;
use serde_json::json;

#[derive(Deserialize)]
struct CommitContext {
    commit: String,
    timestamp: String,
    commands: Vec<String>,
    environment: String,
}

/// Explain changes to a file using Gemini API
/// `file` = file name to explain
//  `api_key` = Gemini API key
pub fn explain_file(file: &str, api_key: &str) {
    let dir = ".git/commitlens";
    let mut history = Vec::new();

    // Load all commit JSONs that mention this file
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(content) = fs::read_to_string(&path) {
                if content.contains(file) {
                    let ctx: CommitContext = serde_json::from_str(&content).unwrap();
                    history.push(ctx);
                }
            }
        }
    }

    if history.is_empty() {
        println!("No commit context found for {}", file);
        return;
    }

    // Build the prompt for Gemini
    let mut prompt = format!("Summarize the changes for file {}:\n", file);
    for ctx in &history {
        prompt.push_str(&format!(
            "- Commit {} at {}\n  Commands: {:?}\n  Environment: {}\n",
            ctx.commit, ctx.timestamp, ctx.commands, ctx.environment
        ));
    }
    prompt.push_str("\nProvide a clear explanation of why these changes happened.\n");

    // Call Gemini API
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
        api_key
    );

    let client = Client::new();
    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }]
        }))
        .send();

    match resp {
        Ok(r) => {
            let json_resp: serde_json::Value = r.json::<serde_json::Value>().unwrap_or_else(|_| json!({}));
            
            if let Some(text) = json_resp
                .get("candidates")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("content"))
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.get(0))
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str()) 
            {
                println!("AI Explanation:\n{}", text);
            } else {
                if let Some(error) = json_resp.get("error") {
                    let message = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown API Error");
                    let status = error.get("status").and_then(|s| s.as_str()).unwrap_or("UNKNOWN");
                    
                    println!("Error from Gemini API:");
                    println!("Status: {}", status);
                    println!("Message: {}", message);
                    
                    if status == "RESOURCE_EXHAUSTED" {
                        println!("\nTip: It looks like you have hit the rate limit for your Gemini API key or the API is restricted in your region (Quota limit 0). You may need to use a paid account or wait until your quota resets.");
                    }
                } else {
                    println!("No explanation returned or error parsing response.");
                }
            }
        }
        Err(e) => println!("Error calling Gemini API: {}", e),
    }
}