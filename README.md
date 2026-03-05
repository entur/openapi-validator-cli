# OpenAPI Validator CLI

[![GitHub Release](https://img.shields.io/github/v/release/entur/openapi-validator-cli?style=flat-square&label=release)](https://github.com/entur/openapi-validator-cli/releases/latest)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Homebrew](https://img.shields.io/github/v/release/entur/openapi-validator-cli?style=flat-square&label=homebrew&color=fbb040)](https://github.com/entur/openapi-validator-cli#homebrew)
[![License](https://img.shields.io/github/license/entur/openapi-validator-cli?style=flat-square)](LICENSE.md)
[![Issues](https://img.shields.io/github/issues/entur/openapi-validator-cli?style=flat-square)](https://github.com/entur/openapi-validator-cli/issues)
[![Pull Requests](https://img.shields.io/github/issues-pr/entur/openapi-validator-cli?style=flat-square)](https://github.com/entur/openapi-validator-cli/pulls)
[![Last Commit](https://img.shields.io/github/last-commit/entur/openapi-validator-cli?style=flat-square)](https://github.com/entur/openapi-validator-cli/commits/main)

Local CLI for linting, generating, and compiling OpenAPI specs. Runs everything through Docker, keeps output under `.oav/`, and uses a simple `.oavc` config file per project.

## Quick start

```bash
oav init --spec openapi/api.yaml
oav validate
```

## Install

### Homebrew

```bash
brew tap entur/openapi-validator-cli https://github.com/entur/openapi-validator-cli
brew install oav
```

### Shell script

```bash
curl -fsSL https://raw.githubusercontent.com/entur/openapi-validator-cli/main/install.sh | bash
```

### Cargo

```bash
cargo install --git https://github.com/entur/openapi-validator-cli
```

### Uninstall

| Method   | Command                       |
|----------|-------------------------------|
| Homebrew | `brew uninstall oav`          |
| Cargo    | `cargo uninstall oav`         |
| Manual   | `rm /usr/local/bin/oav`       |

## Commands

| Command | Description |
|---------|-------------|
| `oav init` | Scaffold `.oav/` and `.oavc` (omit `--spec` for interactive wizard) |
| `oav validate` | Run lint, generate, compile pipeline and write reports |
| `oav config [get\|set\|edit\|print]` | Manage `.oavc` |
| `oav config validate` | Check config for errors |
| `oav config list-generators` | List supported generators |
| `oav config ignore` / `unignore` | Toggle `.oavc` in `.gitignore` |
| `oav clean` | Remove `.oav/` |
| `oav clean --nuke` | Remove `.oav/`, `.oavc`, and gitignore entries |
| `oav agent install` / `uninstall` | Manage the Claude Code skill |
| `oav completions install` / `uninstall` | Manage shell completions |

### Output modes

| Flag | Behavior |
|------|----------|
| *(default)* | Step summaries + per-generator progress |
| `-v, --verbose` | Stream full tool output |
| `-q, --quiet` | Minimal output |
| `--output json` | Machine-readable JSON (for CI/scripts) |

### Parallelism

```bash
oav validate -j4           # 4 parallel jobs
oav validate --jobs auto   # auto-detect (default, capped at 4)
```

Also configurable via the `jobs` key in `.oavc`.

## Generators

| Type | Generators |
|------|------------|
| **Server** | `aspnetcore`, `go-server`, `kotlin-spring`, `python-fastapi`, `spring`, `typescript-nestjs` |
| **Client** | `csharp`, `go`, `java`, `kotlin`, `python`, `typescript-axios`, `typescript-fetch`, `typescript-node` |

Generator configs are available in `.oav/generators/` after init.

## Config

`.oavc` lives in the repo root. Minimal example:

```yaml
spec: openapi/api.yaml
mode: both
linter: spectral
```

See [CONFIGURATION.md](CONFIGURATION.md) for the full reference and [examples/](examples/) for ready-to-use configs.

## GitHub Action

Available as a reusable action for CI workflows. See the [GitHub Action docs](docs/GITHUB_ACTION.md) for setup, inputs, and outputs.

```yaml
- uses: entur/openapi-validator-cli/action/setup@v0
- uses: entur/openapi-validator-cli/action/validate@v0
  with:
    spec: openapi/api.yaml
```

## Output layout

```
.oav/
  generated/          # generated code
  reports/            # logs and status
  reports/dashboard.html  # HTML report summary
```

## Requirements

- Docker (for linting, generation, and compile steps)

## Build

```bash
cargo build --release
```

## Testing

Integration tests live under `tests/` and require Docker.

```bash
cargo test -- --ignored
```
