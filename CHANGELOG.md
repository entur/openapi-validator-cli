# Changelog

All notable changes to OpenAPI Validator will be documented in this file.

## v0.5.0

- Add JSON output (`--output json`) for machine-readable results
- Add parallel generation and compilation (`-j` / `--jobs`)
- Add interactive init wizard when `--spec` is omitted
- Add `oav clean --nuke` to remove all oav artifacts
- Add `oav completions install` / `uninstall` for shell completion management
- Add `oav agent install` / `uninstall` for Claude Code skill integration
- Add `oav config validate` and `oav config list-generators` subcommands
- Add configurable docker timeout (`docker_timeout`) and search depth (`search_depth`)

## v0.4.0

- Add reusable GitHub Actions for setup and validation
- Add test coverage, exit codes, and config validation
- Add docker timeouts and configurable search depth
- Centralize generator definitions
- Add Spectral linting as default linter (replacing Redocly)
- Add input validation for `config set` and CLI args
- Improve terminal output handling and respect `-q`/`-v` flags

## v0.3.0

- Add Homebrew formula and curl installer
- Add release workflow with binary builds

## v0.2.0

- Remove `openapi-validator` binary; `oav` is now the sole entrypoint

## v0.1.0

- Initial Rust CLI: lint, generate, compile pipeline with Docker
- `.oavc` config file and `.oav/` output directory
