# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs` starts the GTK-based image/PDF/archive viewer; supporting modules live in `src/app/`, `src/mapping/`, `src/operation/`, `src/controller/`, and `src/cherenkov/`.
- Rendering helpers and utilities sit under `src/gui.rs`, `src/image_*`, `src/cache.rs`, and `src/logger/`; constants live in `src/constant.rs`.
- `res/` carries runtime resources, and `test-files/` holds sample images/archives used by tests and manual checks.
- `build.rs`, `Cargo.toml`, and `Makefile` define build settings (feature flag `poppler_lock` controls poppler usage).

## Build, Test, and Development Commands
- `cargo build` or `make build-debug`: fast local debug build.
- `make release` (default) or `make release-without-lock`: optimized binary; the former enables the `poppler_lock` feature.
- `cargo run --release -- <path>`: launch the viewer against a file or directory.
- `cargo test` or `make test`: run Rust tests; ensure GTK deps are available if tests touch UI paths.
- `cargo fmt` (preferred over the legacy `make format` target): apply rustfmt using `rustfmt.toml`.

## Coding Style & Naming Conventions
- Rust 2018 edition; 4-space indentation; keep lines readable (~100 chars when possible).
- Modules and files use `snake_case`; types and traits use `CamelCase`; constants are `SCREAMING_SNAKE_CASE`.
- Favor clear error handling over unwraps in non-test code; prefer logging via `log`/`env_logger`.
- Group related helpers in module subdirs (e.g., new operations under `src/operation/` with a matching `mod` entry).

## Testing Guidelines
- Write Rust tests alongside modules under `mod tests { ... }` with `test_*` functions; use fixtures from `test-files/` when exercising image/archive flows.
- Add assertions around command parsing and rendering options to avoid regressions in key bindings and operations.
- For performance-sensitive work (e.g., cherenkov), document manual benchmarks using the provided `make benchmark-cherenkov` targets.

## Commit & Pull Request Guidelines
- Use short, imperative commit messages (`Add clippy fixes`, `Update poppler feature gate`); reference issues when relevant.
- PRs should summarize the change, list test commands executed, and note any new dependencies or feature flags.
- For user-facing/UI changes, attach screenshots or describe the interaction steps.
- Keep diffs focused; split refactors from behavior changes when practical.

## Security & Configuration Tips
- Ensure GTK and poppler development libraries are installed before building; the default release build expects `poppler_lock` support.
- Avoid embedding secrets; external services (e.g., Amazon Rekognition) should be configured via environment variables when used.
