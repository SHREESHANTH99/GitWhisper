<div align="center">

<img src="docs/assets/gitwhisper-hero.svg" alt="GitWhisper Banner" width="100%"/>

<br/>

# 🔮 GitWhisper

### *Your codebase has a story. GitWhisper tells it.*

<br/>

[![Rust](https://img.shields.io/badge/Built_with-Rust-CE422B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-TBD-yellow?style=for-the-badge)](#license)
[![Tests](https://img.shields.io/badge/Tests-26%2F26_Passing-brightgreen?style=for-the-badge&logo=github-actions&logoColor=white)](#testing)
[![AI](https://img.shields.io/badge/AI-Gemini_%7C_Ollama-4285F4?style=for-the-badge&logo=google&logoColor=white)](#configuration)
[![Docker](https://img.shields.io/badge/Docker-Ready-2496ED?style=for-the-badge&logo=docker&logoColor=white)](#docker--postgres)
[![PostgreSQL](https://img.shields.io/badge/Storage-JSON_%7C_PostgreSQL-336791?style=for-the-badge&logo=postgresql&logoColor=white)](#docker--postgres)
[![Stars](https://img.shields.io/github/stars/SHREESHANTH99/GitWhisper?style=for-the-badge&color=FFD700&logo=github)](https://github.com/SHREESHANTH99/GitWhisper/stargazers)

<br/>

> **Git already tells you *what* changed. GitWhisper tells you *why*.**

<br/>

[🚀 Quick Start](#-quick-start) · [📐 Architecture](#-architecture) · [⚡ Commands](#-commands) · [🐳 Docker](#-docker--postgres) · [🗺 Roadmap](#-roadmap) · [🤝 Contributing](#-contributing)

---

</div>

## 🌟 What Is GitWhisper?

GitWhisper is a **Rust-powered CLI intelligence layer** on top of Git. It captures developer context at commit time, analyzes change intent using semantic diff analysis, and uses AI to generate human-readable explanations of why code evolved — not just what bytes moved.

Think of it as a **senior engineer living inside your terminal**, ready to explain any file's history, flag risks before review, and turn raw commit logs into living documentation.

```
┌─────────────────────────────────────────────────────────┐
│  $ gitwhisper explain src/auth.rs                       │
│                                                         │
│  📖 This file evolved through 3 phases:                 │
│  → [Week 1] Initial JWT scaffold (feature work)         │
│  → [Week 3] Token rotation added after security review  │
│  → [Week 5] Refactored to support OAuth2 providers      │
│                                                         │
│  ⚠  Risk: Single contributor owns 94% of this file.    │
│  🔍 Related: src/middleware.rs, src/session.rs          │
└─────────────────────────────────────────────────────────┘
```

---

## 🧠 Why GitWhisper Exists

Git history contains everything — but understanding it requires hours of reading commits. GitWhisper answers the questions that live in a senior engineer's head:

| ❓ Question | 💡 GitWhisper Answer |
|---|---|
| Why did this file change? | `gitwhisper explain <file>` |
| Who really owns this code? | `gitwhisper owners <path>` |
| Is this a security risk? | `gitwhisper security <path>` |
| Which files are getting messy? | `gitwhisper refactor-priority <path>` |
| What should reviewers focus on? | `gitwhisper annotate` |
| What happened in the last sprint? | `gitwhisper digest slack --period weekly` |
| How do I onboard a new dev? | `gitwhisper wiki --output wiki` |

---


### Module Map

![Module Map](docs/flowcharts/Django%20Form%20Modification-2026-05-05-175521.png)

---

## 🔄 Pipelines & Flows

### 1️⃣ Commit Capture Pipeline

Every `git commit` triggers an automatic capture pipeline:

![Commit Capture Pipeline](docs/flowcharts/Django%20Form%20Modification-2026-05-05-175612.png)

---


### 3️⃣ AI Explain Pipeline

![AI Explain Pipeline](docs/flowcharts/Django%20Form%20Modification-2026-05-05-175710.png)

---

### 4️⃣ Engineering Health Pipeline

![Engineering Health Pipeline](docs/flowcharts/Django%20Form%20Modification-2026-05-05-175824.png)

---

### 5️⃣ Collaboration & Publishing Pipeline

![Collaboration and Publishing Pipeline](docs/flowcharts/Django%20Form%20Modification-2026-05-05-175847.png)

---

### 6️⃣ Storage Pipeline

![Storage Pipeline](docs/flowcharts/Django%20Form%20Modification-2026-05-05-175912.png)

---

## 🚀 Quick Start

### Prerequisites

| Requirement | Minimum Version |
|---|---|
| [Rust](https://rustup.rs/) | `1.75+` (2021 edition) |
| [Git](https://git-scm.com/) | `2.30+` |
| [Ollama](https://ollama.ai/) *(optional)* | Any |
| [Docker](https://docker.com/) *(optional)* | `24+` |
| Gemini API Key *(optional)* | — |

---

### ⚡ Install in 3 Steps

**Step 1 — Clone & Build**
```bash
git clone https://github.com/SHREESHANTH99/GitWhisper.git
cd GitWhisper
cargo build --release
```

**Step 2 — Install to PATH**
```bash
# Install via Cargo (recommended)
cargo install --path .

# Or run the binary directly
./target/release/gitwhisper --help
```

**Step 3 — Configure your API key**
```bash
# Copy the example env file
cp .env.example .env

# Add your Gemini API key (or skip for Ollama/heuristic mode)
echo 'GEMINI_API_KEY=your_key_here' >> .env
```

---

### 🏁 First Run

```bash
# 1. Initialize GitWhisper in your repo (installs post-commit hook)
gitwhisper init

# 2. Capture context for the current commit
gitwhisper capture

# 3. Generate an AI explanation and store it
gitwhisper annotate

# 4. Ask your first question!
gitwhisper explain src/main.rs

# 5. Start the web dashboard
gitwhisper dashboard --host 127.0.0.1 --port 7878
# → Open http://127.0.0.1:7878
```

---

## ⚡ Commands

### 📖 Core History & Explanation

```bash
gitwhisper init                        # Install managed post-commit hook
gitwhisper capture                     # Capture context for HEAD commit
gitwhisper annotate [commit]           # Generate + store AI explanation in Git notes
gitwhisper log                         # Show captured context entries
gitwhisper replay [commit]             # Replay captured activity for a commit
gitwhisper timeline <file>             # Visual timeline of a file's commits
gitwhisper explain <file>              # ✨ AI explanation of why a file changed
gitwhisper summarize <file>            # Evolution narrative of a file
gitwhisper owners <path> --limit 10    # Likely code owners by contribution weight
```

### 🔬 Risk & Health Analysis

```bash
gitwhisper quality <path>                     # Complexity, duplication, churn, maintainability
gitwhisper security <path>                    # Security-sensitive patterns + risky changes
gitwhisper performance <path>                 # Performance hotspots + patterns
gitwhisper bug-predict [path] --limit 10      # Files most likely to contain bugs
gitwhisper knowledge-risk [path] --limit 10   # Ownership silos + contributor concentration
gitwhisper refactor-priority [path] --limit 10 # Files most worth refactoring NOW
```

### 🤝 Collaboration & Publishing

```bash
gitwhisper share slack [commit]             # Send commit explanation to Slack
gitwhisper share discord [commit]           # Send commit explanation to Discord
gitwhisper review github [commit]           # Post GitHub PR review helper summary
gitwhisper review gitlab [commit]           # Post GitLab MR review helper summary
gitwhisper digest slack --period daily      # Slack daily digest
gitwhisper digest discord --period weekly   # Discord weekly digest
```

### 🛠 Platform, Docs & Audit

```bash
gitwhisper dashboard --host 127.0.0.1 --port 7878   # Web dashboard
gitwhisper export --format json --output exports/snapshot.json
gitwhisper export --format csv --output exports/snapshot.csv
gitwhisper wiki --output wiki                         # Generate markdown wiki
gitwhisper adr --output docs/adrs                     # Generate ADR files
gitwhisper feedback <commit> --good                   # Rate explanation ✅
gitwhisper feedback <commit> --poor --correct "..."   # Correct explanation ✏️
gitwhisper feedback-log --limit 20
gitwhisper feedback-export --format json --output exports/feedback.json
gitwhisper whoami                                     # Show local auth identity
gitwhisper audit-log --limit 20
gitwhisper audit-prune --days 90
```

---

## 🔧 Configuration

GitWhisper reads `.gitwhisper.toml` from the repository root. Environment variables (via `.env`) override TOML values.

```toml
[ai]
provider = "hybrid"                  # cloud | local | hybrid
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

[integrations.slack]
enabled = false
webhook_url = ""
channel = ""

[integrations.github]
enabled = false
token = ""
auto_comment_on_pr = false

[database]
backend = "json"                     # json | postgres
path = ".git/gitwhisper/gitwhisper.db"
postgres_url = ""

[privacy]
offline_mode = false
local_cache_only = true
exclude_files = []

[audit]
enabled = true
retain_days = 90

[auth]
enabled = false
mode = "disabled"                    # disabled | local
default_role = "admin"
```

### 🌍 Environment Variables

| Variable | Purpose |
|---|---|
| `GEMINI_API_KEY` | Cloud AI key for Gemini flows |
| `GITWHISPER_USER` | Override detected username |
| `GITWHISPER_DATABASE_BACKEND` | `json` or `postgres` |
| `GITWHISPER_POSTGRES_URL` | PostgreSQL connection string |
| `GITWHISPER_DATABASE_URL` | Alias for PostgreSQL URL |
| `GITWHISPER_DATABASE_PATH` | Override JSON storage path |

---

## 🐳 Docker & Postgres

GitWhisper ships a full **Docker Compose stack** for team-style local deployment with Ollama, PostgreSQL, and the live dashboard.

![Docker Compose Stack](docs/flowcharts/Django%20Form%20Modification-2026-05-05-175947.png)

```bash
# Start the full stack (builds images automatically)
docker compose up --build

# Access services
open http://localhost:7878      # GitWhisper dashboard
# Ollama available at http://localhost:11434
# PostgreSQL at localhost:55432
```

**Default service URLs:**

| Service | URL | Purpose |
|---|---|---|
| 📊 GitWhisper Dashboard | `http://localhost:7878` | Analytics + insights UI |
| 🤖 Ollama | `http://localhost:11434` | Local AI inference |
| 🐘 PostgreSQL | `localhost:55432` | Feedback + audit storage |

**For local CLI testing against Compose PostgreSQL:**
```toml
[database]
backend = "postgres"
postgres_url = "postgres://postgres:postgres@localhost:55432/gitwhisper"
```

---

## 📁 Storage Layout

```
your-repo/
├── .gitwhisper.toml              ← Project configuration
├── .git/
│   ├── gitwhisper/
│   │   ├── <short-commit>.json  ← Captured commit context
│   │   ├── cache/
│   │   │   └── cache-index.json ← Explanation cache metadata
│   │   ├── logs/
│   │   │   └── audit.json       ← Audit event log
│   │   └── feedback/
│   │       └── feedback.json    ← Explanation ratings
│   └── notes/
│       └── gitwhisper           ← Git notes (commit explanations)
├── exports/
│   ├── snapshot.json            ← Analytics snapshot
│   ├── snapshot.csv
│   └── feedback.csv
├── wiki/                        ← Generated project wiki
└── docs/
    └── adrs/                    ← Architecture Decision Records
```

---

## 🔐 Privacy & Data Model

![Privacy and Data Model](docs/flowcharts/Django%20Form%20Modification-2026-05-05-180021.png)

**Privacy controls:**

| Setting | Effect |
|---|---|
| `privacy.offline_mode = true` | Blocks all cloud AI selection |
| `privacy.local_cache_only = true` | Keeps explanation cache on disk only |
| `privacy.exclude_files = [...]` | Skips files matching patterns |
| All integration `enabled = false` | No data leaves your machine (default) |

> ⚠️ Cloud AI is **never** called unless you explicitly configure `provider = "cloud"` or `"hybrid"` and provide credentials. Local Ollama mode is fully air-gapped.

---

## 🏃 Example Workflows

### 🕵️ Understand a Confusing File
```bash
# Build a complete picture of why auth.rs is the way it is
gitwhisper timeline src/auth.rs     # See all commits chronologically
gitwhisper explain src/auth.rs      # Get AI narrative of the evolution
gitwhisper summarize src/auth.rs    # Read the file's story in prose
gitwhisper owners src/auth.rs       # Find out who to ask questions
```

### 🔍 Pre-Review Checklist
```bash
# Run before opening a PR or reviewing risky changes
gitwhisper annotate                          # AI-annotate your latest commit
gitwhisper security src                      # Flag security-sensitive patterns
gitwhisper performance src                   # Find performance hotspots
gitwhisper refactor-priority src --limit 10  # Most critical files to review
```

### 🚨 Find Bus Factor Risk
```bash
# Identify knowledge silos before they become outages
gitwhisper owners src/api --limit 10
gitwhisper knowledge-risk src --limit 10
gitwhisper bug-predict src --limit 10
```

### 📚 Generate Living Documentation
```bash
# Turn Git history into searchable knowledge
gitwhisper wiki --output wiki
gitwhisper adr --output docs/adrs
```

### 📊 Team Feedback Loop
```bash
gitwhisper feedback HEAD --good --tags "accurate,helpful"
gitwhisper feedback HEAD --poor --correct "This was a refactor, not a bugfix."
gitwhisper feedback-log --limit 20
gitwhisper feedback-export --format csv --output exports/feedback.csv
gitwhisper audit-log --limit 20
gitwhisper audit-prune --days 90
```

---

## 📊 Dashboard Endpoints

When `gitwhisper dashboard` is running:

| Endpoint | Type | Purpose |
|---|---|---|
| `/` | HTML | Interactive analytics dashboard |
| `/snapshot.json` | JSON | Machine-readable analytics snapshot |
| `/snapshot.csv` | CSV | Spreadsheet-compatible analytics export |
| `/healthz` | JSON | Health check for monitoring |

---

## ✅ Build Status & Testing

```bash
cargo test          # Run all unit tests
cargo fmt           # Format code
cargo clippy -- -D warnings   # Lint with warnings-as-errors
```

| Check | Result |
|---|---|
| ✅ Unit Tests | `26 / 26` passing |
| ✅ PostgreSQL Backend | Live-tested with Docker Compose |
| ✅ Feedback Export | JSON + CSV verified |
| ✅ Audit Prune/Log | Tested on both JSON and PostgreSQL paths |

---

## 🗺 Roadmap

```mermaid
timeline
    title GitWhisper — Development Phases

    section ✅ Complete
        Phase 1 : Context Intelligence
                : Commit & env capture
                : Command redaction & IDE context
        Phase 2 : AI Intelligence
                : Gemini cloud + Ollama local
                : Context optimizer & reasoning-chain prompts
        Phase 3 : Collaboration
                : Git notes integration
                : Slack, Discord, GitHub, GitLab
        Phase 4 : Advanced Analyzers
                : Quality, security, performance scoring
                : Bug prediction, knowledge risk, feedback
        Phase 5 : Enterprise Foundation
                : Docker + Compose stack
                : Auth, audit, JSON/Postgres DB layer

    section 🔮 Planned
        Phase 6 : Richer Analytics
                : Query language for history
                : Distributed worker support
        Phase 7 : Enterprise Scale
                : Full SSO / RBAC policy engine
                : Managed cloud deployment option
```

**Immediate next steps:**

- [ ] 🧪 Add CI workflows (build, test, fmt, clippy)
- [ ] 📄 Add `LICENSE` file before public release
- [ ] 📸 Add real dashboard screenshots / GIFs
- [ ] 🧪 Integration tests for PostgreSQL + Docker
- [ ] 🌳 tree-sitter language-aware parsing (function-level diff)
- [ ] 🔍 `gitwhisper search` — query history in natural language

---

## 🤝 Contributing

Good first contribution areas:

| Area | Why It Helps |
|---|---|
| 🧪 Analyzer tests | Makes risk reports trustworthy |
| 📚 Docs & examples | Helps users understand workflows faster |
| 🔗 Integration tests | Protects Slack / GitHub / GitLab / Postgres behavior |
| 📊 Dashboard polish | Makes team insights easier to scan |
| 🌳 Language parsers | Improves semantic diff quality |

**Recommended flow:**
```bash
git checkout -b feat/your-change
cargo fmt
cargo test
cargo clippy -- -D warnings
# Open a PR! 🎉
```

---

## ❓ FAQ

<details>
<summary><b>Does GitWhisper require cloud AI?</b></summary>

No. You can use local Ollama mode or rely entirely on non-AI heuristic analysis. Cloud AI is only called when you explicitly configure it and provide credentials.

</details>

<details>
<summary><b>Does it send code to Slack, GitHub, or other services by default?</b></summary>

No. All external integrations are `enabled = false` by default. Nothing leaves your machine until you explicitly configure and enable an integration.

</details>

<details>
<summary><b>Is PostgreSQL required?</b></summary>

No. JSON file storage is the default and works with zero setup. PostgreSQL is available for team or Docker-backed deployments.

</details>

<details>
<summary><b>Is GitWhisper production-ready for enterprise?</b></summary>

The enterprise foundation exists — Docker, auth module, audit module, feedback, and DB abstraction are all working. Full SSO, advanced RBAC policy enforcement, distributed workers, and managed cloud deployment are future roadmap items.

</details>

<details>
<summary><b>What's the difference between gitwhisper explain and gitwhisper summarize?</b></summary>

`explain` answers *"why did this specific file change?"* — it focuses on individual commits and the intent behind changes. `summarize` tells the file's entire *evolution story* in prose — ideal for onboarding or writing documentation.

</details>

---

## 📄 License

> ⚠️ A license file is not currently present in the repository root.  
> Add one before public release so contributors and users know how they can use GitWhisper.

---

<div align="center">

**Built with ❤️ and 🦀 Rust**

*If GitWhisper helped you understand your codebase better, give it a ⭐*

[![GitHub stars](https://img.shields.io/github/stars/SHREESHANTH99/GitWhisper?style=social)](https://github.com/SHREESHANTH99/GitWhisper/stargazers)

</div>