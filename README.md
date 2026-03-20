# crow

**Code Review Workflow Accelerator** — a CLI that wraps `gh` and `wt` to streamline GitHub PR review workflows.

## Overview

crow cuts the friction from day-to-day pull request work. It pulls your authored PRs and incoming review requests into a single view, checks out branches into isolated git worktrees, surfaces unresolved review threads, monitors CI checks, and (optionally) launches a Claude-powered code review session — all from one fast binary.

Under the hood crow delegates to two CLIs:

- **`gh`** — GitHub CLI for all API calls (PR listing, review threads, CI checks, posting reviews)
- **`wt`** — worktree manager for creating and removing per-PR git worktrees

## Prerequisites

| Tool | Purpose |
|------|---------|
| [`gh`](https://cli.github.com/) | GitHub API — required for all commands |
| [`wt`](https://github.com/ionrock/wt) | Worktree management — required for `checkout`, `review`, and `done` |
| [Claude Code](https://claude.ai/code) | AI review session — required for `review` and the Claude Code plugin |

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
| `make check` | Format, lint, test, and build |
| `make clean` | Remove build artifacts |

## Quick Start

The typical workflow for responding to review feedback:

```sh
# 1. See what needs attention
crow status

# 2. Check out a PR into its own worktree (shows unresolved threads automatically)
crow checkout 42

# 3. Read through the review threads
crow reviews

# 4. Make your changes, then push and reply to all open threads
crow push --reply "Fixed in latest commit"

# 5. Check CI
crow ci

# 6. Clean up when done
crow done --ready
```

For reviewing someone else's PR with Claude:

```sh
crow review 99
```

## Commands Reference

### `crow status`

Show PRs needing your attention — both PRs you authored and PRs where you have been requested as a reviewer.

```
crow status
```

Output is grouped into two sections: **Authored PRs** (with review decision status) and **Review Requested** (with author handles). Each row shows PR number, title, status, and relative timestamp.

---

### `crow checkout <pr>`

Check out a PR branch into an isolated git worktree via `wt`, then automatically display its unresolved review threads.

```
crow checkout <PR-NUMBER>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<pr>` | PR number to check out (required) |

**Example:**

```sh
crow checkout 42
```

After checkout, crow runs `crow reviews` automatically so you see the open feedback immediately.

---

### `crow reviews [pr] [--all] [--diff] [--unresolved]`

Show review threads for a PR, grouped by file and sorted by line number. Defaults to the PR associated with the current branch.

```
crow reviews [PR-NUMBER] [--all] [--diff] [--unresolved]
```

**Arguments and flags:**

| Flag / Arg | Default | Description |
|------------|---------|-------------|
| `[pr]` | current branch | PR number |
| `--all` | false | Include resolved threads |
| `--diff` | false | Show the diff hunk for each thread |
| `--unresolved` | true | Show only unresolved threads |

**Examples:**

```sh
# Unresolved threads on current branch
crow reviews

# All threads (including resolved) on PR #55
crow reviews 55 --all

# Unresolved threads with diff context
crow reviews --diff
```

---

### `crow ci [pr] [--watch]`

Show CI check status for a PR, grouped by workflow. Defaults to the PR associated with the current branch.

```
crow ci [PR-NUMBER] [--watch]
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--watch` | Hand off to `gh pr checks --watch` for live updates |

**Examples:**

```sh
# Snapshot of CI on current branch
crow ci

# Watch CI on PR #42 until checks complete
crow ci 42 --watch
```

Failed checks include a direct link to the run log.

---

### `crow push [--reply <msg>]`

Run `git push` on the current branch. With `--reply`, also batch-reply to every unresolved review thread with the given message.

```
crow push [--reply <MESSAGE>]
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--reply <msg>` | Reply to all unresolved threads with this message after pushing |

**Examples:**

```sh
# Plain push
crow push

# Push and close out all open threads with a note
crow push --reply "Addressed in this commit"
```

---

### `crow done [--ready]`

Clean up after finishing work on a PR. Removes the current worktree via `wt remove`. If the PR is yours and there are uncommitted pushes, they are pushed first. With `--ready`, marks the PR as ready for review before cleanup.

```
crow done [--ready]
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--ready` | Mark your own PR as ready for review before removing the worktree |

**Examples:**

```sh
# Clean up worktree
crow done

# Mark ready and clean up
crow done --ready
```

---

### `crow review <pr>`

Launch an interactive Claude Code review session for a PR. crow fetches the PR description, diff (up to 100 KB), and existing unresolved review threads, builds a structured prompt, then execs into the PR's worktree with `claude --dangerously-skip-permissions`.

```
crow review <PR-NUMBER>
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `<pr>` | PR number to review (required) |

**Example:**

```sh
crow review 99
```

Claude receives full PR context and can read files, run tests, and explore the codebase. The session replaces the current process (exec), so it returns directly to your shell when Claude exits.

Requires Claude Code to be installed.

---

### `crow comment <pr> [--event <type>] [body]`

Post a review on a PR — approve, request changes, or leave a comment.

```
crow comment <PR-NUMBER> [--event approve|request-changes|comment] [BODY]
```

**Arguments and flags:**

| Flag / Arg | Default | Description |
|------------|---------|-------------|
| `<pr>` | — | PR number (required) |
| `--event` | `comment` | Review type: `approve`, `request-changes`, or `comment` |
| `[body]` | opens `$EDITOR` | Review body text |

**Examples:**

```sh
# Approve with a note
crow comment 42 --event approve "Looks good to me"

# Request changes (body written in $EDITOR)
crow comment 42 --event request-changes

# Leave a plain comment
crow comment 42 "Minor nit: see inline comment"
```

When `body` is omitted, crow opens `$EDITOR` (falling back to `vi`) for you to write the review body.

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

## Claude Code Plugin

The plugin surfaces crow commands as `/crow:*` slash commands inside Claude Code, and ships a `pr-reviewer` agent for in-editor reviews.

### Install

```sh
crow install-plugin
# Then restart Claude Code
```

### Slash commands

| Command | Description |
|---------|-------------|
| `/crow:status` | Show PRs needing attention |
| `/crow:checkout <pr>` | Check out a PR into a worktree |
| `/crow:reviews [pr] [--all] [--diff]` | Show review threads (Claude summarizes them) |
| `/crow:ci [pr]` | Show CI status (Claude summarizes results) |
| `/crow:push [--reply <msg>]` | Push changes |
| `/crow:done [--ready]` | Clean up worktree |
| `/crow:comment <pr> [--event ...] [body]` | Post a review |

### `pr-reviewer` agent

The plugin also registers a `pr-reviewer` agent. When invoked, it reads the diff and surrounding code, runs the test suite if one exists, and produces a prioritized findings report organized by severity (P0 through P3). It can be used independently or as part of a `crow review` session.

### Uninstall

```sh
crow install-plugin --uninstall
# Then restart Claude Code
```

## Architecture

```
crow/
├── src/
│   ├── main.rs        Entry point — parses CLI and dispatches to cmd/*
│   ├── cli.rs         Clap command definitions (Command enum, ReviewEvent enum)
│   ├── types.rs       Serde types shared across modules (Pr, ReviewThread, CheckRun, …)
│   ├── gh.rs          Adapter for all gh CLI and GraphQL calls
│   ├── wt.rs          Adapter for wt worktree commands (checkout, exec, remove)
│   ├── display.rs     Terminal formatting (colors, section headers, thread display)
│   └── cmd/
│       ├── mod.rs
│       ├── status.rs
│       ├── checkout.rs
│       ├── reviews.rs
│       ├── ci.rs
│       ├── push.rs
│       ├── done.rs
│       ├── review.rs
│       ├── comment.rs
│       └── install_plugin.rs
└── plugin/
    ├── .claude-plugin/plugin.json   Plugin manifest
    ├── commands/                    Slash command definitions (*.md)
    ├── skills/review-pr/SKILL.md   Review skill definition
    └── agents/pr-reviewer.md       pr-reviewer agent definition
```

**Module responsibilities:**

- `gh.rs` — all external GitHub state: PR lists, review threads, CI checks, posting reviews, repo info. Uses `gh` CLI for REST and GraphQL calls.
- `wt.rs` — all worktree operations: creating worktrees (`wt switch pr:<n>`), exec-ing into them, and removing them.
- `display.rs` — terminal output only: color-coded PR rows, review thread trees, CI check rows.
- `types.rs` — plain Serde structs used as the data contract between `gh.rs` and `cmd/*`.
- `cmd/*` — one file per subcommand; thin orchestration layer that calls `gh`, `wt`, and `display`.

## Contributing

```sh
# Build
make build

# Run tests
make test

# Lint and format
make lint
make fmt

# All checks (fmt + lint + test + build)
make check
```

Please ensure `make check` passes before submitting a pull request.

## License

MIT
