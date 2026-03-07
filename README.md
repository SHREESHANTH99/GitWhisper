# Git-insight 🔍
### AI-Powered Git Commit Intelligence for Developers

Git-insight is a **Rust-based developer tool** that explains the evolution of your codebase using AI.

It captures developer context during commits and combines it with Git history to generate **human-readable explanations of why files changed**, not just what changed.

---

# 🚀 Motivation

Git shows **what changed**, but developers often struggle to understand:

- Why the change happened
- What problem it solved
- The intent behind the commit
- The evolution of a file over time

Git-insight bridges that gap by combining:

- Git commit history
- developer commands
- environment context
- AI reasoning

This allows developers to quickly understand the history and purpose of any file.

---

# ✨ Features

## AI File Explanation

Explain why a file changed using AI.

```
Git-insight explain auth.js
```

Example output:

```
AI Explanation:
The authentication module was updated to introduce JWT token validation.
Earlier commits added login functionality but lacked proper token
expiration checks. The latest change fixes this issue and improves
authentication security.
```

---

## Commit Context Capture

Git-insight captures developer activity during commits including:

- commands executed
- environment metadata
- timestamps

Example stored metadata:

```
.git/Git-insight/f6e3058.json
```

Example structure:

```
{
  "commit": "f6e3058",
  "timestamp": "2026-03-07T12:14:26",
  "commands": ["npm test", "git add auth.js"],
  "environment": "Node 20.3, Windows 10"
}
```

---

## Explanation Caching

To avoid repeated AI calls, Git-insight stores explanations locally.

Cache location:

```
.git/Git-insight/cache/
```

Benefits:

- faster responses
- reduced API usage
- improved CLI performance

---

## Timeline Viewer

Git-insight can show the **timeline of changes for a file**.

```
Git-insight timeline auth.js
```

Example:

```
auth.js timeline:

f6e3058  Fix JWT validation
a3b2819  Add authentication middleware
918c92f  Initial login system
```

---

# 🧠 How It Works

Git-insight collects context during commits and later uses it to explain changes.

Pipeline:

```
User Command
     │
     ▼
Load Commit Metadata
     │
     ▼
Build File History
     │
     ▼
Generate AI Prompt
     │
     ▼
Call AI Model (Gemini)
     │
     ▼
Return Explanation
```

---

# 📦 Installation

Clone the repository:

```
git clone https://github.com/YOUR_USERNAME/Git-insight.git
```

Enter the project directory:

```
cd Git-insight
```

Build the project:

```
cargo build --release
```

Run the CLI:

```
cargo run -- explain auth.js
```

---

# 🔑 Gemini API Setup

Git-insight uses **Google Gemini API** for AI explanations.

Create an API key:

https://aistudio.google.com/app/apikey

Set environment variable.

### Windows

```
setx GEMINI_API_KEY "YOUR_API_KEY"
```

### Linux / macOS

```
export GEMINI_API_KEY="YOUR_API_KEY"
```

---

# 📁 Project Structure

```
Git-insight
│
├── src
│   ├── collectors
│   │   ├── commands.rs
│   │   ├── env.rs
│   │   └── tests.rs
│   │
│   ├── storage
│   │   ├── context.rs
│   │   ├── load.rs
│   │   └── save.rs
│   │
│   ├── viewer
│   │   ├── explain.rs
│   │   ├── log.rs
│   │   ├── replay.rs
│   │   └── timeline.rs
│   │
│   ├── capture.rs
│   ├── cli.rs
│   ├── git.rs
│   ├── hooks.rs
│   ├── main.rs
│   └── storage.rs
│
├── Cargo.toml
├── Cargo.lock
└── README.md
```

---

# ⚡ Example Commands

Explain a file:

```
Git-insight explain auth.js
```

Show commit timeline:

```
Git-insight timeline auth.js
```

Replay commit activity:

```
Git-insight replay
```

View commit logs:

```
Git-insight log
```

---

# 🛠 Roadmap

Planned improvements:

- AI change intent detection (bug / feature / refactor)
- GitHub PR summarization
- commit impact analysis
- code evolution visualization
- VSCode extension
- web dashboard for repository insights

---

# 🤝 Contributing

Contributions are welcome.

Steps:

1. Fork the repository
2. Create a feature branch
3. Commit changes
4. Submit a pull request

---

# ⭐ Support

If you find Git-insight useful:

- ⭐ Star the repository
- 🐛 Report bugs
- 💡 Suggest new features

---

# 📜 License

MIT License

---

# 👨‍💻 Author

Built with Rust for developers who want **better insights into their Git history**.
