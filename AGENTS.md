# Repository Guidelines

## Project Structure & Module Organization
- Source: `src/main.rs` (binary crate entry). Add modules under `src/` (e.g., `src/net.rs`).
- Binaries: For multiple tools, use `src/bin/<name>.rs`.
- Tests: Co-locate unit tests with modules using `#[cfg(test)]`; add integration tests in `tests/`.
- Docs & examples: Place runnable examples in `examples/` when useful.

## Build, Test, and Development Commands
- Build: `cargo build` (debug) or `cargo build --release` (optimized).
- Run: `cargo run -- <args>` (passes CLI args to the binary).
- Test: `cargo test` (runs unit and integration tests).
- Lint: `cargo clippy -- -D warnings` (treat lints as errors).
- Format: `cargo fmt --all` (applies `rustfmt`).
- Docs: `cargo doc --no-deps --open` (local API docs).

## Coding Style & Naming Conventions
- Formatting: Use `rustfmt` defaults; run `cargo fmt --all` before committing.
- Linting: Keep `clippy` clean; prefer fixes over `allow`.
- Naming: `snake_case` for functions/files, `CamelCase` for types/traits, `SCREAMING_SNAKE_CASE` for consts.
- Errors: Prefer `Result<T, E>` returns; avoid `unwrap()`/`expect()` outside tests and `main`.
- Platform guards: Use `#[cfg(windows)]` for Windows-specific code paths; add alternatives or clear errors for other OSes.

## Testing Guidelines
- Unit tests: In the same file as the code under `#[cfg(test)]` modules.
- Integration tests: One file per feature in `tests/` (e.g., `tests/cli.rs`).
- Naming: `function_under_test_condition_expected()`.
- CLI behavior: Prefer golden tests or snapshot assertions for help/version output.
- Run locally: `cargo test` and ensure no flakiness or network dependence.

## Commit & Pull Request Guidelines
- Commits: Short, imperative subject (<= 72 chars), body explains what/why.
- Scope: Prefer focused commits/PRs per feature or fix.
- PRs: Include description, rationale, and manual test notes; link issues (e.g., `Closes #12`).
- Checks: Ensure `cargo fmt`, `cargo clippy -D warnings`, and `cargo test` pass.
- Docs: Update `README.md` for user-visible changes and usage examples.

## Security & Configuration Tips
- Input handling: Treat CLI args as untrusted; validate and sanitize.
- Privileges: Avoid requiring elevated permissions; document when unavoidable.
- Logging: Prefer structured, non-verbose logs by default; no sensitive data.
- Dependencies: Run `cargo update -p <crate>` intentionally; prefer minimal, well-maintained crates.

