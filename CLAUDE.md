# Poly Mail

**GitHub**: [polylabs-dev/polymail](https://github.com/polylabs-dev/polymail)
**Platform**: eStream v0.8.3
**Depends on**: PolyKit v0.3.0, eStream graph/DAG constructs

## Purpose

Post-quantum encrypted email with scatter storage, SMTP/IMAP bridge, and ESLM spam/phishing detection. All crypto in Rust/WASM, TypeScript is DOM-only.

## Zero-Linkage Privacy

HKDF context: `poly-mail-v1`. User identities are completely isolated from all other Poly products. StreamSight telemetry stays within `polylabs.mail.*` lex namespace. Blinded billing tokens prevent cross-product correlation.

## Structure

- `circuits/fl/` — FastLang circuit definitions (encryption, routing, classification, metering, SMTP bridge)
- `circuits/fl/graphs/` — Graph/DAG constructs (mailbox_registry, email_thread)
- `crates/` — Rust backend crates (poly-mail-core, poly-smtp-bridge, poly-sdk-backend)
- `packages/` — TypeScript SDKs and console widgets
- `apps/` — Desktop (Tauri) and mobile (React Native) clients
- `docs/` — Architecture and design documents

## Key Graphs

- `graph mailbox_registry` — accounts, folders, labels, contacts with `ai_feed spam_detection`
- `dag email_thread` — conversation threading with `enforce acyclic`, `state_machine email_lifecycle`
- `polymail_smtp_bridge.fl` — classical email gateway for inbound/outbound SMTP/IMAP

## Commit Convention

Commit to the GitHub issue or epic the work was done under.

## Cross-Repo Coordination

This repo is part of the [polylabs-dev](https://github.com/polylabs-dev) organization, coordinated through the **AI Toolkit hub** at `toddrooke/ai-toolkit/`.

For cross-repo context, strategic priorities, and the master work queue:
- `toddrooke/ai-toolkit/CLAUDE-CONTEXT.md` — org map and priorities
- `toddrooke/ai-toolkit/scratch/BACKLOG.md` — master backlog
- `toddrooke/ai-toolkit/repos/polylabs-dev.md` — this org's status summary
