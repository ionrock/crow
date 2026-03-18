---
name: review-pr
description: Review a GitHub PR with Claude — fetches PR context, diff, CI status, and existing comments, then performs a thorough code review. Use when the user wants to review a PR, look at a PR, or analyze PR changes.
argument-hint: <pr-number>
allowed-tools: "Read, Grep, Glob, Bash, Agent"
---

# PR Code Review

Review PR #$ARGUMENTS thoroughly.

## Step 1: Gather Context

Fetch all PR context in parallel:

```bash
# PR details
gh pr view $ARGUMENTS

# PR diff
gh pr diff $ARGUMENTS

# CI status
crow ci $ARGUMENTS

# Existing review comments
crow reviews $ARGUMENTS --all
```

## Step 2: Check Out the Code

Check out the PR into a worktree so you can read and explore the actual files:

```bash
crow checkout $ARGUMENTS
```

## Step 3: Review the Code

Now review the PR. For each changed file in the diff:

1. **Read the full file** to understand surrounding context, not just the diff
2. **Check correctness**: logic errors, edge cases, off-by-one errors, nil/null handling
3. **Check safety**: error handling, resource cleanup, injection risks, auth/authz
4. **Check design**: does the abstraction make sense, is the API surface right, coupling
5. **Check tests**: are new paths tested, are edge cases covered, do tests actually assert the right things
6. **Check style**: naming, idiomatic patterns for this language/framework, consistency with codebase

Run tests if a test suite exists:
```bash
make test  # or cargo test, npm test, pytest, etc.
```

## Step 4: Summarize Findings

Organize feedback by severity:

**Must Fix** — Bugs, security issues, data loss risks, broken behavior
**Should Fix** — Missing error handling, poor abstractions, test gaps
**Consider** — Style nits, minor improvements, alternative approaches

For each finding, include:
- The file and line number
- What the issue is
- A concrete suggestion for how to fix it

## Step 5: Discuss

The user may want to:
- Discuss specific findings
- Ask you to run additional tests or checks
- Make fixes directly (you can edit code in the worktree)
- Post the review via `/crow:comment`

Stay in the worktree and keep helping until the user is satisfied. When done, suggest running `/crow:done` to clean up.
