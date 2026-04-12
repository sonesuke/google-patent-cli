# AGENTS.md

This file contains instructions for AI coding agents working on this project.

## Project Overview

Google Patent CLI — A Rust-based command-line tool for searching and fetching patents from Google Patents.

## Rules

### Git

- Use **conventional commits** (e.g., `feat:`, `fix:`, `refactor:`, `chore:`). Commit messages are in **English**.
- **NEVER** use `git commit --no-verify`. The pre-commit hook exists to enforce quality. If it fails, fix the issue.
- Do not force-push to `main`.

### Code Quality

- Run `mise run pre-commit` before committing. This runs `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo test`.
- Follow existing patterns in the codebase.
- Make small, focused changes.

### Language

- Code comments, commit messages, **Pull Requests**, and **Artifacts** (Implementation Plans, Walkthroughs, Tasks): **English**
- Responses to the user: **日本語**

## Project Structure

```
src/                    # Rust source code
e2e/                    # E2E tests (CLI-level, using assert_cmd)
agents/
  pr-healer/            # PR-Healer autonomous agent
    healer.sh           # Host-side daemon loop
    prompt.txt          # Agent instructions
    tools/              # Agent tools
      load-progress.sh  # Read past context (JSONL)
      record-progress.sh # Write progress logs (JSONL)
claude-plugin/          # Claude Code Plugin structure
  skills/               # Individual skill definitions
scripts/                # Build and setup scripts (build.sh, up.sh, setup.sh)
flake.nix               # Nix flake for reproducible Docker image
mise.toml               # Task definitions (fmt, clippy, test, pre-commit)
```

## Skill-Bench Testing

`tests/` contains skill test cases using the `skill-bench` CLI:

- **Test cases are in English** - All `test_prompt` values in TOML files must be English
- **List tests**: `skill-bench list tests`
- **Run tests**: `skill-bench run tests`
- **Filter by skill**: `skill-bench run tests --skill patent-search`
- **Log directory**: `skill-bench run tests --log logs/`

## Tools

| Command | Description |
|---|---|
| `mise run fmt` | Check formatting with `cargo fmt` |
| `mise run clippy` | Lint with `cargo clippy` |
| `mise run test` | Run tests with `cargo test` |
| `mise run coverage` | Generate test coverage report |
| `mise run pre-commit` | Run all of the above |

## Development Container

The dev environment uses a Nix flake-based Docker image managed via mise tasks.

- **Build**: `mise run build` — Build the Docker image with Nix
- **Start**: `mise run up` — Start the dev container
- **Setup**: `mise run setup` — Configure git, Rust, Claude CLI, MCP tools, and skills inside the container
- **Attach**: `mise run attach` — Open a shell inside the running container
- **Stop**: `mise run down` — Stop and remove the container
