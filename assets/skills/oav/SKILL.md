---
name: oav
description: Validate, lint, and fix OpenAPI specs using the oav CLI. Use when the user works with OpenAPI specs, mentions API validation, linting, code generation errors, compile failures, .oavc config, or asks to check, fix, or set up validation for their API.
argument-hint: "[validate | init | question]"
---

oav validates OpenAPI specs through three phases:

1. **Lint** — Spectral or Redocly rules against the spec
2. **Generate** — Docker-based code generation (server stubs, client SDKs)
3. **Compile** — compiles generated code to catch type errors and broken refs

**Docker must be running** for generate and compile phases. If you see "Docker is not available", the user needs to start Docker Desktop or the Docker daemon.

Config lives in `.oavc`. Use `oav --help` or `oav <command> --help` for full flag documentation.

## Commands

```bash
oav validate --output json     # Validation with machine-readable JSON output
oav validate                   # Validation with human-readable terminal output
oav init                       # Interactive setup wizard
oav config print               # Show current config
oav config list-generators     # List available code generators
oav config set <key> <value>   # Update a config value
oav clean                      # Remove generated artifacts
```

## Workflows

### Validate and diagnose

1. Run `oav validate --output json` and parse the JSON output (schema in [reference.md](reference.md))
2. Present a summary: phase | target | scope | status
3. For failures, extract the relevant error lines from the `log` field — trim timestamps, progress bars, and boilerplate
4. Identify root cause and suggest a targeted fix (see error examples in [reference.md](reference.md))

If the user just wants a quick check, `oav validate` (without `--output json`) prints a human-readable summary directly.

### Fix spec issues

1. Run `oav validate --output json` to identify failures
2. If `summary.failed == 0`, report success and stop
3. For each failure: read the relevant section of the spec file, identify the exact error from the log, apply a minimal fix, explain the change
4. Re-run `oav validate --output json` to confirm the fix works
5. Only edit the spec file — not `.oavc` or generator configs. Prefer fixing the API definition over suppressing lint rules. Flag breaking changes before applying.

### Set up oav

`oav init` has an interactive wizard that walks through spec discovery, mode selection, generators, and linter choice. If the user already knows what they want, pass flags directly:

```bash
oav init --spec <path> --mode <mode> --server-generators <list> --client-generators <list>
```

See [reference.md](reference.md) for the full JSON output schema, config fields, generators, and common error patterns.
