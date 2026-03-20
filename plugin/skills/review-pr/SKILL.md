---
name: review-pr
description: Review a GitHub PR — automatically detects whether you are the author (checking feedback on your PR) or a reviewer (performing a code review). Use when the user wants to review a PR, respond to review comments, or analyze PR changes.
argument-hint: <pr-number>
allowed-tools: "Read, Grep, Glob, Bash, Agent"
---

# PR Review

Review PR #$ARGUMENTS.

## Step 1: Detect Role

First, determine your role for this PR:

```bash
# Get PR details including author
gh pr view $ARGUMENTS --json author,number,title,state

# Get current git user
git config user.email
```

If you are the **author** of the PR, follow the Author flow below.
If you are a **reviewer**, follow the Reviewer flow below.

## Author Flow: Respond to Feedback

You authored this PR and want to understand reviewer feedback.

### Gather feedback

```bash
# All review comments
gh pr view $ARGUMENTS --json reviews,comments

# CI status
crow status
```

### Summarize what needs attention

Group feedback by type:
- **Must address**: Requested changes, blocking issues
- **Should consider**: Non-blocking suggestions
- **FYI**: Informational comments

For each item, show the file/line and what the reviewer said. Be concise — one line per issue.

Ask the user which items they want to address, then help them make the changes directly.

## Reviewer Flow: Code Review

You are reviewing someone else's PR.

### Step 1: Gather context

```bash
# PR details
gh pr view $ARGUMENTS

# Full diff
gh pr diff $ARGUMENTS
```

### Step 2: Check out the code

Check out the PR so you can read full file context:

```bash
gh pr checkout $ARGUMENTS
```

### Step 3: Review

For each changed file:

1. Read the full file for surrounding context
2. Check correctness: logic errors, edge cases, null handling
3. Check safety: error handling, resource cleanup, auth
4. Check design: abstractions, API surface, coupling
5. Check tests: new paths covered, edge cases, assertion quality

Run tests if a suite exists:
```bash
make test  # or cargo test, npm test, pytest, etc.
```

### Step 4: Give feedback

Keep feedback simple and direct. Three levels only:

- **Must Fix**: Bugs, security issues, broken behavior
- **Should Fix**: Missing error handling, test gaps
- **Nit**: Style, naming

For each finding: file + line, what is wrong, how to fix it. No padding. No praise unless the change is genuinely notable.

Post the review when ready:
```bash
gh pr review $ARGUMENTS --comment --body "<your review>"
# or to request changes:
gh pr review $ARGUMENTS --request-changes --body "<your review>"
# or to approve:
gh pr review $ARGUMENTS --approve
```
