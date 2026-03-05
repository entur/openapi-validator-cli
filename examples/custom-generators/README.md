# Custom generators

A project using two custom generators not included in oav's built-in set.

- **Server:** `rust-axum` — generates a Rust Axum server and compiles with `cargo build`
- **Client:** `swift5` — generates a Swift 5 client for iOS with async/await support
- **Custom dir:** `generators/` contains the YAML definitions for both
- **Timeout:** Bumped to 600s — Rust compilation can be slow on first build

## Generator definitions

Each YAML file in `generators/` defines:
- `name` — must be unique and not collide with built-in generators
- `scope` — `server` or `client`
- `generate` — Docker image and command for code generation
- `compile` (optional) — Docker image and command for compilation

## Usage

```bash
cp -r .oavc generators/ /path/to/your/project/
oav validate
```
