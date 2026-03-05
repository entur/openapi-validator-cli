# Examples

Example configurations for common oav usage patterns. Each directory contains a complete `.oavc` config and any supporting files needed.

| Example | Description |
|---|---|
| [client-only](client-only/) | Frontend/mobile team generating only client SDKs |
| [custom-generators](custom-generators/) | Two custom generators (Rust Axum + Swift 5), no built-ins |
| [custom-linter](custom-linter/) | Redocly linter instead of Spectral, compile disabled |
| [server-and-client](server-and-client/) | Full-stack with generator overrides and parallel jobs |
| [lint-only](lint-only/) | CI-focused lint-only config, no code generation |

## Getting started

Copy the `.oavc` file (and any supporting files) into your project root:

```bash
cp examples/client-only/.oavc /path/to/your/project/.oavc
```

Then point the `spec` field to your OpenAPI spec and run:

```bash
oav validate
```

See [CONFIGURATION.md](../CONFIGURATION.md) for the full config reference.
