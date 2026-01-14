# OpenAPI Validator

Local CLI for linting, generating, and compiling OpenAPI specs. The tool keeps all output under `.oav/` in the repo and uses a simple config file (`.oavc`) for per-project settings.

## Quick Start

```bash
oav init --spec openapi/api.yaml
oav validate
```

`.oav/` is automatically added to `.gitignore` on first run.

## Commands

- `oav init` — create `.oav/`, scaffold `.oavc`, and add gitignore entries
- `oav validate` — run lint → generate → compile and write reports
- `oav config [get|set|edit|print]` — manage `.oavc`
- `oav config ignore` — add `.oavc` to `.gitignore`
- `oav config unignore` — remove `.oavc` from `.gitignore`
- `oav clean` — remove `.oav/`

### Output Modes

- Default: step summaries plus per-generator progress for generate/compile
- `-v, --verbose`: stream full tool output
- `-q, --quiet`: minimal output (still prints final locations)

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
```

## Output Layout

- `.oav/generated/` — generated code
- `.oav/reports/` — logs and status
- `.oav/reports/dashboard.html` — HTML report summary

## Build

```bash
cargo build --release
```

The CLI provides two binaries: `openapi-validator` and `oav`.

## Requirements

- Docker (for linting, generation, and compile steps)

## Testing

Integration tests live under `tests/` and use fixtures from `test/`. Docker is required.

```bash
cargo test -- --ignored
```
