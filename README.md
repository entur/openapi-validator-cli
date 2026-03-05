# OpenAPI Validator

Local CLI for linting, generating, and compiling OpenAPI specs. The tool keeps all output under `.oav/` in the repo and uses a simple config file (`.oavc`) for per-project settings.

## Quick Start

```bash
oav init --spec openapi/api.yaml
oav validate
```

`.oav/` is automatically added to `.gitignore` on first run.

## Install

### Homebrew (repo tap)

```bash
brew tap entur/openapi-validator-cli https://github.com/entur/openapi-validator-cli
brew install oav
```

The formula at `Formula/oav.rb` is updated automatically by the release workflow.

### Curl install

```bash
curl -fsSL https://raw.githubusercontent.com/entur/openapi-validator-cli/main/install.sh | bash
```

The installer requires `bash` (it uses bash arrays).

### Cargo install (Rust required)

```bash
cargo install --git https://github.com/entur/openapi-validator-cli
```

### Uninstall

- Homebrew: `brew uninstall oav`
- Cargo: `cargo uninstall oav`
- Curl/manual: `rm /usr/local/bin/oav` (or wherever you installed it)

## GitHub Action

Use oav directly in GitHub Actions workflows. The validate action requires Docker, so it must run on `ubuntu-latest` (or other Linux runners).

### Basic usage

```yaml
- uses: entur/openapi-validator-cli/action/setup@v0

- uses: entur/openapi-validator-cli/action/validate@v0
  with:
    spec: openapi/api.yaml
```

### Full example

```yaml
name: Validate OpenAPI
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Setup oav
        uses: entur/openapi-validator-cli/action/setup@v0

      - name: Validate OpenAPI spec
        id: oav
        uses: entur/openapi-validator-cli/action/validate@v0
        with:
          spec: openapi/api.yaml
          mode: both
          skip-compile: "true"
```

### Setup only

If you want to use oav commands directly:

```yaml
- uses: entur/openapi-validator-cli/action/setup@v0
  with:
    version: "0.3.0"

- run: |
    oav init --spec openapi/api.yaml
    oav validate --color never
```

### Inputs (validate action)

| Input               | Default   | Description                              |
|---------------------|-----------|------------------------------------------|
| `spec`              | —         | Path to OpenAPI spec file                |
| `mode`              | —         | Validation mode (server, client, both)   |
| `server-generators` | —         | Comma-separated server generators        |
| `client-generators` | —         | Comma-separated client generators        |
| `skip-lint`         | `false`   | Skip the lint step                       |
| `skip-generate`     | `false`   | Skip the generate step                   |
| `skip-compile`      | `false`   | Skip the compile step                    |
| `linter`            | —         | Linter to use (spectral, redocly, none)  |
| `ruleset`           | —         | Path to custom Spectral ruleset          |
| `docker-timeout`    | —         | Docker timeout in seconds                |
| `search-depth`      | —         | Max directory depth for spec search      |
| `working-directory` | `.`       | Working directory for oav commands       |
| `upload-reports`    | `true`    | Upload reports as workflow artifact      |

### Outputs

| Output   | Description                              |
|----------|------------------------------------------|
| `result` | `pass`, `fail`, or `error`               |
| `total`  | Total number of validation targets       |
| `passed` | Number of passed targets                 |
| `failed` | Number of failed targets                 |

## Commands

- `oav init` — scaffold `.oav/` and `.oavc`. Omit `--spec` to launch an interactive wizard
- `oav validate` — run lint → generate → compile and write reports
- `oav config [get|set|edit|print]` — manage `.oavc`
- `oav config validate` — check the current config for errors
- `oav config list-generators` — list all supported generators
- `oav config ignore` / `unignore` — add or remove `.oavc` from `.gitignore`
- `oav clean` — remove `.oav/`
- `oav clean --nuke` — remove `.oav/`, `.oavc`, and gitignore entries (prompts for confirmation)
- `oav agent install` / `uninstall` — install or remove the Claude Code skill for oav
- `oav completions install` / `uninstall` — manage shell completions

### Shell Completions

```bash
oav completions install              # auto-detect shell and install
oav completions install --shell zsh  # explicit shell
oav completions uninstall            # remove installed completions
oav completions generate bash        # print script to stdout
```

Homebrew users get completions automatically on `brew install`.

Supported shells for automatic install: **bash**, **zsh**, **fish**. For elvish and powershell, use `generate` and follow the printed instructions.

### Output Modes

- Default: step summaries plus per-generator progress for generate/compile
- `-v, --verbose`: stream full tool output
- `-q, --quiet`: minimal output (still prints final locations)

### Structured Output

Use `--output json` for machine-readable JSON output (for CI pipelines, scripts, etc.). Mutually exclusive with `-v` and `-q`.

```bash
oav validate --output json
```

### Parallel Execution

Use `-j` / `--jobs` to control parallelism during generate and compile steps:

```bash
oav validate -j4           # 4 parallel jobs
oav validate --jobs auto   # auto-detect (default, capped at 4)
```

Also configurable via the `jobs` key in `.oavc`.

### Gitignore Behavior

- `.oav/` is always gitignored.
- `.oavc` is committed by default.
- Use `oav init --ignore-config` or `oav config ignore` to ignore `.oavc`.

## Config File

`.oavc` lives in the repo root and controls defaults. Example:

```yaml
spec: openapi/api.yaml
mode: both
lint: true
generate: true
compile: true
server_generators:
  - aspnetcore
  - go-server
client_generators:
  - typescript-axios
generator_image: openapitools/openapi-generator-cli:v7.17.0
redocly_image: redocly/cli:1.25.5
linter: spectral
spectral_image: stoplight/spectral:6
spectral_ruleset: https://raw.githubusercontent.com/entur/api-guidelines/refs/tags/v2/.spectral.yml
spectral_fail_severity: error
manage_gitignore: true
docker_timeout: 300
search_depth: 4
jobs: auto
```

See [CONFIGURATION.md](CONFIGURATION.md) for the full reference and [examples/](examples/) for ready-to-use configs covering common setups.

## Generators

**Server:** `aspnetcore`, `go-server`, `kotlin-spring`, `python-fastapi`, `spring`, `typescript-nestjs`

**Client:** `csharp`, `go`, `java`, `kotlin`, `python`, `typescript-axios`, `typescript-fetch`, `typescript-node`

After `oav init`, generator configs are available in `.oav/generators/` for customization. See [CONFIGURATION.md](CONFIGURATION.md) for details. You can also skip the init call and just call `oav validate` directly, which also scaffolds a basic config for the CLI.

## Output Layout

- `.oav/generated/` — generated code
- `.oav/reports/` — logs and status
- `.oav/reports/dashboard.html` — HTML report summary

## Build

```bash
cargo build --release
```

## Requirements

- Docker (for linting, generation, and compile steps)

## Testing

Integration tests live under `tests/` and use fixtures from `tests/fixtures/`. Docker is required.

```bash
cargo test -- --ignored
```
