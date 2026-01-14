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

The formula lives at `Formula/oav.rb`. Update the version and sha256 values per release.

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
- Cargo: `cargo uninstall openapi-validator`
- Curl/manual: remove both binaries from your install dir (e.g. `rm /usr/local/bin/oav /usr/local/bin/openapi-validator`)

Both `oav` and `openapi-validator` are installed together and should be removed together.

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

The CLI provides two binaries: `openapi-validator` and `oav`.

## Requirements

- Docker (for linting, generation, and compile steps)

## Testing

Integration tests live under `tests/` and use fixtures from `tests/fixtures/`. Docker is required.

```bash
cargo test -- --ignored
```
