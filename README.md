# GitWhisper

<p align="center">
  <img src="docs/assets/gitwhisper-hero.svg" alt="GitWhisper project banner" width="100%">
</p>

<p align="center">
  <strong>AI-powered Git intelligence for developers and teams.</strong><br>
  GitWhisper captures commit context, analyzes change intent, explains code evolution, and turns raw Git history into practical engineering insight.
</p>

<p align="center">
  <a href="https://github.com/SHREESHANTH99/GitWhisper/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/SHREESHANTH99/GitWhisper/ci.yml?branch=main&label=ci"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-green"></a>
  <a href="CHANGELOG.md"><img alt="Release" src="https://img.shields.io/badge/release-v0.1.0-blue"></a>
  <img alt="Rust" src="https://img.shields.io/badge/rust-2021-orange">
  <img alt="Storage" src="https://img.shields.io/badge/storage-JSON%20%7C%20PostgreSQL-336791">
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> |
  <a href="#what-is-in-v010">v0.1.0</a> |
  <a href="#pipelines-and-flow">Pipelines</a> |
  <a href="#commands">Commands</a> |
  <a href="#configuration">Configuration</a> |
  <a href="#docker-and-postgresql">Docker</a> |
  <a href="#release-status">Release Status</a>
</p>

---

## What GitWhisper Does

Git already tells you what changed. GitWhisper helps explain why it changed, how risky it is, who knows the area best, and what reviewers should inspect next.

GitWhisper is a Rust CLI that runs inside a Git repository. It combines Git history, commit-time context, semantic diff analysis, AI explanation, local/team storage, and collaboration outputs.

## What Is In v0.1.0

| Area | Status | Included |
| --- | --- | --- |
| Context capture | Working | Commit metadata, commands, environment, IDE/review context hooks |
| Semantic diff analysis | Working | File stats, symbol/import signals, rough complexity movement |
| Intent detection | Working | Category, urgency, risk, scope, conventional commit signals |
| Impact analysis | Working | Dependency hints, direct/transitive dependents, circular dependency detection |
| AI explanation | Working | Gemini cloud, Ollama local, hybrid selection, fallback summaries |
| Engineering health | Working | Quality, security, performance, bug prediction, knowledge risk, refactor priority |
| Collaboration | Working | Git notes, Slack/Discord sharing, GitHub/GitLab review helpers, digests |
| Knowledge generation | Working | File summaries, owners, wiki generation, ADR generation |
| Persistence | Working | JSON backend and PostgreSQL backend |
| Enterprise foundation | Started | Docker, auth module, audit module, feedback system, DB abstraction |

<p align="center">
  <img src="docs/assets/pipeline-overview.svg" alt="GitWhisper pipeline overview" width="100%">
</p>

---

## Quick Start

### Build

```bash
git clone https://github.com/SHREESHANTH99/GitWhisper.git
cd GitWhisper
cargo build --release
```

Install locally:

```bash
cargo install --path .
```

Or run without installing:

```powershell
.\target\release\gitwhisper.exe --help
```

### Bootstrap A Repository

```bash
gitwhisper init
gitwhisper capture
gitwhisper annotate
```

`gitwhisper init` installs a managed `post-commit` hook. New commits can then be captured and annotated automatically.

### First Useful Commands

```bash
gitwhisper explain src/main.rs
gitwhisper summarize src/main.rs
gitwhisper owners src --limit 10
gitwhisper refactor-priority src --limit 10
gitwhisper dashboard --host 127.0.0.1 --port 7878
```

---

## Pipelines And Flow
### System Overview

![System Overview](docs/flowcharts/system_Overview.png)

GitWhisper starts with repository facts, adds local context, converts the result into structured analysis, then chooses the best output path: terminal, Git notes, dashboard, generated docs, or team integrations.

### Commit Capture Pipeline

![Commit Capture Pipeline](docs/flowcharts/Commit%20Capture%20Pipeline.png)

The capture layer stores metadata and sanitized context. It does not capture full IDE file contents.

### Explain Pipeline

![Explain Pipeline](docs/flowcharts/Explain%20Pipeline.png)

Hybrid mode prefers fast local explanation for smaller prompts, uses cloud AI for deeper analysis when configured, and falls back to heuristic summaries when AI is unavailable.

### Engineering Health Pipeline

<p align="center">
  <img src="docs/assets/risk-dashboard.svg" alt="GitWhisper engineering health dashboard illustration" width="100%">
</p>

![Engineering Health Pipeline](docs/flowcharts/Engineering%20health%20pipeline.png)

The health analyzers are heuristic in v0.1.0. They are useful as review guidance and prioritization signals, not as a replacement for engineering judgment.

### Storage Pipeline

![Storage Pipeline](docs/flowcharts/storage%20pipeline.png)

JSON is the default zero-setup backend. PostgreSQL is available for Docker/team workflows and has been live-tested with feedback and audit commands.

---

## Commands

### Core History And Explanation

| Command | Purpose |
| --- | --- |
| `gitwhisper init` | Install the managed post-commit hook |
| `gitwhisper capture` | Capture context for the current `HEAD` commit |
| `gitwhisper annotate [commit]` | Generate an explanation and store it in Git notes |
| `gitwhisper log` | Show captured context entries |
| `gitwhisper replay [commit]` | Replay captured activity for a commit |
| `gitwhisper timeline <file>` | Show a file's commit timeline |
| `gitwhisper explain <file>` | Explain why a file changed |
| `gitwhisper summarize <file>` | Tell the file's evolution story |
| `gitwhisper owners <path> --limit 10` | Show likely code owners |

### Risk And Health

| Command | Purpose |
| --- | --- |
| `gitwhisper quality <path>` | Analyze complexity, duplication, churn, and maintainability risk |
| `gitwhisper security <path>` | Flag security-sensitive patterns and risky changes |
| `gitwhisper performance <path>` | Flag performance-sensitive patterns and hotspots |
| `gitwhisper bug-predict [path] --limit 10` | Rank likely bug-prone files |
| `gitwhisper knowledge-risk [path] --limit 10` | Report ownership concentration and knowledge silos |
| `gitwhisper refactor-priority [path] --limit 10` | Rank files most worth refactoring |

### Collaboration, Docs, Feedback, Audit

| Command | Purpose |
| --- | --- |
| `gitwhisper share slack [commit]` | Send a commit explanation to Slack |
| `gitwhisper share discord [commit]` | Send a commit explanation to Discord |
| `gitwhisper review github [commit]` | Publish a GitHub PR helper summary |
| `gitwhisper review gitlab [commit]` | Publish a GitLab MR helper summary |
| `gitwhisper digest slack --period daily` | Send a Slack digest |
| `gitwhisper dashboard --host 127.0.0.1 --port 7878` | Start the local dashboard |
| `gitwhisper export --format json --output exports/snapshot.json` | Export analytics JSON |
| `gitwhisper wiki --output wiki` | Generate markdown wiki pages |
| `gitwhisper adr --output docs/adrs` | Generate ADR files |
| `gitwhisper feedback <commit> --good` | Store positive explanation feedback |
| `gitwhisper feedback-export --format csv --output exports/feedback.csv` | Export feedback |
| `gitwhisper audit-log --limit 20` | Show audit events |
| `gitwhisper audit-prune --days 90` | Prune old audit events |

---

## Configuration

GitWhisper reads `.gitwhisper.toml` from the repository root and also loads `.env`.

```toml
[ai]
provider = "hybrid" # cloud | local | hybrid
model = "gemini-1.5-flash"
local_model = "mistral"
prompt_char_budget = 12000
history_depth = 10
request_timeout_secs = 45
hybrid_max_prompt_chars = 8000
ollama_url = "http://localhost:11434"

[privacy]
offline_mode = false
local_cache_only = true
exclude_files = []

[database]
backend = "json" # json | postgres
path = ".git/gitwhisper/gitwhisper.db"
postgres_url = ""
```

### Environment Variables

| Variable | Purpose |
| --- | --- |
| `GEMINI_API_KEY` | Cloud AI key for Gemini flows |
| `GITWHISPER_USER` | Override detected username for auth/audit |
| `GITWHISPER_DATABASE_BACKEND` | Override backend: `json` or `postgres` |
| `GITWHISPER_DB_BACKEND` | Short alias for database backend |
| `GITWHISPER_POSTGRES_URL` | PostgreSQL connection string |
| `GITWHISPER_DATABASE_URL` | Alias for PostgreSQL connection string |
| `GITWHISPER_DATABASE_PATH` | Override JSON storage path |
| `GITWHISPER_DB_PATH` | Short alias for JSON storage path |

---

## Docker And PostgreSQL

```bash
docker compose up --build
```

| Service | URL |
| --- | --- |
| GitWhisper dashboard | `http://localhost:7878` |
| Ollama | `http://localhost:11434` |
| PostgreSQL | `localhost:55432` |

For local CLI testing against Compose PostgreSQL:

```toml
[database]
backend = "postgres"
postgres_url = "postgres://postgres:postgres@localhost:55432/gitwhisper"
```

---

## Storage Layout

```text
.git/gitwhisper/
  cache/
  feedback/
  logs/
  <short-commit>.json
```

| Path | Purpose |
| --- | --- |
| `.git/gitwhisper/<short-commit>.json` | Captured commit context |
| `.git/gitwhisper/cache/cache-index.json` | Explanation cache metadata |
| `.git/gitwhisper/logs/audit.json` | JSON audit backend |
| `.git/gitwhisper/feedback/feedback.json` | JSON feedback backend |
| `refs/notes/gitwhisper` | Git notes for commit explanations |
| `exports/*.json` and `exports/*.csv` | User-generated exports, ignored by Git |

---

## Privacy

| Control | Behavior |
| --- | --- |
| `privacy.offline_mode = true` | Prevents cloud AI selection |
| `privacy.local_cache_only = true` | Keeps explanation cache local |
| `privacy.exclude_files` | Excludes matching file patterns |
| Integration `enabled = false` | External publishing remains disabled |

Cloud AI and team integrations are opt-in. Review `.gitwhisper.toml` before enabling external providers on private repositories.

---

## Release Status

| Check | Status |
| --- | --- |
| Version | `0.1.0` |
| License | MIT |
| Unit tests | 26 passing |
| PostgreSQL backend | Live-tested with Docker Compose |
| Feedback export | JSON and CSV tested |
| Audit prune/log | Tested on JSON and PostgreSQL paths |
| CI workflow | Added under `.github/workflows/ci.yml` |
| Release workflow | Added under `.github/workflows/release.yml` |

See [CHANGELOG.md](CHANGELOG.md), [docs/RELEASE.md](docs/RELEASE.md), and [docs/releases/v0.1.0.md](docs/releases/v0.1.0.md) before publishing the GitHub release.

---

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) for local setup, test expectations, and PR guidance.

Good contribution areas:

| Area | Why it helps |
| --- | --- |
| Analyzer tests | Makes risk reports more trustworthy |
| Integration tests | Protects provider and Postgres behavior |
| Dashboard polish | Makes team insights easier to scan |
| Language parsers | Improves semantic diff quality |

---

## Security

Please read [SECURITY.md](SECURITY.md) before reporting a vulnerability. Do not open public issues for suspected secrets, credential leaks, or exploitable vulnerabilities.

---

## License

GitWhisper is released under the [MIT License](LICENSE).
