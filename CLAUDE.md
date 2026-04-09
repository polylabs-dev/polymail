# Q Mail

**GitHub**: [polylabs-dev/qmail](https://github.com/polylabs-dev/qmail)
**Platform**: eStream v0.22.0
**Depends on**: QKit v0.3.0, eStream graph/DAG constructs

100% FastLang. No hand-written Rust.

## Purpose

Post-quantum encrypted email with scatter storage, SMTP/IMAP bridge, and ESLM spam/phishing detection. All crypto compiled from FastLang via FLIR codegen.

## Zero-Linkage Privacy

HKDF context: `q-mail-v1`. User identities are completely isolated from all other Q products. StreamSight telemetry stays within `polyqlabs.mail.*` lex namespace. Blinded billing tokens prevent cross-product correlation.

## Structure

- `circuits/fl/` — FastLang circuit definitions (encryption, routing, classification, metering, SMTP bridge, RBAC, search, filter, calendar)
- `circuits/fl/graphs/` — Graph/DAG constructs (mailbox_registry, email_thread)
- `apps/` — Desktop (Tauri) and mobile (React Native) clients
- `docs/` — Architecture and design documents

> **Note**: `crates/` and `packages/` are legacy scaffolding superseded by FLIR codegen. All logic lives in FastLang circuits.

## Circuits (12 total)

| Circuit | File | Description |
|---------|------|-------------|
| `qmail_encrypt` | `circuits/fl/qmail_encrypt.fl` | E2E encryption with ML-KEM-1024 |
| `qmail_classify` | `circuits/fl/qmail_classify.fl` | ESLM spam/phishing classification (on-device) |
| `qmail_metering` | `circuits/fl/qmail_metering.fl` | Email operation metering |
| `qmail_platform_health` | `circuits/fl/qmail_platform_health.fl` | Blind relay health monitoring |
| `qmail_route` | `circuits/fl/qmail_route.fl` | Scatter-CAS routing + MX gateway |
| `qmail_smtp_bridge` | `circuits/fl/qmail_smtp_bridge.fl` | SMTP/IMAP bridge for classical clients |
| `qmail_rbac` | `circuits/fl/qmail_rbac.fl` | RBAC graph (Owner, Admin, Member, ReadOnly) |
| `qmail_search` | `circuits/fl/qmail_search.fl` | Encrypted search index (on-device only) |
| `qmail_filter` | `circuits/fl/qmail_filter.fl` | Server-side filter rules (metadata only) |
| `qmail_calendar` | `circuits/fl/qmail_calendar.fl` | iCalendar invite handling |
| `qmail_mailbox_graph` | `circuits/fl/graphs/qmail_mailbox_graph.fl` | Mailbox registry graph |
| `qmail_thread_dag` | `circuits/fl/graphs/qmail_thread_dag.fl` | Email thread DAG |

## Key Graphs

- `graph mailbox_registry` — accounts, folders, labels, contacts with `ai_feed spam_detection`
- `dag email_thread` — conversation threading with `enforce acyclic`, `state_machine email_lifecycle`
- `graph mail_roles` — RBAC for mailbox operations (Owner > Admin > Member > ReadOnly)

## v0.22.0 Conventions

- All stored types use `data X : mail v1 { ... }` with `store kv|graph|dag`, `govern lex`, and `cortex { ... }`
- FSMs use `persistence wal`, `terminal [...]`, `ai_anomaly_detection true`
- All circuits include `precision` level and `property safety|liveness` where appropriate
- Config in `estream.toml` (replaces `estream-component.toml`)

## Commit Convention

Commit to the GitHub issue or epic the work was done under.
