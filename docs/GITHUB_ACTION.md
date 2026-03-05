# GitHub Action

Use `oav` directly in GitHub Actions workflows. The validate action requires Docker, so it must run on `ubuntu-latest` (or other Linux runners).

## Basic usage

```yaml
- uses: entur/openapi-validator-cli/action/setup@v0

- uses: entur/openapi-validator-cli/action/validate@v0
  with:
    spec: openapi/api.yaml
```

## Full example

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

## Setup only

If you want to use oav commands directly:

```yaml
- uses: entur/openapi-validator-cli/action/setup@v0
  with:
    version: "0.3.0"

- run: |
    oav init --spec openapi/api.yaml
    oav validate --color never
```

## Inputs (validate action)

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

## Outputs

| Output   | Description                              |
|----------|------------------------------------------|
| `result` | `pass`, `fail`, or `error`               |
| `total`  | Total number of validation targets       |
| `passed` | Number of passed targets                 |
| `failed` | Number of failed targets                 |
