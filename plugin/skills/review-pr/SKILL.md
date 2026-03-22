---
name: review-pr
description: Review a GitHub PR — automatically detects whether you are the author (checking feedback on your PR) or a reviewer (performing a code review). Use when the user wants to review a PR, respond to review comments, or analyze PR changes.
argument-hint: <pr-number>
allowed-tools: "Bash"
---

# PR Review

Launch a review session for PR #$ARGUMENTS. This checks out the PR into a worktree, gathers full context (diff, review threads, CI status), and starts an interactive review session with role detection.

```bash
crow review $ARGUMENTS
```
