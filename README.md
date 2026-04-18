# Gitwhisper

AI-powered Git commit intelligence for developers and teams.

Gitwhisper captures commit-time context, analyzes diffs semantically, infers likely developer intent, and uses AI to explain how files and features evolved. It is built as a Rust CLI and is designed to stay useful even when cloud AI is unavailable by supporting local and heuristic fallbacks.

All command examples below assume the binary is installed as `gitwhisper`. During development, you can run the same commands with `cargo run -- <command>`.

## Current Status

Gitwhisper currently includes the major deliverables from the first three phases of the roadmap:

| Phase | Status | Implemented today |
| --- | --- | --- |
| Phase 1: Context intelligence | Implemented | Semantic diff parsing, intent detection, impact analysis, IDE/review/behavior capture, local caching |
| Phase 2: AI intelligence | Implemented | Gemini cloud backend, Ollama local backend, hybrid model selection, prompt/context optimization, reasoning-chain prompts, file summaries, ownership insights |
| Phase 3: Collaboration | Implemented in a practical repo-native form | Post-commit annotations, Git notes, Slack/Discord sharing, GitHub/GitLab review posting, analytics dashboard, JSON/CSV export, wiki generation, ADR generation |
| Phase 4+ | Not yet implemented | Advanced quality analyzers, predictive bug analysis, enterprise auth, deeper analytics, REPL/query language, monetization features |

This README focuses on what is already available in the codebase right now.

## What Gitwhisper Does Today

- Captures commit context for the current `HEAD` commit into `.git/gitwhisper/`.
- Parses Git patches into structured diff facts such as file operations, line counts, import changes, symbol changes, and rough complexity deltas.
- Infers change category, urgency, scope, and risk from commit messages plus diff signals.
- Builds impact hints such as direct dependents, transitive dependents, circular dependency chains, and an impact score.
- Explains why a file changed using commit history, captured context, and AI.
- Summarizes the evolution of a file over time.
- Shows likely code owners for a file or directory.
- Annotates commits with compact AI-generated summaries and stores them in Git notes.
- Shares commit summaries and digests to Slack or Discord.
- Publishes review-style summaries to GitHub pull requests or GitLab merge requests.
- Serves a lightweight local team dashboard with ownership, hotspot, trend, and risk views.
- Exports analytics snapshots as JSON or CSV.
- Generates markdown wiki pages and ADR-style decision records from captured repository knowledge.

## Quick Start

### 1. Install or build

```bash
cargo install --path .
```

Or, if you only want a local build:

```bash
cargo build --release
```

### 2. Create `.gitwhisper.toml`

Minimal hybrid setup:

```toml
[ai]
provider = "hybrid"
model = "gemini-1.5-flash"
local_model = "mistral"
prompt_char_budget = 12000
history_depth = 10

[capture]
command_limit = 25
include_environment = true
include_analysis = true

[collaboration]
auto_annotate_commits = true
enable_git_notes = true

[privacy]
offline_mode = false
local_cache_only = true
```

### 3. Configure AI

For Gemini cloud usage:

```bash
export GEMINI_API_KEY="your-key"
```

PowerShell:

```powershell
$env:GEMINI_API_KEY = "your-key"
```

For local AI usage, make sure Ollama is running and the configured model is available:

```bash
ollama pull mistral
ollama serve
```

### 4. Install the managed post-commit hook

```bash
gitwhisper init
```

This installs a `post-commit` hook that runs `gitwhisper post-commit`, which captures the latest commit context and, if enabled in config, annotates the commit automatically.

### 5. Use the core commands

```bash
gitwhisper explain src/auth.rs
gitwhisper summarize src/auth.rs
gitwhisper owners src/auth.rs
gitwhisper annotate
gitwhisper dashboard --host 127.0.0.1 --port 7878
```

## Command Reference

### Core history and explanation commands

| Command | What it does |
| --- | --- |
| `gitwhisper init` | Installs the managed `post-commit` hook |
| `gitwhisper capture` | Captures context for the current `HEAD` commit |
| `gitwhisper annotate [commit]` | Captures context, generates a compact commit summary, writes Git notes, and may trigger configured delivery |
| `gitwhisper log` | Lists saved commit context entries |
| `gitwhisper replay [commit]` | Replays captured developer activity for a commit |
| `gitwhisper timeline <file>` | Shows the commit timeline for a file |
| `gitwhisper explain <file>` | Explains why the file changed using history plus captured context |
| `gitwhisper summarize <file>` | Generates a file-evolution narrative |
| `gitwhisper owners <path> --limit 10` | Shows top contributors for a file or directory |

### Collaboration and reporting commands

| Command | What it does |
| --- | --- |
| `gitwhisper share slack [commit]` | Sends a commit explanation to Slack |
| `gitwhisper share discord [commit]` | Sends a commit explanation to Discord |
| `gitwhisper review github [commit]` | Publishes a review summary to a GitHub PR |
| `gitwhisper review gitlab [commit]` | Publishes a review summary to a GitLab MR |
| `gitwhisper digest slack --period daily` | Sends a daily digest to Slack |
| `gitwhisper digest discord --period weekly` | Sends a weekly digest to Discord |
| `gitwhisper dashboard --host 127.0.0.1 --port 7878` | Starts the built-in analytics dashboard |
| `gitwhisper export --format json --output exports/snapshot.json` | Exports analytics snapshot as JSON |
| `gitwhisper export --format csv --output exports/snapshot.csv` | Exports analytics snapshot as CSV |
| `gitwhisper wiki --output wiki` | Generates markdown wiki pages and search index |
| `gitwhisper adr --output docs/adrs` | Generates ADR markdown files |

`gitwhisper post-commit` also exists, but it is an internal command used by the installed hook.

## Implemented Features

### Phase 1: Context Intelligence

#### Semantic diff analysis

Gitwhisper parses raw Git patches into structured change data, including:

- files changed, added, deleted, and renamed
- lines added, removed, and net line delta
- per-file diff statistics
- import and dependency statement changes
- symbol-level changes such as functions, modules, and types
- rough complexity delta based on changed code shape

This is implemented in the analysis layer and stored inside each captured commit context.

#### Intent detection

Gitwhisper classifies commit intent without requiring explicit developer input. The current implementation detects:

- category: `bug-fix`, `feature`, `refactor`, `performance`, `documentation`, `dependency-update`, `test`, `chore`
- urgency: `low`, `normal`, `high`, `critical`
- risk: `low`, `medium`, `high`, `critical`
- scope: `single-file`, `cross-file`, `broad`
- conventional commit fields, including scope and breaking-change markers
- confidence and signal traces used to make the decision

#### Impact analysis

Impact analysis currently records:

- impact score
- direct dependents
- transitive dependents
- circular dependency chains

This gives the explanation pipeline better hints about blast radius and related files.

#### Expanded context capture

Each stored commit context can include:

- commands run before commit
- environment metadata such as OS, branch, shell, working directory, and tool versions
- IDE/editor metadata such as process, version, build system, extensions, and active files
- review and test metadata such as PR number, reviewers, labels, test status, and coverage when available
- behavioral context such as recent commit frequency, work-hour patterns, burnout risk, and per-file expertise

IDE and review/test capture are best-effort. Gitwhisper does not store editor file contents from IDE capture.

#### Caching

Explanation caching currently includes:

- in-memory cache for the latest results
- on-disk cache index under `.git/gitwhisper/cache/`
- access counts and timestamps
- related-file prediction helpers used to preselect useful neighboring context

### Phase 2: AI Intelligence

#### Multi-model AI architecture

Gitwhisper supports three AI modes:

- `cloud`: Gemini-based explanation flow
- `local`: Ollama-based explanation flow
- `hybrid`: chooses local or cloud depending on prompt size, availability, and configuration

If cloud AI is unavailable and local AI is not configured, Gitwhisper falls back to heuristic summaries instead of failing hard.

#### Context window optimization

The explanation stack now trims and prioritizes prompt context using:

- configurable history depth
- prompt character budget
- relevance-based commit selection
- related-file prediction signals

This keeps prompts focused while still preserving useful historical context.

#### Reasoning-chain prompt construction

Before sending an AI request, Gitwhisper assembles a structured prompt that includes:

- what changed
- detected intent
- impact hints
- behavioral and review context
- related history

This produces better file explanations and better commit annotations than a plain "explain this diff" prompt.

#### File summaries and ownership insights

Phase 2 also added:

- `gitwhisper summarize <file>` for file-evolution narratives
- `gitwhisper owners <path>` for top-contributor and knowledge-silo visibility

### Phase 3: Collaboration and Team Knowledge

#### Commit annotations and Git notes

`gitwhisper annotate` prepares a commit report, generates a compact summary, writes a Git note, and can optionally fan that report out to external tools. The post-commit hook can run this automatically after each commit.

Git notes are written to the configured ref:

```text
refs/notes/gitwhisper
```

#### Team sharing

Gitwhisper can send:

- single commit summaries to Slack
- single commit summaries to Discord
- daily digests
- weekly digests

Slack and Discord delivery both require valid webhook or bot configuration.

#### PR and MR review posting

Gitwhisper can publish commit-aware review summaries to:

- GitHub pull requests
- GitLab merge requests

The current implementation resolves the remote from `origin` and uses the configured provider credentials.

#### Local analytics dashboard

The dashboard is a lightweight built-in HTTP server. It serves:

- `/` for the HTML dashboard
- `/snapshot.json` for analytics JSON
- `/snapshot.csv` for analytics CSV
- `/healthz` for a simple health check

The dashboard currently shows:

- overview metrics
- contributor activity
- hot files and ownership concentration
- weekly activity trend
- simple risk signals
- recent commits

#### Metrics export

Analytics can be exported as:

- JSON for tooling or downstream analysis
- CSV for spreadsheets and lightweight reporting

#### Wiki and ADR generation

Gitwhisper can generate project documentation directly from captured repository history:

- wiki output includes `index.md`, per-file pages, per-person pages, and `search-index.json`
- ADR output includes a `README.md` index and numbered ADR markdown files

## Captured Commit Context Format

Commit context is stored under:

```text
.git/gitwhisper/<short-commit>.json
```

Current schema example:

```json
{
  "schema_version": 3,
  "commit": "f6e3058",
  "timestamp": "2026-03-07T12:14:26Z",
  "commands": ["cargo test", "git add src/auth.rs"],
  "environment": {
    "os": "windows",
    "branch": "main",
    "shell": "powershell.exe",
    "working_directory": "C:/repo",
    "tools": {
      "node": "v22.14.0",
      "python": "3.12.2",
      "rust": "1.77.2"
    }
  },
  "ide": {
    "name": "VSCode",
    "process": "Code.exe",
    "version": "1.88.0",
    "build_system": "cargo",
    "extensions": ["rust-analyzer"],
    "active_files": ["src/auth.rs"]
  },
  "review": {
    "ci_provider": "github-actions",
    "pr_number": "42",
    "reviewers": ["alice", "bob"],
    "labels": ["security"],
    "milestone": "v0.3",
    "test_status": "passed",
    "tests_run": 124,
    "tests_failed": 0,
    "coverage_percent": 81,
    "source": "github"
  },
  "behavior": {
    "author": "alice@example.com",
    "commits_last_7d": 12,
    "commits_last_30d": 41,
    "late_night_ratio": 8,
    "typical_work_hours": "09:00-18:00",
    "burnout_risk": "normal",
    "expertise": [
      { "path": "src/auth.rs", "commit_count": 9 }
    ]
  },
  "files": ["src/auth.rs"],
  "analysis": {
    "intent": {
      "category": "bug-fix",
      "urgency": "normal",
      "risk": "medium",
      "scope": "single-file",
      "conventional_type": "fix",
      "conventional_scope": "auth",
      "breaking_change": false,
      "signals": ["type `fix`", "conventional commit header"],
      "confidence": 94
    },
    "diff": {
      "files_changed": 1,
      "files_added": 0,
      "files_deleted": 0,
      "files_renamed": 0,
      "lines_added": 18,
      "lines_removed": 4,
      "net_lines": 14,
      "complexity_delta": 2
    },
    "impact": {
      "impact_score": 42,
      "direct_dependents": ["src/session.rs"],
      "transitive_dependents": ["src/api/auth_handler.rs"],
      "circular_dependencies": []
    }
  }
}
```

## Configuration Reference

Gitwhisper loads `.gitwhisper.toml` from the repository root and also reads `.env` so `GEMINI_API_KEY` can be supplied without passing `--api-key` every time.

Full example:

```toml
[ai]
provider = "hybrid"         # cloud | local | hybrid
model = "gemini-1.5-flash"
local_model = "mistral"
prompt_char_budget = 12000
hybrid_max_prompt_chars = 8000
ollama_url = "http://localhost:11434"
history_depth = 10
request_timeout_secs = 45

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
exclude_files = ["*.key", "*.secret"]
```

## Storage and Outputs

Gitwhisper currently writes data to these places:

- `.git/gitwhisper/<short-commit>.json` for captured commit contexts
- `.git/gitwhisper/cache/cache-index.json` for explanation cache metadata
- `.git/gitwhisper/logs/ai.log` for AI request and fallback logging
- `.git/gitwhisper/logs/collaboration.log` for annotation and delivery events
- Git notes under `refs/notes/gitwhisper` by default
- user-selected output directories for exports, wiki pages, and ADRs

## Example Workflows

### Explain why a file changed

```bash
gitwhisper explain src/auth.rs
```

### Tell the story of a file

```bash
gitwhisper summarize src/auth.rs
gitwhisper timeline src/auth.rs
```

### Find who knows a file best

```bash
gitwhisper owners src/auth.rs --limit 5
```

### Annotate the latest commit and store Git notes

```bash
gitwhisper annotate
git notes --ref refs/notes/gitwhisper show HEAD
```

### Share updates with the team

```bash
gitwhisper share slack
gitwhisper digest discord --period weekly
```

### Generate documentation from repository history

```bash
gitwhisper wiki --output wiki
gitwhisper adr --output docs/adrs
```

## Project Structure

```text
src/
  ai/             cloud/local/hybrid AI backends and prompt optimization
  analysis/       diff parsing, intent detection, impact analysis, behavior patterns
  collectors/     command, environment, IDE, and review-context collection
  generators/     wiki and ADR generation
  integrations/   Slack, Discord, GitHub, and GitLab delivery
  metrics/        analytics snapshot and export
  storage/        context persistence and caching
  viewer/         explain, summarize, replay, timeline, owners, log views
  capture.rs      commit capture pipeline
  collaboration.rs commit annotation and delivery flow
  dashboard.rs    built-in analytics dashboard server
  history.rs      commit history helpers
  hooks.rs        managed post-commit hook installer
  config.rs       .gitwhisper.toml parsing
  cli.rs          Clap command definitions
  main.rs         command dispatch
```

## Notes and Limitations

- Phase 4 and later roadmap items are not implemented yet, so this README intentionally does not claim predictive bug analysis, enterprise auth, advanced wellness analytics, or interactive REPL/query features.
- IDE capture and review metadata are best-effort and depend on local processes, repository remotes, and available provider metadata.
- Slack, Discord, GitHub, and GitLab integrations require valid configuration and reachable network access.
- The dashboard is intentionally lightweight and built into the CLI rather than being a separate full frontend application.

## Contributing

Issues, pull requests, and feature suggestions are welcome. If you are contributing, keeping the README aligned with actual implementation status is especially helpful because the roadmap is larger than the currently shipped surface area.

## License

MIT
