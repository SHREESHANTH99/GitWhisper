# Changelog

All notable changes to GitWhisper will be documented in this file.

The format follows Keep a Changelog style, and this project uses semantic versioning.

## [0.1.0] - 2026-05-06

### Added

- Rust CLI named `gitwhisper`.
- Commit context capture with managed post-commit hook support.
- Semantic diff parser with file stats, symbol/import signals, and rough complexity movement.
- Intent detection for category, urgency, risk, scope, and conventional commit signals.
- Impact analysis with dependency graph hints.
- AI explanation pipeline with Gemini, Ollama, hybrid model selection, context optimization, and heuristic fallback.
- File timeline, explanation, summarization, and ownership commands.
- Quality, security, performance, bug prediction, knowledge risk, and refactor priority analyzers.
- Git notes, Slack, Discord, GitHub, and GitLab collaboration helpers.
- Dashboard, JSON/CSV analytics export, wiki generation, and ADR generation.
- Feedback capture, feedback export, audit log, and audit prune commands.
- JSON persistence backend and PostgreSQL persistence backend.
- Docker Compose stack with GitWhisper dashboard, Ollama, and PostgreSQL.
- MIT license, release notes, CI workflow, release workflow, security policy, and contribution guide.

### Validated

- Unit test suite passes with 26 tests.
- PostgreSQL feedback/audit path was live-tested through Docker Compose.
- JSON and CSV feedback export were tested.
- Audit prune/log flows were tested on JSON and PostgreSQL paths.

### Known Limitations

- Advanced analyzers are heuristic in this release.
- Full enterprise SSO, distributed workers, and managed cloud deployment are not included yet.
- External integrations require user-provided credentials and have not been exercised against every provider environment.
