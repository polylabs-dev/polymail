# Poly Mail

**GitHub**: [polylabs-dev/polymail](https://github.com/polylabs-dev/polymail)
**Platform**: eStream v0.11.0
**Depends on**: PolyKit v0.3.0, eStream graph/DAG constructs

## Purpose

Post-quantum encrypted email with scatter storage, SMTP/IMAP bridge, and ESLM spam/phishing detection. All crypto in Rust/WASM, TypeScript is DOM-only.

## Zero-Linkage Privacy

HKDF context: `poly-mail-v1`. User identities are completely isolated from all other Poly products. StreamSight telemetry stays within `polylabs.mail.*` lex namespace. Blinded billing tokens prevent cross-product correlation.

## Structure

- `circuits/fl/` — FastLang circuit definitions (encryption, routing, classification, metering, SMTP bridge, RBAC, search, filter, calendar)
- `circuits/fl/graphs/` — Graph/DAG constructs (mailbox_registry, email_thread)
- `crates/` — Rust backend crates (poly-mail-core, poly-smtp-bridge, poly-sdk-backend)
- `packages/` — TypeScript SDKs and console widgets
- `apps/` — Desktop (Tauri) and mobile (React Native) clients
- `docs/` — Architecture and design documents

## Circuits (12 total)

| Circuit | File | Description |
|---------|------|-------------|
| `polymail_encrypt` | `circuits/fl/polymail_encrypt.fl` | E2E encryption with ML-KEM-1024 |
| `polymail_classify` | `circuits/fl/polymail_classify.fl` | ESLM spam/phishing classification (on-device) |
| `polymail_metering` | `circuits/fl/polymail_metering.fl` | Email operation metering |
| `polymail_platform_health` | `circuits/fl/polymail_platform_health.fl` | Blind relay health monitoring |
| `polymail_route` | `circuits/fl/polymail_route.fl` | Scatter-CAS routing + MX gateway |
| `polymail_smtp_bridge` | `circuits/fl/polymail_smtp_bridge.fl` | SMTP/IMAP bridge for classical clients |
| `polymail_rbac` | `circuits/fl/polymail_rbac.fl` | RBAC graph (Owner, Admin, Member, ReadOnly) |
| `polymail_search` | `circuits/fl/polymail_search.fl` | Encrypted search index (on-device only) |
| `polymail_filter` | `circuits/fl/polymail_filter.fl` | Server-side filter rules (metadata only) |
| `polymail_calendar` | `circuits/fl/polymail_calendar.fl` | iCalendar invite handling |
| `polymail_mailbox_graph` | `circuits/fl/graphs/polymail_mailbox_graph.fl` | Mailbox registry graph |
| `polymail_thread_dag` | `circuits/fl/graphs/polymail_thread_dag.fl` | Email thread DAG |

## Key Graphs

- `graph mailbox_registry` — accounts, folders, labels, contacts with `ai_feed spam_detection`
- `dag email_thread` — conversation threading with `enforce acyclic`, `state_machine email_lifecycle`
- `graph mail_roles` — RBAC for mailbox operations (Owner > Admin > Member > ReadOnly)

## v0.11.0 Conventions

- All stored types use `data X : mail v1 { ... }` with `store kv|graph|dag`, `govern lex`, and `cortex { ... }`
- FSMs use `persistence wal`, `terminal [...]`, `ai_anomaly_detection true`
- All circuits include `precision` level and `property safety|liveness` where appropriate
- Config in `estream.toml` (replaces `estream-component.toml`)

## Commit Convention

Commit to the GitHub issue or epic the work was done under.

## Cross-Repo Coordination

This repo is part of the [polylabs-dev](https://github.com/polylabs-dev) organization, coordinated through the **AI Toolkit hub** at `toddrooke/ai-toolkit/`.

For cross-repo context, strategic priorities, and the master work queue:
- `toddrooke/ai-toolkit/CLAUDE-CONTEXT.md` — org map and priorities
- `toddrooke/ai-toolkit/scratch/BACKLOG.md` — master backlog
- `toddrooke/ai-toolkit/repos/polylabs-dev.md` — this org's status summary
