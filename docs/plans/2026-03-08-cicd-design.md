# CI/CD Design

**Goal:** GitHub ActionsでCI パイプラインを構築し、コード品質を自動チェックする

**Platform:** GitHub Actions

## Triggers

- `push` to `main`
- `pull_request` to `main`

## Jobs (4 parallel)

| Job | Description | Commands |
|-----|-------------|----------|
| `format` | Rust + TOML format check | `cargo fmt --check`, `taplo check` |
| `lint` | Clippy pedantic lint | `cargo clippy --all-targets --all-features -- -D warnings` |
| `test` | Unit tests | `cargo nextest run` |
| `deny` | License & vulnerability check | `cargo deny check` |

## Common Settings

- **OS:** `ubuntu-latest`
- **Rust toolchain:** `stable`
- **Cache:** `Swatinem/rust-cache@v2` with shared-key per job
- **Additional tools:** taplo, cargo-nextest, cargo-deny (installed per job as needed)

## Files

- `.github/workflows/ci.yml`
