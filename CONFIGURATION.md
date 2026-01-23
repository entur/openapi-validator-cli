# Configuration

## .oavc Defaults

| Key                   | Default                                      | Description                       |
|-----------------------|----------------------------------------------|-----------------------------------|
| `spec`                | —                                            | Path to OpenAPI spec (required)   |
| `mode`                | `server`                                     | `server`, `client`, or `both`     |
| `lint`                | `true`                                       | Run Redocly linting               |
| `generate`            | `true`                                       | Generate code from spec           |
| `compile`             | `true`                                       | Build generated code              |
| `server_generators`   | `[]`                                         | Server generators to use          |
| `client_generators`   | `[]`                                         | Client generators to use          |
| `generator_overrides` | `{}`                                         | Custom config paths per generator |
| `generator_image`     | `openapitools/openapi-generator-cli:v7.17.0` | OpenAPI Generator image           |
| `redocly_image`       | `redocly/cli:1.25.5`                         | Redocly CLI image                 |

When `server_generators` or `client_generators` is empty, all generators for that mode are used.

## Generator Reference

### Server Generators

| Generator           | Stack                  |
|---------------------|------------------------|
| `aspnetcore`        | .NET 8.0, ASP.NET Core |
| `go-server`         | Go, Chi router         |
| `kotlin-spring`     | Kotlin, Spring Boot 3  |
| `python-fastapi`    | Python 3, FastAPI      |
| `spring`            | Java 21, Spring Boot 3 |
| `typescript-nestjs` | TypeScript, NestJS     |

### Client Generators

| Generator          | Stack                 |
|--------------------|-----------------------|
| `csharp`           | .NET 8.0              |
| `go`               | Go                    |
| `java`             | Java 21, Maven        |
| `kotlin`           | Kotlin, OkHttp4       |
| `python`           | Python 3, setuptools  |
| `typescript-axios` | TypeScript, Axios     |
| `typescript-fetch` | TypeScript, Fetch API |
| `typescript-node`  | TypeScript, Node.js   |

## Customizing Generators

After running `oav init`, the `.oav/` folder contains:

```
.oav/
├── generators/
│   ├── server/          # Server generator configs (YAML)
│   └── client/          # Client generator configs (YAML)
├── docker-compose.yaml  # Build services for compile step
├── generated/           # Generated code output
└── reports/             # Lint/generate/compile logs
```

Edit files in `.oav/generators/` to customize OpenAPI Generator options. Changes persist across `oav validate` runs. For configs you want to version control, use `generator_overrides` to point to files outside `.oav/`:

```yaml
# .oavc
generator_overrides:
  spring: ./generator-configs/spring.yaml
  typescript-axios: ./generator-configs/ts-axios.yaml
```

Resolution order: override path (if set) → `.oav/generators/{scope}/{name}.yaml` → embedded defaults.

The `docker-compose.yaml` defines build services using standard language images (e.g., `golang:1.25-alpine`, `node:24-alpine`). Modify if you need different base images or build commands.

## Adding Custom Generators

You can add any [OpenAPI Generator](https://openapi-generator.tech/docs/generators) not included by default:

**1. Create the generator config**

Add a YAML file in `.oav/generators/server/` or `.oav/generators/client/`:

```yaml
# .oav/generators/server/rust-axum.yaml
generatorName: rust-axum
outputDir: generated/server/rust-axum

additionalProperties:
  packageName: openapi_server
  # See generator docs for available options
```

**2. Add a build service**

Add a matching service in `.oav/docker-compose.yaml`:

```yaml
build-rust-axum:
  image: rust:1.83-alpine
  volumes:
    - ./generated/server/rust-axum:/src:rw
  working_dir: /src
  command: cargo build
```

The service name must be `build-{generator-name}` for servers or `build-client-{generator-name}` for clients.

**3. Reference in config**

Add the generator to `.oavc`:

```yaml
server_generators:
  - rust-axum
```

The CLI will pick up the new generator on the next `oav validate` run.
