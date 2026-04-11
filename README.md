# Gitwhisper
### AI-Powered Git Commit Intelligence for Developers

`Gitwhisper` is a Rust CLI that explains the evolution of your codebase using AI plus commit-time developer context.

It captures useful commit metadata after each commit and combines that with Git history to answer the harder question: why did this file change?

---

# Motivation

Git already shows what changed. The missing piece is usually intent:

- Why the change happened
- What problem it solved
- What the developer was trying to accomplish
- How a file evolved across multiple commits

`Gitwhisper` bridges that gap by combining:

- Git commit history
- developer commands
- environment context
- semantic diff + intent + impact analysis
- AI-powered explanations (Gemini cloud + optional local Ollama)

---

# Features

## AI File Explanation

Explain why a file changed using recent Git history plus captured commit context.

```bash
gitwhisper explain auth.js
```

Example output:

```text
AI Explanation:
The authentication module was updated to introduce JWT token validation.
Earlier commits added login functionality but lacked proper token
expiration checks. The latest change closes that gap and improves
authentication security.
```

## File Evolution Summary

Summarize how a file evolved over time (milestones + narrative).

```bash
gitwhisper summarize auth.js
```

## Semantic Diff + Intent + Impact

Gitwhisper extracts structured signals from commits (without needing developer input):

- semantic diff facts (file ops, imports, symbols, complexity hints)
- intent detection (conventional commits, urgency, risk, breaking changes + "why" signals)
- change impact signals (dependency-aware impact scoring where supported)

## Code Owners (Team Insight)

Show who contributes to a file or directory the most (useful for reviews and spotting knowledge silos).

```bash
gitwhisper owners src/auth.rs
gitwhisper owners src/api --limit 10
```

## Commit Context Capture

`Gitwhisper` captures developer activity for each commit, including:

- commands executed
- structured environment metadata
- IDE/editor context (best-effort, no file contents)
- review/test context (best-effort)
- behavioral snapshot (commit patterns)
- timestamps
- files changed in the commit
- semantic analysis (diff/intent/impact)

Example stored metadata:

```text
.git/gitwhisper/f6e3058.json
```

Example structure:

```json
{
  "schema_version": 2,
  "commit": "f6e3058",
  "timestamp": "2026-03-07T12:14:26Z",
  "commands": ["npm test", "git add auth.js"],
  "environment": {
    "os": "windows",
    "branch": "main",
    "shell": "powershell.exe",
    "working_directory": "C:/repo",
    "tools": { "node": "v22.14.0", "python": "3.12.2", "rust": "1.77.2" }
  },
  "ide": { "name": "VSCode", "active_files": ["src/auth.rs"], "extensions": [] },
  "review": { "ci_provider": "github-actions", "test_status": "passed" },
  "behavior": { "author": "alice", "commits_last_7d": 12, "burnout_risk": "low" },
  "files": ["auth.js"],
  "analysis": {
    "intent": {
      "category": "bug-fix",
      "urgency": "normal",
      "risk": "medium",
      "scope": "cross-file",
      "conventional_type": "fix",
      "conventional_scope": "auth",
      "breaking_change": false,
      "signals": ["type `fix`", "bug-fix wording"],
      "confidence": 94
    },
    "diff": { "files_changed": 1, "lines_added": 5, "lines_removed": 2 },
    "impact": { "score": 0.42, "hotspots": ["auth"] }
  }
}
```

## Configuration

Project settings live in `.gitwhisper.toml` at the repo root.

Example (cloud, local, or hybrid AI):

```toml
[ai]
provider = "hybrid"         # cloud | local | hybrid
model = "gemini-1.5-flash"  # cloud model (Gemini)
local_model = "mistral"     # Ollama model name
prompt_char_budget = 12000  # prompt budget for context-window optimization
history_depth = 10
request_timeout_secs = 45
hybrid_max_prompt_chars = 8000
ollama_url = "http://localhost:11434"

[privacy]
offline_mode = false        # disables cloud calls; local still works
local_cache_only = true
```

## Explanation Caching

To avoid repeated AI calls, `Gitwhisper` stores explanations locally.

Cache location:

```text
.git/gitwhisper/cache/
```

Benefits:

- faster repeat lookups
- reduced API usage
- improved CLI responsiveness
- multi-level caching (in-memory + on-disk index)

## Timeline Viewer

Show the timeline of changes for a file.

```bash
gitwhisper timeline auth.js
```

Example:

```text
auth.js timeline:

f6e3058  Fix JWT validation
a3b2819  Add authentication middleware
918c92f  Initial login system
```

## Replay Captured Commit Activity

Replay the metadata captured for the latest commit, or pass a commit hash prefix.

```bash
gitwhisper replay
gitwhisper replay f6e3058
```

## Log Captured Context

View saved commit context entries:

```bash
gitwhisper log
```

## Post-Commit Hook Setup

Install the Git hook that captures commit context automatically:

```bash
gitwhisper init
```

---

# How It Works

```text
User Command
     |
     v
Load Commit Metadata
     |
     v
Build File History + Context Window
     |
     v
Semantic Diff + Intent + Impact
     |
     v
Generate Structured Prompt
     |
     v
Select Model (Cloud/Local/Hybrid)
     |
     v
Call AI Provider (Gemini or Ollama)
     |
     v
Cache Result
     |
     v
Return Explanation
```

---

# Installation

Clone the repository:

```bash
git clone https://github.com/YOUR_USERNAME/gitwhisper.git
cd gitwhisper
```

Build the project:

```bash
cargo build --release
```

Run the CLI:

```bash
cargo run -- explain auth.js
```

Optional: install the post-commit hook so context is captured automatically.

```bash
cargo run -- init
```

---

# AI Setup

## Cloud (Gemini)

Create a Gemini API key:

[https://aistudio.google.com/app/apikey](https://aistudio.google.com/app/apikey)

Set the environment variable:

### Windows

```powershell
setx GEMINI_API_KEY "YOUR_API_KEY"
```

### Linux / macOS

```bash
export GEMINI_API_KEY="YOUR_API_KEY"
```

## Local (Ollama)

Install Ollama and pull a model:

```bash
ollama pull mistral
```

Then set in `.gitwhisper.toml`:

```toml
[ai]
provider = "local"
local_model = "mistral"
ollama_url = "http://localhost:11434"
```

---

# Project Structure

```text
gitwhisper
|
|-- src
|   |-- ai
|   |   |-- cloud_gemini.rs
|   |   |-- local_ollama.rs
|   |   |-- model_selector.rs
|   |   `-- mod.rs
|   |
|   |-- analysis
|   |   |-- behavior_patterns.rs
|   |   |-- diff_parser.rs
|   |   |-- impact_analysis.rs
|   |   |-- intent_detection.rs
|   |   `-- mod.rs
|   |
|   |-- collectors
|   |   |-- commands.rs
|   |   |-- env.rs
|   |   |-- ide.rs
|   |   |-- review_context.rs
|   |   `-- tests.rs
|   |
|   |-- storage
|   |   |-- cache_manager.rs
|   |   |-- context.rs
|   |   |-- load.rs
|   |   |-- predictive_cache.rs
|   |   `-- save.rs
|   |
|   |-- viewer
|   |   |-- explain.rs
|   |   |-- log.rs
|   |   |-- replay.rs
|   |   `-- timeline.rs
|   |
|   |-- capture.rs
|   |-- cli.rs
|   |-- config.rs
|   |-- error.rs
|   |-- git.rs
|   |-- hooks.rs
|   |-- main.rs
|   `-- storage.rs
|
|-- Cargo.toml
|-- Cargo.lock
`-- README.md
```

---

# Example Commands

Explain a file:

```bash
gitwhisper explain auth.js
```

Show a file timeline:

```bash
gitwhisper timeline auth.js
```

Replay the latest captured activity:

```bash
gitwhisper replay
```

Replay a specific commit:

```bash
gitwhisper replay f6e3058
```

View saved commit logs:

```bash
gitwhisper log
```

Capture context manually:

```bash
gitwhisper capture
```

Install the Git hook:

```bash
gitwhisper init
```

---

# Roadmap

- Phase 1 (Done): semantic diff + intent detection + impact signals + caching
- Phase 2 (In Progress): multi-model AI (Gemini + Ollama), context window optimization, structured reasoning prompts
- Next: PR metadata ingestion + summaries, richer dependency graphs, dashboards

---

# Contributing

Contributions are welcome.

1. Fork the repository
2. Create a feature branch
3. Commit changes
4. Open a pull request

---

# License

MIT License
