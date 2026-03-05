# Client-only validation

A frontend or mobile team that consumes an API and only needs client SDKs.

- **Mode:** `client` — skips all server generators
- **Generators:** `typescript-axios` for the web app, `kotlin` for Android
- **Linter:** Spectral with `warn` severity — catches warnings, not just errors

## Usage

```bash
cp .oavc /path/to/your/project/.oavc
oav validate
```
