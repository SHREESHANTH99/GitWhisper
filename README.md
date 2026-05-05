# GitWhisper

<p align="center">
  <img src="docs/assets/gitwhisper-hero.svg" alt="GitWhisper project banner" width="100%">
</p>

<p align="center">
  <b>AI-powered Git intelligence for developers and teams.</b><br/>
  GitWhisper captures commit context, analyzes change intent, explains code evolution, and turns raw Git history into practical engineering insight.
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> |
  <a href="#what-is-built-now">Built Now</a> |
  <a href="#pipelines-and-flow">Pipelines</a> |
  <a href="#commands">Commands</a> |
  <a href="#configuration">Configuration</a> |
  <a href="#docker-and-postgres">Docker</a> |
  <a href="#architecture">Architecture</a>
</p>

---

## Why GitWhisper Exists

Git already tells you what changed. GitWhisper helps answer the questions that usually live in a senior engineer's head:

- Why did this file change?
- What kind of change was this: feature, fix, refactor, security, performance, docs, or dependency work?
- What other files and modules might be affected?
- Which files are becoming risky because of churn, complexity, bugs, or single-person ownership?
- What should reviewers pay attention to before merging?
- How can a team preserve knowledge without manually writing documentation after every commit?

GitWhisper is a Rust CLI that stays useful in both solo and team workflows. It supports local JSON storage by default, a live-tested PostgreSQL backend for team deployments, AI explanations through cloud or local models, and heuristic analysis when AI is unavailable.

---

## What Is Built Now

GitWhisper currently includes the major working pieces from Phases 1 through 5 foundation work.

| Area | Status | What exists today |
| --- | --- | --- |
| Context intelligence | Working | Commit capture, environment capture, command redaction, IDE/review context hooks, semantic diff facts |
| Diff and intent analysis | Working | Diff stats, symbol/import signals, intent classification, urgency, risk, scope |
| Impact analysis | Working | Dependency graph hints, direct/transitive dependents, circular dependency detection |
| AI explanation | Working | Gemini cloud flow, Ollama local flow, hybrid selection, context optimization, reasoning-chain prompt builder |
| Summaries and ownership | Working | File evolution summaries, likely owners, team insights, wiki output, ADR output |
| Collaboration | Working | Git notes, Slack/Discord sharing, GitHub/GitLab review helpers, digests |
| Engineering health | Working | Quality, security, performance, bug prediction, knowledge risk, refactor priority |
| Feedback and audit | Working | Feedback submit/log/export, audit log/prune, local auth roles |
| Storage | Working | JSON backend and live-tested PostgreSQL backend |
| Docker foundation | Working | Compose stack with GitWhisper dashboard, Ollama sidecar, PostgreSQL |
| Enterprise scale | Foundation only | Auth/audit/DB modules exist; full SSO, RBAC policy engine, distributed workers are future work |

<p align="center">
  <img src="docs/assets/pipeline-overview.svg" alt="GitWhisper pipeline overview" width="100%">
</p>

---

## Quick Start

### 1. Build the CLI

```bash
cargo build --release
```

Install it into Cargo's binary path:

```bash
cargo install --path .
```

Or run the built executable directly:

```powershell
.\target\release\gitwhisper.exe --help
```

For development:

```bash
cargo run -- --help
```

### 2. Initialize GitWhisper in a repository

```bash
gitwhisper init
gitwhisper capture
gitwhisper annotate
```

`gitwhisper init` installs a managed `post-commit` hook. After that, GitWhisper can automatically capture context for new commits and optionally annotate them.

### 3. Ask your first questions

```bash
gitwhisper explain src/main.rs
gitwhisper summarize src/main.rs
gitwhisper owners src --limit 10
gitwhisper refactor-priority src --limit 10
```

### 4. Start the dashboard

```bash
gitwhisper dashboard --host 127.0.0.1 --port 7878
```

Open:

```text
http://127.0.0.1:7878
```

---

## Pipelines And Flow

This section explains what happens inside GitWhisper when you run the main commands.

### System Overview

```mermaid
flowchart LR
    dev["Developer"] --> cli["gitwhisper CLI"]
    cli --> git["Git repository"]
    cli --> capture["Context collectors"]
    cli --> analysis["Analysis engine"]
    analysis --> intent["Intent detection"]
    analysis --> impact["Impact analysis"]
    analysis --> health["Quality, security, performance, bug, knowledge risk"]
    capture --> storage["Persistence layer"]
    git --> analysis
    analysis --> optimizer["Context optimizer"]
    optimizer --> ai["AI layer: Gemini, Ollama, hybrid"]
    ai --> explain["Explanations and summaries"]
    health --> reports["Risk reports"]
    explain --> outputs["CLI, Git notes, PR comments, Slack, Discord, wiki, ADRs"]
    reports --> outputs
    storage --> outputs
```

GitWhisper starts with facts from Git and local context. The analyzers produce structured signals, the optimizer trims those signals into useful prompt context, and the AI layer turns them into readable explanations. Reports can stay local, go into Git notes, or publish to team tools.

### Commit Capture Pipeline

```mermaid
sequenceDiagram
    participant Dev as Developer
    participant Git as Git
    participant Hook as post-commit hook
    participant CLI as gitwhisper
    participant Collectors as Collectors
    participant Store as .git/gitwhisper
    Dev->>Git: git commit
    Git->>Hook: run post-commit
    Hook->>CLI: gitwhisper capture / annotate
    CLI->>Git: read HEAD, diff, author, branch
    CLI->>Collectors: collect commands, env, IDE, review context
    Collectors-->>CLI: sanitized context
    CLI->>Store: write commit context JSON
    CLI->>Git: optional Git note with explanation
```

The capture step is designed to be useful but conservative. It stores metadata and context, not full IDE file contents. Command capture redacts common secret patterns before data is persisted.

### Semantic Analysis Pipeline

```mermaid
flowchart TD
    patch["Raw git patch"] --> parser["Diff parser"]
    parser --> stats["File stats: added, removed, renamed, line delta"]
    parser --> symbols["Symbol and import signals"]
    parser --> complexity["Rough complexity delta"]
    stats --> intent["Intent detection"]
    symbols --> intent
    complexity --> intent
    intent --> impact["Impact analysis"]
    impact --> featureVector["ML-ready feature vector"]
    featureVector --> reports["Explain, quality, risk, and dashboard reports"]
```

The analyzer looks beyond filenames. It extracts change shape, rough complexity movement, import/dependency changes, and commit-message intent signals. These facts feed both AI explanations and non-AI reports.

### Explain Command Pipeline

```mermaid
flowchart TD
    command["gitwhisper explain path/to/file"] --> history["Load file history"]
    history --> contexts["Load captured commit contexts"]
    contexts --> relevance["Rank relevant commits"]
    relevance --> budget["Apply prompt budget"]
    budget --> reasoning["Build reasoning-chain prompt"]
    reasoning --> selector["Model selector"]
    selector --> local["Ollama local model"]
    selector --> cloud["Gemini cloud model"]
    selector --> fallback["Heuristic fallback"]
    local --> answer["Human explanation"]
    cloud --> answer
    fallback --> answer
    answer --> cache["Cache explanation"]
    answer --> terminal["Print result"]
```

The model selector chooses local, cloud, or fallback behavior based on configuration and prompt size. This keeps the tool fast for small local explanations while still allowing deeper cloud analysis when requested.

### Engineering Health Pipeline

<p align="center">
  <img src="docs/assets/risk-dashboard.svg" alt="GitWhisper engineering health dashboard illustration" width="100%">
</p>

```mermaid
flowchart LR
    repo["Repository files"] --> quality["Quality analyzer"]
    repo --> security["Security analyzer"]
    repo --> performance["Performance analyzer"]
    history["Git history"] --> bug["Bug predictor"]
    history --> knowledge["Knowledge risk"]
    quality --> priority["Refactor priority"]
    security --> priority
    performance --> priority
    bug --> priority
    knowledge --> priority
    priority --> cli["Ranked CLI report"]
    priority --> dashboard["Dashboard and exports"]
```

The health pipeline is heuristic today. It ranks files using churn, complexity pressure, repeated patterns, security markers, performance markers, bug-fix history, ownership concentration, and contributor spread. The goal is not to replace review; it is to point reviewers and maintainers at the places most worth inspecting.

### Collaboration Pipeline

```mermaid
flowchart TD
    annotate["gitwhisper annotate"] --> summary["Commit explanation"]
    summary --> notes["Git notes: refs/notes/gitwhisper"]
    summary --> webhook["Webhook payload"]
    webhook --> slack["Slack"]
    webhook --> discord["Discord"]
    summary --> pr["GitHub PR / GitLab MR"]
    pr --> reviewers["Reviewer context: intent, impact, risk"]
    notes --> future["Future explain and summarize commands"]
```

Collaboration output is opt-in. Git notes are local/repo-native; Slack, Discord, GitHub, and GitLab require explicit integration configuration.

### Storage Pipeline

```mermaid
flowchart LR
    cli["gitwhisper CLI"] --> dbLayer["DB abstraction layer"]
    dbLayer --> json["JSON backend"]
    dbLayer --> postgres["PostgreSQL backend"]
    json --> gitDir[".git/gitwhisper/logs and feedback"]
    postgres --> tables["feedback and audit_events tables"]
    tables --> exports["feedback-export, audit-log, dashboard"]
    gitDir --> exports
```

The JSON backend is the default local path. PostgreSQL is available for team or Docker-based use and has been live-tested with feedback and audit workflows.

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
| `gitwhisper refactor-priority [path] --limit 10` | Rank the files most worth refactoring |

### Collaboration And Publishing

| Command | Purpose |
| --- | --- |
| `gitwhisper share slack [commit]` | Send a commit explanation to Slack |
| `gitwhisper share discord [commit]` | Send a commit explanation to Discord |
| `gitwhisper review github [commit]` | Publish a GitHub PR review helper summary |
| `gitwhisper review gitlab [commit]` | Publish a GitLab MR review helper summary |
| `gitwhisper digest slack --period daily` | Send a Slack digest |
| `gitwhisper digest discord --period weekly` | Send a Discord digest |

### Platform, Docs, Feedback, Audit

| Command | Purpose |
| --- | --- |
| `gitwhisper dashboard --host 127.0.0.1 --port 7878` | Start the analytics dashboard |
| `gitwhisper export --format json --output exports/snapshot.json` | Export analytics JSON |
| `gitwhisper export --format csv --output exports/snapshot.csv` | Export analytics CSV |
| `gitwhisper wiki --output wiki` | Generate markdown wiki pages |
| `gitwhisper adr --output docs/adrs` | Generate ADR files |
| `gitwhisper feedback <commit> --good` | Store positive explanation feedback |
| `gitwhisper feedback <commit> --poor --correct "..."` | Store corrected feedback |
| `gitwhisper feedback-log --limit 20` | Show recent feedback |
| `gitwhisper feedback-export --format json --output exports/feedback.json` | Export feedback |
| `gitwhisper whoami` | Show resolved local auth identity |
| `gitwhisper audit-log --limit 20` | Show audit events |
| `gitwhisper audit-prune --days 90` | Prune old audit events |

---

## Configuration

GitWhisper reads `.gitwhisper.toml` from the repository root. It also reads `.env` through `dotenvy`, so environment variables such as `GEMINI_API_KEY` can live outside your shell profile.

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

[capture]
command_limit = 25
include_environment = true
include_analysis = true

[collaboration]
auto_annotate_commits = true
enable_git_notes = true
git_notes_ref = "refs/notes/gitwhisper"
webhook_url = ""
webhook_timeout_secs = 10

[integrations.slack]
enabled = false
webhook_url = ""
bot_token = ""
channel = ""
auto_share_on_commit = false
include_digest = false

[integrations.discord]
enabled = false
webhook_url = ""
auto_share_on_commit = false
include_digest = false

[integrations.github]
enabled = false
token = ""
api_url = "https://api.github.com"
auto_comment_on_pr = false
update_pr_description = false

[integrations.gitlab]
enabled = false
token = ""
api_url = "https://gitlab.com/api/v4"
auto_comment_on_mr = false
update_mr_description = false

[privacy]
offline_mode = false
local_cache_only = true
exclude_files = []

[database]
backend = "json" # json | postgres
path = ".git/gitwhisper/gitwhisper.db"
postgres_url = ""

[audit]
enabled = true
retain_days = 90

[auth]
enabled = false
mode = "disabled" # disabled | local
default_role = "admin"

[[auth.users]]
username = "docker-admin"
role = "admin"

[feedback]
enabled = true
allow_anonymous = false
```

### Environment Overrides

| Variable | Purpose |
| --- | --- |
| `GEMINI_API_KEY` | Cloud AI key used by Gemini flows |
| `GITWHISPER_USER` | Overrides detected username for auth/audit |
| `GITWHISPER_DATABASE_BACKEND` | Overrides database backend, for example `json` or `postgres` |
| `GITWHISPER_DB_BACKEND` | Short alias for database backend |
| `GITWHISPER_POSTGRES_URL` | PostgreSQL connection string |
| `GITWHISPER_DATABASE_URL` | Alias for PostgreSQL connection string |
| `GITWHISPER_DATABASE_PATH` | Overrides JSON database path |
| `GITWHISPER_DB_PATH` | Short alias for JSON database path |

---

## Docker And Postgres

GitWhisper includes a Compose stack for a local team-style deployment.

```bash
docker compose up --build
```

Default services:

| Service | URL |
| --- | --- |
| GitWhisper dashboard | `http://localhost:7878` |
| Ollama | `http://localhost:11434` |
| PostgreSQL | `localhost:55432` |

Compose passes these defaults into the app container:

```text
GITWHISPER_DATABASE_BACKEND=postgres
GITWHISPER_POSTGRES_URL=postgres://postgres:postgres@postgres:5432/gitwhisper
```

For local CLI testing against Compose PostgreSQL, use:

```toml
[database]
backend = "postgres"
postgres_url = "postgres://postgres:postgres@localhost:55432/gitwhisper"
```

The PostgreSQL backend currently creates and uses:

| Table | Purpose |
| --- | --- |
| `feedback` | Stores explanation ratings, corrections, tags, actor, commit, timestamp |
| `audit_events` | Stores actor, action, target, outcome, timestamp, metadata |

---

## Architecture

```mermaid
flowchart TB
    main["main.rs command dispatch"] --> cli["cli.rs Clap definitions"]
    main --> viewer["viewer modules"]
    viewer --> history["history and git helpers"]
    viewer --> analysis["analysis modules"]
    viewer --> ai["ai modules"]
    main --> capture["capture.rs"]
    capture --> collectors["collectors"]
    capture --> storage["storage"]
    main --> collaboration["collaboration.rs"]
    collaboration --> integrations["integrations"]
    main --> dashboard["dashboard.rs"]
    dashboard --> metrics["metrics"]
    main --> generators["wiki and ADR generators"]
    main --> feedback["feedback.rs"]
    feedback --> auth["auth"]
    feedback --> audit["audit"]
    feedback --> db["db abstraction"]
    audit --> db
    db --> json["JSON files"]
    db --> pg["PostgreSQL"]
```

Top-level module map:

| Module | Responsibility |
| --- | --- |
| `src/analysis/` | Diff parsing, intent detection, impact analysis, behavior, quality/security/performance/bug/knowledge/refactor analyzers |
| `src/ai/` | Cloud model, local model, model selector, context optimizer, reasoning-chain prompt generation |
| `src/collectors/` | Command, environment, IDE, and review-context collection |
| `src/storage/` | Context persistence, caching, predictive cache helpers |
| `src/integrations/` | Slack, Discord, GitHub, GitLab integrations |
| `src/metrics/` | Dashboard/export snapshots |
| `src/generators/` | Wiki and ADR generation |
| `src/auth/` | Local identity and permission checks |
| `src/audit/` | Audit event recording and pruning |
| `src/db/` | JSON/PostgreSQL persistence abstraction |
| `src/viewer/` | User-facing command implementations |

---

## Data And Privacy Model

```mermaid
flowchart LR
    local["Local repo"] --> gitDir[".git/gitwhisper"]
    local --> json["JSON backend"]
    local --> pg["Optional Postgres"]
    gitDir --> explain["Explain and summarize"]
    json --> feedback["Feedback and audit"]
    pg --> feedback
    explain --> cloud["Cloud AI only if configured"]
    explain --> ollama["Local Ollama if configured"]
```

Privacy defaults are conservative:

| Control | Behavior |
| --- | --- |
| `privacy.offline_mode = true` | Prevents cloud AI selection |
| `privacy.local_cache_only = true` | Keeps explanation cache local |
| `privacy.exclude_files` | Excludes matching file patterns from capture/analysis flows |
| Integration `enabled = false` | Slack, Discord, GitHub, GitLab publishing is off until configured |

GitWhisper can still call cloud AI if you configure cloud mode and provide credentials. For private repositories, review `.gitwhisper.toml` before enabling external integrations.

---

## Storage Layout

Local JSON-backed data is written under the repository `.git` directory:

```text
.git/gitwhisper/
  cache/
  feedback/
  logs/
  <short-commit>.json
```

Common outputs:

| Path | Purpose |
| --- | --- |
| `.git/gitwhisper/<short-commit>.json` | Captured commit context |
| `.git/gitwhisper/cache/cache-index.json` | Explanation cache metadata |
| `.git/gitwhisper/logs/audit.json` | JSON audit backend |
| `.git/gitwhisper/feedback/feedback.json` | JSON feedback backend |
| `refs/notes/gitwhisper` | Git notes for commit explanations |
| `exports/*.json` and `exports/*.csv` | Analytics and feedback exports |
| `wiki/` | Generated project wiki |
| `docs/adrs/` | Generated architecture decision records |

---

## Example Workflows

### Explain a confusing file

```bash
gitwhisper explain src/auth.rs
gitwhisper timeline src/auth.rs
gitwhisper summarize src/auth.rs
```

Use this when you inherited a file and want a narrative instead of reading every commit manually.

### Prepare for review

```bash
gitwhisper annotate
gitwhisper security src
gitwhisper performance src
gitwhisper refactor-priority src --limit 10
```

Use this before opening a PR or when reviewing a risky change.

### Find ownership risk

```bash
gitwhisper owners src/api --limit 10
gitwhisper knowledge-risk src --limit 10
gitwhisper bug-predict src --limit 10
```

Use this to identify files that depend too heavily on one person or have a risky change history.

### Build a lightweight knowledge base

```bash
gitwhisper wiki --output wiki
gitwhisper adr --output docs/adrs
```

Use this when you want repository knowledge to become searchable documentation.

### Capture feedback and audit it

```bash
gitwhisper feedback HEAD --good --tags accurate,helpful
gitwhisper feedback HEAD --poor --correct "This was actually a refactor."
gitwhisper feedback-log --limit 20
gitwhisper feedback-export --format csv --output exports/feedback.csv
gitwhisper audit-log --limit 20
gitwhisper audit-prune --days 90
```

Use this to build a feedback dataset for explanation quality and keep an auditable trail of user-facing actions.

---

## Dashboard Endpoints

When the dashboard is running:

| Endpoint | Purpose |
| --- | --- |
| `/` | Interactive dashboard |
| `/snapshot.json` | Machine-readable analytics snapshot |
| `/snapshot.csv` | CSV analytics snapshot |
| `/healthz` | Health check |

---

## Testing

```bash
cargo test
cargo fmt
cargo clippy -- -D warnings
```

Current validation from the latest implementation pass:

| Check | Result |
| --- | --- |
| Unit tests | `26/26` passing |
| PostgreSQL backend | Live-tested with Compose |
| Feedback export | JSON and CSV tested |
| Audit prune/log | Tested on JSON and PostgreSQL paths |

---

## Roadmap

GitWhisper is intentionally growing from local-first CLI toward team and enterprise intelligence.

```mermaid
timeline
    title GitWhisper roadmap
    Phase 1 : Context intelligence
            : Diff parser
            : Intent and impact analysis
    Phase 2 : AI intelligence
            : Gemini, Ollama, hybrid selector
            : Context optimizer and reasoning-chain prompts
    Phase 3 : Collaboration
            : Git notes, Slack, Discord
            : GitHub and GitLab review helpers
    Phase 4 : Advanced analyzers
            : Quality, security, performance
            : Bug prediction, knowledge risk, feedback
    Phase 5 : Enterprise foundation
            : Docker, auth, audit, JSON/Postgres DB layer
    Phase 6+ : Future scale
            : richer analytics, query language, workers, SSO
```

Next high-value upgrades:

1. Add CI workflows for build, test, fmt, and clippy.
2. Add a `LICENSE` file before public release.
3. Add real dashboard screenshots or GIFs after the UI is stabilized.
4. Add integration tests for PostgreSQL and Docker.
5. Expand tree-sitter language-aware parsing for more precise function-level analysis.

---

## Contributing

Good first contribution areas:

| Area | Why it helps |
| --- | --- |
| Analyzer tests | Makes risk reports more trustworthy |
| Docs examples | Helps users understand workflows faster |
| Integration tests | Protects Slack/GitHub/GitLab/Postgres behavior |
| Dashboard polish | Makes team insights easier to scan |
| Language parsers | Improves semantic diff quality |

Recommended flow:

```bash
git checkout -b feat/your-change
cargo fmt
cargo test
cargo clippy -- -D warnings
```

---

## FAQ

### Is the project GitWhisper or CommitLens?

The Rust package and CLI are `gitwhisper`. This README uses GitWhisper as the product name.

### Does GitWhisper require cloud AI?

No. You can use local Ollama mode or let non-AI analyzers provide reports. Cloud AI is only used when configured.

### Does it send code to Slack, GitHub, or other services by default?

No. Integrations are disabled by default and must be explicitly configured.

### Is PostgreSQL required?

No. JSON storage is the default. PostgreSQL is available for team or Docker-backed use.

### Is the enterprise roadmap finished?

No. The foundation exists: Docker, auth module, audit module, feedback, and DB abstraction. Full SSO, advanced RBAC, distributed workers, and managed cloud deployment are future work.

---

## License

A license file is not currently present in the repository root. Add one before public release so contributors and users know how they can use the project.
