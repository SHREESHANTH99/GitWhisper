# Security Policy

## Supported Versions

| Version | Supported |
| --- | --- |
| `0.1.x` | Yes |

## Reporting A Vulnerability

Please do not open a public GitHub issue for suspected vulnerabilities, leaked credentials, or exploitable behavior.

Use GitHub private vulnerability reporting if it is enabled for the repository. If it is not enabled yet, contact the maintainer through the repository owner profile and include:

- A short description of the issue.
- Steps to reproduce.
- Impact and affected versions.
- Any logs or screenshots that do not expose secrets.

## Security Notes For Users

- Cloud AI and external integrations are opt-in through `.gitwhisper.toml`.
- `.env` is ignored by Git and should never be committed.
- Command capture includes redaction heuristics, but users should still avoid committing secrets in commands, config files, or logs.
- Review `privacy.offline_mode`, `privacy.exclude_files`, and integration settings before using GitWhisper on private repositories.
