# oav Reference

## JSON output schema

`oav validate --output json` produces:

```json
{
  "spec": "<absolute path to OpenAPI spec>",
  "mode": "server | client | both",
  "phases": {
    "lint": {
      "linter": "spectral | redocly",
      "status": "pass | fail",
      "log": "<full linter output>"
    },
    "generate": [
      {
        "generator": "<generator name>",
        "scope": "server | client",
        "status": "pass | fail",
        "log": "<generator output>"
      }
    ],
    "compile": [
      {
        "generator": "<generator name>",
        "scope": "server | client",
        "status": "pass | fail",
        "log": "<compiler output>"
      }
    ]
  },
  "summary": {
    "total": 0,
    "passed": 0,
    "failed": 0
  }
}
```

`phases.lint` is omitted if linting is disabled. `phases.generate` and `phases.compile` are omitted if those phases are skipped. Each `log` field contains the full raw output for that step (up to 100 KB).

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | All phases passed |
| 1 | One or more validation failures |
| 2 | Infrastructure error (Docker unavailable, config invalid, etc.) |

## Config fields (`.oavc`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `spec` | string | — | Path to the OpenAPI spec file |
| `mode` | enum | `server` | `server`, `client`, or `both` |
| `server_generators` | string[] | all | Which server generators to run |
| `client_generators` | string[] | all | Which client generators to run |
| `lint` | bool | `true` | Enable lint phase |
| `generate` | bool | `true` | Enable generate phase |
| `compile` | bool | `true` | Enable compile phase |
| `linter` | enum | `spectral` | `spectral`, `redocly`, or `none` |
| `spectral_ruleset` | string | default | Path or URL to custom Spectral ruleset |
| `docker_timeout` | u64 | `300` | Docker step timeout in seconds |
| `search_depth` | usize | `4` | Directory depth for spec auto-discovery |
| `jobs` | string | `auto` | Parallel jobs: `auto` or a positive integer |
| `manage_gitignore` | bool | `true` | Auto-add `.oav/` to `.gitignore` |

## Available generators

**Server:** aspnetcore, go-server, kotlin-spring, python-fastapi, spring, typescript-nestjs

**Client:** csharp, go, java, kotlin, python, typescript-axios, typescript-fetch, typescript-node

## Common error patterns

Use these examples to identify root causes and suggest fixes when parsing `log` output from failed phases.

### Spectral lint errors

Format: `<json-path>  <severity>  <rule-name>  <message>`

```
/paths/~1pets/get/responses/200  error  oas3-schema  "schema" property must be present
```
Fix: The `/pets` GET 200 response is missing a schema. Add `content.application/json.schema` with a proper schema object or `$ref`.

```
/components/schemas/Pet/properties/status  warning  oas3-valid-schema-example  "example" property must match schema
```
Fix: The `example` value for `Pet.status` doesn't match its type or enum constraints. Update the example to be a valid value.

### Generator errors

Typically appear as Java/Kotlin/Python exceptions or validation warnings in the openapi-generator output:

```
[main] WARN o.o.codegen.utils.ModelUtils - Schema 'Address' has no properties defined
[main] ERROR o.o.codegen.DefaultGenerator - Could not process model 'Address'
```
Fix: The `Address` schema in the spec is empty or uses an unsupported construct (e.g., bare `additionalProperties` without `type: object`). Define at least one property or add `type: object`.

```
[main] ERROR o.o.codegen.DefaultGenerator - /paths/~1orders/post/requestBody/content/application~1json/schema - $ref '#/components/schemas/OrderRequest' not found
```
Fix: A `$ref` points to `OrderRequest` which doesn't exist in `components/schemas`. Either create the schema or fix the typo in the `$ref` path.

### Compile errors

Standard compiler output for the target language. Java/Spring example:

```
src/main/java/org/openapitools/model/Pet.java:15: error: cannot find symbol
    private PetStatus status;
            ^
  symbol:   class PetStatus
  location: class Pet
```
Fix: `PetStatus` is referenced but not defined. The spec likely uses an inline enum for the `status` property that the generator couldn't resolve. Define `PetStatus` as a named schema in `components/schemas` and reference it via `$ref`.

```
src/main/java/org/openapitools/api/PetsApi.java:42: error: incompatible types
    return ResponseEntity.ok(result);
                             ^
  required: List<Pet>
  found:    Pet
```
Fix: The spec declares the response as `type: array` with `items: $ref: Pet`, but the operation's response schema is inconsistent (e.g., wraps the array in an object somewhere). Check that the response schema directly uses `type: array` with the correct `items`.
