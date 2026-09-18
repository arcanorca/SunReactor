# Development

sunreactor keeps its engineering baseline intentionally small:

- `rustfmt` for formatting
- `clippy` for linting
- `cargo test` for correctness across both `--all-features` and `--no-default-features`
- one lean GitHub Actions workflow that runs the same checks as local development

The repo intentionally uses stock `rustfmt` behavior. There is no custom formatting profile unless a real project-wide need appears.

## Toolchain

The project pins Rust `1.97.1` via `rust-toolchain.toml`. When using `rustup`, the pinned toolchain and its components (`rustfmt`, `clippy`) are activated automatically in the repository:

```bash
rustup show
```

To install manually:

```bash
rustup toolchain install 1.97.1 --component rustfmt --component clippy
rustup default 1.97.1
```

## Local Workflow

Use the `Makefile` shortcuts or run the equivalent Cargo commands directly.

```bash
make fmt
make clippy
make test
make ci
```

Raw Cargo equivalents:

```bash
cargo fmt --all
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::dbg_macro -D clippy::todo
cargo test --workspace --all-targets --all-features
cargo test --workspace --no-default-features
```

## Lint Policy

The lint baseline is strict enough to catch obvious mistakes without turning the repo into a lint-tuning project:

- `-D warnings` keeps the codebase warning-free in CI.
- `-D clippy::dbg_macro` blocks committed debug prints via `dbg!`.
- `-D clippy::todo` blocks unfinished `todo!` placeholders.

The project does not currently deny broader Clippy groups such as `pedantic` or `nursery`. That keeps the signal-to-noise ratio high while the codebase is still stabilizing.

## CI Parity

CI runs the same gates as local development:

1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings -D clippy::dbg_macro -D clippy::todo`
3. `cargo test --workspace --all-targets --all-features`
4. `cargo test --workspace --no-default-features`

Before opening a PR, also run the lightweight binary smoke checks used during local verification:

```bash
cargo run --bin sunreactord -- --help
cargo run --bin sunreactorctl -- --help
```

`discover` and hardware-facing daemon runs are intentionally not part of CI because they depend on host tools and real device access.
