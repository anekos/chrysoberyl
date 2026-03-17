# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is Chrysoberyl

A GTK-based controllable image/PDF/archive viewer for Linux. Supports key/mouse mapping, multi-cell views, remote URLs, archives, cherenkov effects, and shell-scriptable operation commands via stdin/fifo/socket.

## Build & Development Commands

```bash
cargo build                              # Debug build
make release                             # Release build with poppler_lock feature
make release-without-lock                # Release build without poppler_lock
cargo test                               # Run tests
cargo clippy -- -D warnings              # Lint (must pass — enforced by pre-commit hook)
cargo test && cargo clippy -- -D warnings # Full pre-commit check
```

**Build dependencies:** GTK3, poppler-glib (detected via pkg-config), libarchive. The `build.rs` probes for `poppler-glib` and generates vergen build constants.

**Rust toolchain:** Pinned to `1.78` via `rust-toolchain` file.

## Pre-commit Hook

cargo-husky runs `cargo test` then `cargo clippy -- -D warnings` on every commit. Both must pass.

## Architecture

### Main Loop (`src/chrysoberyl.rs`)
Custom GTK event loop: parses CLI args into an `App`, then polls two channels (primary/secondary) for `Operation` values, dispatching them via `app.operate()`. GTK events are interleaved. Idle events fire after a configurable delay.

### Core Types
- **`Operation`** (`src/operation/mod.rs`): Large enum representing every user command (@next, @push, @cherenkov, @shell, etc.). Parsed from strings in `src/operation/parser.rs`.
- **`App`** (`src/app/`): Central application state. Owns entries, GUI, options, mappings, and handles operation dispatch.
- **`AppError`/`AppResult`** (`src/errors.rs`): Error types using the `failure` crate with a custom `AppError` enum.

### Module Groups
- **Entry management:** `src/entry/` (entries, filtering with expression parser, image metadata)
- **Input/control:** `src/controller/` (stdin, fifo, file, unix socket), `src/mapping/` (key/mouse/event/region mappings)
- **Rendering:** `src/gui.rs`, `src/image.rs`, `src/image_cache.rs`, `src/cherenkov/` (visual effects)
- **Options:** `src/option/` (viewer options with get/set/toggle/cycle semantics)
- **External integration:** `src/poppler/` (PDF via FFI), `src/archive.rs` (libarchive), `src/remote_cache/` (curl-based HTTP)

### Macro Modules
Several modules are `#[macro_use]` and must appear first in `src/main.rs`: `macro_utils`, `logger`, `errors`, `error_channel`, `from_macro`, `gtk_utils`, `util`.

## Coding Conventions

- Rust 2018 edition, 4-space indent
- Error handling via `failure` crate (`AppError` / `AppResult`)
- Tests live in `mod tests` blocks within source files; test fixtures in `test-files/`
- Operation commands are `@`-prefixed strings (e.g., `@next`, `@push`, `@shell`)
