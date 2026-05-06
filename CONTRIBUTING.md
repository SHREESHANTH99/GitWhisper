# Contributing To GitWhisper

Thanks for helping improve GitWhisper. This project is early, so the best contributions are focused, tested, and easy to review.

## Local Setup

```bash
git clone https://github.com/SHREESHANTH99/GitWhisper.git
cd GitWhisper
cargo build
cargo test
```

Optional local services:

```bash
docker compose up -d postgres
```

## Development Checks

Run these before opening a pull request:

```bash
cargo fmt --all -- --check
cargo test --all --locked
cargo clippy --all-targets --all-features -- -D warnings
```

On Windows GNU, place a 64-bit MSYS2 toolchain before older 32-bit MinGW installs:

```powershell
$env:PATH = "C:\msys64\ucrt64\bin;$env:PATH"
```

If `cargo clippy` flags existing code unrelated to your change, mention that in the PR.

## Pull Request Guidelines

- Keep changes focused on one behavior or one documentation area.
- Include CLI output or screenshots when changing user-facing behavior.
- Add tests when touching analyzers, config parsing, storage, auth, audit, or integrations.
- Do not commit `.env`, generated exports, local database files, or build logs.
- Update `README.md` and `CHANGELOG.md` when changing release-facing behavior.

## Good First Areas

- Analyzer test cases.
- Better examples in docs.
- PostgreSQL integration tests.
- Dashboard accessibility and layout polish.
- Language-aware parsing improvements.
