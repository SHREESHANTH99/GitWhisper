# Release Process

This document describes how to publish a GitWhisper GitHub release.

## Preflight Checklist

1. Confirm the version in `Cargo.toml`.
2. Confirm `CHANGELOG.md` has an entry for the version.
3. Confirm `docs/releases/vX.Y.Z.md` exists.
4. Run local validation:

```bash
cargo fmt --all -- --check
cargo test --all --locked
cargo clippy --all-targets --all-features -- -D warnings
cargo package --list
```

On Windows GNU toolchains, make sure a 64-bit GCC/MSYS2 toolchain is first in `PATH`. For example:

```powershell
$env:PATH = "C:\msys64\ucrt64\bin;$env:PATH"
```

5. Confirm no local artifacts are staged:

```bash
git status --short
```

6. Commit the release prep changes:

```bash
git add .
git commit -m "chore: prepare v0.1.0 release"
```

7. Create and push the release tag:

```bash
git tag -a v0.1.0 -m "GitWhisper v0.1.0"
git push origin main
git push origin v0.1.0
```

If `v0.1.0` already exists locally from an earlier dry run, recreate it after the release prep commit:

```bash
git tag -d v0.1.0
git tag -a v0.1.0 -m "GitWhisper v0.1.0"
```

The `release.yml` workflow builds release binaries and creates the GitHub release for `v*` tags.

## Manual GitHub Release Fields

If creating the release manually:

- Tag: `v0.1.0`
- Target: `main`
- Title: `GitWhisper v0.1.0`
- Description: use `docs/releases/v0.1.0.md`
- Mark as latest release: yes
- Pre-release: no

## Post-Release

1. Confirm release assets uploaded.
2. Confirm README badges resolve.
3. Confirm the release notes render correctly.
4. Open a new issue for the next milestone.
