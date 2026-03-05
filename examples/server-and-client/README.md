# Server and client validation

A full-stack project generating both server stubs and client SDKs with customized generator configs.

- **Mode:** `both` — generates servers and clients
- **Servers:** `spring` (Java) and `go-server`, with a custom Spring config override
- **Clients:** `typescript-axios` (with a custom config) and `java`
- **Generator overrides:** Version-controlled configs in `generator-configs/` for Spring and TypeScript
- **Jobs:** 4 parallel — speeds up generation and compilation across 4 generators

## Generator overrides

Files in `generator-configs/` customize OpenAPI Generator behavior:
- `spring.yaml` — sets package names, enables Spring Boot 3 and Jakarta EE
- `typescript-axios.yaml` — sets the npm package name, enables ES6 and separate models

These override the defaults in `.oav/generators/` and can be committed to the repo.

## Usage

```bash
cp -r .oavc generator-configs/ /path/to/your/project/
oav validate
```
