# Contributing to velog-cli

Thanks for your interest in contributing!

## Development Setup

```bash
git clone https://github.com/hamsurang/velog-cli.git
cd velog-cli
cargo build
```

## Before Submitting a PR

Please ensure:

1. **Format** — `cargo fmt-check` passes
2. **Lint** — `cargo lint` passes (clippy with `-D warnings`)
3. **Test** — `cargo test` passes
4. **Commit messages** — Use [Conventional Commits](https://www.conventionalcommits.org/) format

```bash
cargo fmt-check && cargo lint && cargo test
```

> **Tip:** Run `lefthook install` to have these checks run automatically on every commit.
> To skip hooks for WIP commits: `git commit --no-verify`

## Project Structure

```
src/
  cli.rs       — clap CLI definitions
  auth.rs      — Credential storage, JWT validation
  client.rs    — VelogClient (GraphQL HTTP client)
  handlers.rs  — Command handlers and display
  models.rs    — Domain types, GraphQL envelope
  main.rs      — Entry point
tests/
  cli_tests.rs — CLI integration tests
```

## Adding Features

- Follow existing patterns in the codebase
- Add unit tests for new logic
- Keep functions small and focused
- Use `anyhow` for error handling

## Reporting Issues

Use [GitHub Issues](https://github.com/hamsurang/velog-cli/issues) to report bugs or request features.
