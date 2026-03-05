# Custom linter (Redocly)

A project that uses Redocly CLI instead of Spectral for linting.

- **Linter:** `redocly` with a pinned image version (`1.30.2`)
- **Compile:** Disabled — this team only cares about linting and code generation, not compilation
- **Generator:** Just `spring` for a Java backend

Redocly uses the `.redocly.yaml` config in your project root (if present) for custom rules. See the [Redocly docs](https://redocly.com/docs/cli/configuration/) for configuration options.

## Usage

```bash
cp .oavc /path/to/your/project/.oavc
# Optionally add a .redocly.yaml for custom Redocly rules
oav validate
```
