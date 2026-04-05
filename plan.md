# Middleware Layer — Planning Document

## Context

This plan was started in a conversation about ATC (Automatic Tool Changer).
The earlier design conversation was lost to context compression — open questions
are marked clearly below so they can be resolved at the start of a new session.

---

## What is ATC?

ATC is a FastAPI service that:
1. Receives credentials via `POST /ingest` (from collectors like NoPhish)
2. Matches them against campaign watchers → creates runs
3. Builds CLI commands from a tool YAML definition
4. Fires the commands to an external executor via **webhook push**
5. Receives results back via `POST /runs/{id}/callback`

The current push model requires the executor to have a **publicly reachable URL**
to receive webhooks — which is a problem for tools behind NAT/firewalls.

---

## The Middleware

A **separate standalone application** (own codebase, own repo) that sits between
ATC and tools like NoPhish.

### Problem it solves

Tools like NoPhish cannot always expose a public webhook URL. The middleware
provides a **polling interface** so tools can pull work from ATC rather than
waiting for ATC to push to them.

### High-level flow (as discussed)

```
NoPhish → POST /ingest to ATC
ATC builds commands → run queued as "pending_pickup"

Middleware polls ATC  →  GET /runs/poll
Middleware claims run →  PATCH /runs/{id}/claim
Middleware dispatches to NoPhish (or other tool)
Tool executes
Middleware reports back → POST /runs/{id}/callback to ATC
```

---

## Decisions already made

- [ ] Separate codebase / new repo (confirmed)
- [ ] Polling-based, not push-based (confirmed)
- [ ] Should work with NoPhish as the first target tool

---

## Open questions (to resolve in new conversation)

1. **What exactly does the middleware own?**
   - Does it just relay work between ATC and tools?
   - Or does it have its own job queue, retry logic, scheduling?

2. **How do tools register with the middleware?**
   - Static config file listing tool endpoints?
   - Dynamic registration API?

3. **How does the middleware authenticate with ATC?**
   - Ingest key? App token? New key type?

4. **How do tools authenticate with the middleware?**
   - Shared secret? Per-tool API keys?

5. **Tech stack?**
   - Python (FastAPI, like ATC)? Something else?
   - Any decisions made on persistence (DB, Redis, in-memory)?

6. **Name of the project?**

7. **Should the middleware also handle fan-out?**
   - e.g. one ATC run → dispatched to multiple tool instances in parallel?

8. **Retry / dead-letter behaviour?**
   - What happens if a tool claims a run but never calls /callback?

---

## ATC endpoints relevant to the middleware

| Endpoint | Purpose |
|---|---|
| `POST /ingest` | Submit credentials (NoPhish already calls this) |
| `GET /runs/poll` | **New** — returns pending_pickup runs (needs building in ATC) |
| `PATCH /runs/{id}/claim` | **New** — atomic claim to prevent double-pickup |
| `POST /runs/{id}/callback` | Existing — report execution result |

---

## Related ATC files

- `main.py` — all endpoints, run lifecycle, webhook fire logic
- `database.py` — `db_list_audit`, `db_append_audit`, run table schema
- `tools/*.yaml` — tool definitions (command, env vars, parameters)
- `tool_loader.py` — builds CLI commands from YAML + token values
