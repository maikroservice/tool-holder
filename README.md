# tool-holder

Collects credentials and tokens from "dumb" tools (tools that store results locally but cannot push them anywhere) and ingests them into [ATC](../automatic-tool-changer) via `POST /ingest`.

## How it works

```
Operator runs tool manually (e.g. NoPhish)
Tool stores found credentials → local DB / file / stdout
                                        ↓
              tool-holder reads tool's storage
              extracts new credentials/tokens
                                        ↓
              POST /ingest → ATC
              ATC tests the credentials
```

## Directory structure

Each tool gets its own subdirectory under `tools/`:

```
tools/
  nophish/
    config.yaml      # committed — source + mapping config, uses ${VAR} refs
    .env             # gitignored — real secrets
    .env.example     # committed — documents required variables
  another_tool/
    config.yaml
    .env
    .env.example
example.env          # committed — root-level template for shared variables
```

Variable lookup order for each tool: tool's `.env` → root `.env` → process environment.

See `tools/example/` for a fully annotated template.

## Configuration

`config.yaml` inside each tool directory:

```yaml
name: nophish

source:
  type: database       # database | mongo | file | stdout
  driver: postgres     # postgres | sqlite | mysql  (database only)
  host: db.internal
  port: 5432
  database: nophish
  table: found_credentials
  columns:
    - credential_value
    - credential_type
  cursor_field: id
  credentials:         # optional — omit if no auth required
    username: ${DB_USER}
    password: ${DB_PASS}

mapping:
  token: credential_value   # ATC field: tool field
  type: credential_type

atc:
  url: https://atc.example.com
  ingest_key: ${INGEST_KEY}
```

All `${VAR}` references are resolved at startup. Secrets must never be written as literal values in YAML files.

### Source types

| Type | Required fields |
|---|---|
| `database` | `driver`, `host`, `port`, `database`, then `table`+`columns` **or** `query` |
| `mongo` | `host`, `port`, `database`, `collection` |
| `file` | `format` (`json`\|`yaml`\|`txt`), `path` |
| `stdout` | `command`, optional `args`, optional `format` |

`credentials` is optional for all source types.

### Mapping

Maps ATC field names (left) to the field names used by the tool (right):

```yaml
mapping:
  token: credential_value   # send row["credential_value"] as "token" to ATC
  type: credential_type
```

Only mapped fields are sent — unmapped tool fields are dropped.

## Secrets

Each tool's `.env` only needs the variables referenced in its own `config.yaml`.
Because each tool has its own `.env`, variable names don't need to be globally unique
(both tools can have `DB_USER` without conflict).

```
# tools/nophish/.env
INGEST_KEY=atc_...
DB_USER=nophish_reader
DB_PASS=...
```

Shared variables (e.g. a single `INGEST_KEY` used by all tools) can live in the root `.env`:

```
# .env  (root)
INGEST_KEY=atc_...
```

Copy `example.env` to `.env` as a starting point.

## Adding a new tool

1. Create `tools/<tool_name>/`
2. Copy `tools/example/config.yaml` and fill in the source, mapping, and ATC details
3. Copy `tools/example/.env.example` to `.env` and fill in real values
4. Run `cargo run` to verify the config loads

## Running

```bash
cargo run
```

## Testing

```bash
cargo test
```
