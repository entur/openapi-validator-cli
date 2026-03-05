# Lint-only validation

A minimal config for CI pipelines that only need to lint the spec — no code generation or compilation.

- **Generate/Compile:** Both disabled
- **Linter:** Spectral with `hint` severity — strictest setting, fails on any finding
- **Ruleset:** Entur API guidelines (can be replaced with any Spectral ruleset URL or local path)

This is useful for early-stage projects or repos that only publish the spec (e.g., API-first design).

## Usage

```bash
cp .oavc /path/to/your/project/.oavc
oav validate
```

## CI example

```yaml
- uses: entur/openapi-validator-cli/action/validate@v0
  with:
    spec: openapi/api.yaml
    skip-generate: "true"
    skip-compile: "true"
```
