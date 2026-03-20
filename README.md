# crow

**AI-Powered Code Review CLI** — launches Claude-powered review sessions for GitHub pull requests.

## Overview

crow reduces friction from pull request review work by launching a Claude Code session with full PR context. It automatically detects whether you are the PR author or a reviewer and tailors the session accordingly:

- **As the PR author**: Claude helps you understand reviewer feedback, propose fixes, and run tests to verify your changes.
- **As a reviewer**: Claude helps you read the diff, understand the codebase, and compose actionable, specific feedback.

Under the hood crow delegates to `gh` for all GitHub API calls.

## Prerequisites

| Tool | Purpose |
|------|---------|
| [`gh`](https://cli.github.com/) | GitHub API — required for all commands |
| [Claude Code](https://claude.ai/code) | AI review session — required for `review` |
| [Rust / cargo](https://rustup.rs/) | Build toolchain — required to install from source |

Authenticate with GitHub before using crow:

```sh
gh auth login
```

## Installation

### From source

```sh
git clone https://github.com/ionrock/crow
cd crow
make install
```

`make install` runs `cargo build --release` then `cargo install --path .`, placing `crow` on your `$PATH`.

### Makefile targets

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

## Quick Start

### As the PR author — addressing review feedback

```sh
# 1. See what needs your attention
crow status

# 2. Launch a Claude session to help address feedback
crow review 42
```

Claude receives the unresolved review threads and the PR diff. It helps you work through each piece of feedback, propose fixes, and run tests.

### As a reviewer — providing actionable feedback

```sh
# 1. See which PRs are waiting for your review
crow status

# 2. Launch a Claude session to help review the code
crow review 99
```

Claude receives the full PR diff and context. It reads files, runs tests, and helps you compose specific, actionable feedback.

## Commands Reference

### `crow status`

Show PRs needing your attention — both PRs you authored and PRs where you have been requested as a reviewer.

```
crow status
```

Output is grouped into two sections: **Authored PRs** (with review decision status) and **Review Requested** (with author handles). Each row shows PR number, title, status, and relative timestamp.

---

### `crow review <pr>`

Launch an interactive Claude Code review session for a PR. crow auto-detects whether you are the PR author or a reviewer and builds the appropriate prompt.

```
crow review <PR-NUMBER>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<pr>` | PR number (required) |

**Example:**

```sh
crow review 42
```

crow fetches the PR description, diff, and unresolved review threads, then execs into a Claude session with `--dangerously-skip-permissions`. Claude can read any file, run tests, and explore the codebase. The session replaces the current process, so control returns to your shell when Claude exits.

Requires Claude Code to be installed.

---

### `crow install-plugin [--uninstall]`

Install or uninstall the crow Claude Code plugin. The plugin ships as embedded files compiled into the `crow` binary and writes them to `~/.claude/plugins/cache/local/crow/<version>/`.

```
crow install-plugin [--uninstall]
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--uninstall` | Remove the plugin instead of installing |

**Examples:**

```sh
crow install-plugin
crow install-plugin --uninstall
```

Also available as Makefile targets: `make install-plugin` and `make uninstall-plugin`.

## How It Works

### Author flow

When you run `crow review <pr>` on a PR you authored, crow:

1. Fetches the PR description, diff, and all unresolved review threads
2. Builds a prompt that lists each piece of reviewer feedback with file and line context
3. Launches Claude with instructions to help you address each issue, propose fixes, and verify changes with tests

### Reviewer flow

When you run `crow review <pr>` on a PR you did not author, crow:

1. Fetches the PR description, diff (up to 100 KB), and any existing review threads
2. Builds a prompt that includes the full diff and changed-files summary
3. Launches Claude with instructions to review for correctness, design, safety, and style — producing specific, file-and-line-anchored feedback

## Claude Code Plugin

The plugin surfaces crow as `/crow:*` slash commands inside Claude Code.

### Install

```sh
crow install-plugin
# Then restart Claude Code
```

### Slash commands

| Command | Description |
|---------|-------------|
| `/crow:status` | Show PRs needing attention |
| `/crow:review <pr>` | Launch a Claude review session |

### Uninstall

```sh
crow install-plugin --uninstall
# Then restart Claude Code
```

## Architecture

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

**Module responsibilities:**

- `gh.rs` — all external GitHub state: PR lists, review threads, diff, repo info. Uses `gh` CLI for REST and GraphQL calls.
- `display.rs` — terminal output only: color-coded PR rows, review thread trees.
- `types.rs` — plain Serde structs used as the data contract between `gh.rs` and `cmd/*`.
- `cmd/*` — one file per subcommand; thin orchestration layer that calls `gh` and `display`.

## Contributing

```sh
# Build
make build

# Run tests
make test

# Check line coverage (requires cargo-llvm-cov: cargo install cargo-llvm-cov)
make coverage

# Lint and format
make lint
make fmt

# All checks (fmt + lint + test + build)
make check
```

Please ensure `make check` passes before submitting a pull request.

### Coverage

Install the coverage tool once:

```sh
cargo install cargo-llvm-cov
```

Then run:

```sh
make coverage
```

This prints a line-by-line coverage report. The following modules are excluded from coverage targets because they require live external processes (`gh`, `git`) or are the binary entrypoint:

- `gh.rs` — wraps the `gh` CLI; requires a real GitHub token
- `main.rs` — process entrypoint

## License

MIT
