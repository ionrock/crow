# crow

**AI-Powered Code Review CLI** — launch Claude-powered review sessions for GitHub pull requests.

crow reduces friction from pull request reviews by launching a Claude Code session with full PR context. It automatically detects whether you are the PR author or a reviewer and tailors the session accordingly:

- **As the PR author**: Claude helps you understand reviewer feedback, propose fixes, and run tests to verify your changes.
- **As a reviewer**: Claude helps you read the diff, understand the codebase, and compose actionable, specific feedback.

## Prerequisites

- [`gh`](https://cli.github.com/) — authenticated (`gh auth login`)
- [Claude Code](https://claude.ai/code) — installed and working
- [Rust / cargo](https://rustup.rs/) — only needed for installation

## Installation

### From GitHub (no checkout required)

```sh
cargo install --git https://github.com/ionrock/crow
```

### From source

```sh
git clone https://github.com/ionrock/crow
cd crow
make install
```

## Quick Start

### Review a PR

```sh
crow review 123
```

That's it. crow fetches the PR description, diff, and any unresolved review threads, detects your role (author or reviewer), and drops you into a Claude Code session with full context.

### Find PRs that need your attention

```sh
crow status
```

Shows PRs grouped into **Authored PRs** (with review decision status) and **Review Requested** (PRs where you've been asked to review). Each row shows the PR number, title, status, and relative timestamp.

### Typical workflow

```sh
# See what's on your plate
crow status

# Review a PR (yours or someone else's)
crow review 42
```

## How It Works

### Author flow

When you run `crow review <pr>` on a PR you authored, crow:

1. Fetches the PR description, diff, and all unresolved review threads
2. Builds a prompt listing each piece of reviewer feedback with file and line context
3. Launches Claude with instructions to help you address each issue, propose fixes, and verify changes with tests

### Reviewer flow

When you run `crow review <pr>` on a PR you did not author, crow:

1. Fetches the PR description, diff (up to 100 KB), and any existing review threads
2. Builds a prompt with the full diff and changed-files summary
3. Launches Claude with instructions to review for correctness, design, safety, and style — producing specific, file-and-line-anchored feedback

## Claude Code Plugin

crow also ships as a Claude Code plugin, surfacing `/crow:*` slash commands inside Claude Code sessions.

```sh
crow install-plugin        # install, then restart Claude Code
crow install-plugin --uninstall  # remove
```

| Command | Description |
|---------|-------------|
| `/crow:status` | Show PRs needing attention |
| `/crow:review <pr>` | Launch a Claude review session |

## Commands Reference

| Command | Description |
|---------|-------------|
| `crow review <pr>` | Launch an interactive Claude review session for a PR |
| `crow status` | Show PRs needing your attention |
| `crow install-plugin [--uninstall]` | Install or remove the Claude Code plugin |

---

## Development

### Building and testing

```sh
make build     # Debug build
make test      # Run the test suite
make check     # Format + lint + test + build
make coverage  # Line coverage report (requires cargo-llvm-cov)
```

### All Makefile targets

| Target | Description |
|--------|-------------|
| `make build` | Debug build |
| `make release` | Release build |
| `make install` | Release build + install to Cargo bin dir |
| `make install-plugin` | Install binary then install the Claude Code plugin |
| `make uninstall-plugin` | Uninstall the Claude Code plugin |
| `make test` | Run the test suite |
| `make coverage` | Run tests with line coverage report (requires `cargo-llvm-cov`) |
| `make check` | Format, lint, test, and build |
| `make clean` | Remove build artifacts |

### Architecture

```
crow/
├── src/
│   ├── main.rs            Entry point — parses CLI and dispatches to cmd/*
│   ├── cli.rs             Clap command definitions (Command enum)
│   ├── types.rs           Serde types shared across modules (Pr, ReviewThread, …)
│   ├── gh.rs              Adapter for all gh CLI and GraphQL calls
│   ├── display.rs         Terminal formatting (colors, section headers)
│   └── cmd/
│       ├── mod.rs
│       ├── status.rs
│       ├── review.rs
│       └── install_plugin.rs
└── plugin/
    ├── .claude-plugin/plugin.json   Plugin manifest
    └── commands/                    Slash command definitions (*.md)
```

### Contributing

Please ensure `make check` passes before submitting a pull request.

## License

MIT
