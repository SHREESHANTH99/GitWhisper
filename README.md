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
- Gemini-powered explanations

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

## Commit Context Capture

`Gitwhisper` captures developer activity for each commit, including:

- commands executed
- environment metadata
- timestamps
- files changed in the commit

Example stored metadata:

```text
.git/gitwhisper/f6e3058.json
```

Example structure:

```json
{
  "commit": "f6e3058",
  "timestamp": "2026-03-07T12:14:26Z",
  "commands": ["npm test", "git add auth.js"],
  "environment": "OS: windows\nBranch: main\nNode: v22.14.0",
  "files": ["auth.js"]
}
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
Build File History
     |
     v
Generate AI Prompt
     |
     v
Call Gemini
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

# Gemini API Setup

`Gitwhisper` uses the Google Gemini API for AI explanations.

Create an API key here:

[https://aistudio.google.com/app/apikey](https://aistudio.google.com/app/apikey)

Set the environment variable.

### Windows

```powershell
setx GEMINI_API_KEY "YOUR_API_KEY"
```

### Linux / macOS

```bash
export GEMINI_API_KEY="YOUR_API_KEY"
```

---

# Project Structure

```text
gitwhisper
|
|-- src
|   |-- collectors
|   |   |-- commands.rs
|   |   |-- env.rs
|   |   `-- tests.rs
|   |
|   |-- storage
|   |   |-- context.rs
|   |   |-- load.rs
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

- AI change intent detection (bug / feature / refactor)
- GitHub PR summarization
- commit impact analysis
- code evolution visualization
- VS Code extension
- web dashboard for repository insights

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
