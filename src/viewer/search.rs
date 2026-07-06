use crate::error::AppResult;
use crate::storage::context::CommitContext;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use colored::Colorize;

struct SearchFilters {
    keywords: Vec<String>,
    since: Option<DateTime<Utc>>,
    intent_hint: Option<String>,
    path_hint: Option<String>,
}

#[derive(Serialize)]
struct SearchResult {
    commit: String,
    author: String,
    subject: String,
    timestamp: String,
    score: usize,
}

pub fn show_search(query: &str, limit: usize, as_json: bool, _api_key: &str) -> AppResult<()> {
    let contexts = crate::storage::load::load_all_contexts()?;
    let filters = parse_query(query);
    let results = apply_filters(&contexts, &filters, limit);
    
    if as_json {
        println!("{}", serde_json::to_string_pretty(&results)?);
        return Ok(());
    }
    
    print_search_results(query, &results, contexts.len());
    Ok(())
}

fn parse_query(query: &str) -> SearchFilters {
    let lower = query.to_lowercase();
    let mut filters = SearchFilters {
        keywords: extract_keywords(&lower),
        since: None,
        intent_hint: None,
        path_hint: None,
    };
    
    // Time parsing
    if lower.contains("last week") || lower.contains("this week") {
        filters.since = Some(Utc::now() - Duration::days(7));
    } else if lower.contains("last month") || lower.contains("this month") {
        filters.since = Some(Utc::now() - Duration::days(30));
    } else if lower.contains("yesterday") {
        filters.since = Some(Utc::now() - Duration::days(1));
    } else if lower.contains("last 3 months") || lower.contains("quarter") {
        filters.since = Some(Utc::now() - Duration::days(90));
    }
    
    // Intent hint
    if lower.contains("security") || lower.contains("vuln") || lower.contains("cve") {
        filters.intent_hint = Some("security".to_string());
    } else if lower.contains("fix") || lower.contains("bug") || lower.contains("patch") {
        filters.intent_hint = Some("fix".to_string());
    } else if lower.contains("feature") || lower.contains("feat") || lower.contains("add") {
        filters.intent_hint = Some("feature".to_string());
    } else if lower.contains("refactor") || lower.contains("cleanup") || lower.contains("rewrite") {
        filters.intent_hint = Some("refactor".to_string());
    } else if lower.contains("perf") || lower.contains("performance") || lower.contains("speed") {
        filters.intent_hint = Some("performance".to_string());
    }
    
    // Path hint
    for word in lower.split_whitespace() {
        if word.contains('/') || word.ends_with(".rs") || word.ends_with(".py") || word.ends_with(".js") {
            filters.path_hint = Some(word.to_string());
            break;
        }
        if ["auth", "login", "api", "db", "database", "handler", "model", "service"].contains(&word) {
            filters.path_hint = Some(word.to_string());
            break;
        }
    }
    
    filters
}

fn apply_filters(contexts: &[CommitContext], filters: &SearchFilters, limit: usize) -> Vec<SearchResult> {
    let mut scored = contexts.iter()
        .filter(|ctx| matches_filters(ctx, filters))
        .map(|ctx| score_result(ctx, filters))
        .filter(|r| r.score > 0)
        .collect::<Vec<_>>();
        
    scored.sort_by(|a, b| b.score.cmp(&a.score).then(b.timestamp.cmp(&a.timestamp)));
    scored.into_iter().take(limit).collect()
}

fn extract_keywords(query: &str) -> Vec<String> {
    query.split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

fn matches_filters(ctx: &CommitContext, filters: &SearchFilters) -> bool {
    if let Some(since) = filters.since {
        if let Ok(ts) = DateTime::parse_from_rfc3339(&ctx.timestamp) {
            if ts.with_timezone(&Utc) < since {
                return false;
            }
        }
    }
    true
}

fn score_result(ctx: &CommitContext, filters: &SearchFilters) -> SearchResult {
    let mut score = 0;
    
    // Check keywords
    let text = format!("{} {} {} {}", ctx.behavior.author, ctx.analysis.intent.summary(), ctx.files.join(" "), ctx.commit).to_lowercase();
    for kw in &filters.keywords {
        if text.contains(kw) {
            score += 1;
        }
    }
    
    // Check path hint
    if let Some(path_hint) = &filters.path_hint {
        if ctx.files.iter().any(|f| f.to_lowercase().contains(path_hint)) {
            score += 5;
        }
    }
    
    // Check intent hint
    if let Some(intent_hint) = &filters.intent_hint {
        if format!("{:?}", ctx.analysis.intent.category).to_lowercase().contains(intent_hint) {
            score += 3;
        }
    }
    
    SearchResult {
        commit: ctx.commit.clone(),
        author: ctx.behavior.author.clone(),
        subject: ctx.analysis.intent.summary().clone(),
        timestamp: ctx.timestamp.clone(),
        score,
    }
}

fn print_search_results(query: &str, results: &[SearchResult], total_contexts: usize) {
    println!("\n🔍 Search Results for \"{}\"", query.cyan());
    println!("Found {} matching commits (searched {} captured contexts)\n", results.len().to_string().bold(), total_contexts);
    
    for (i, res) in results.iter().enumerate() {
        println!("{}. {} by {}", 
            (i + 1).to_string().dimmed(),
            res.commit[..8.min(res.commit.len())].yellow(),
            res.author.cyan()
        );
        println!("   {} {}", "Subject:".dimmed(), res.subject);
        println!("   {} {}", "Date:".dimmed(), res.timestamp.dimmed());
        println!();
    }
}
