# Poly Mail

Post-quantum encrypted email built on eStream v0.8.1.

## Overview

Poly Mail is a quantum-safe email service where every message is ML-KEM-1024 encrypted, scatter-distributed across multiple providers/jurisdictions, and authenticated via SPARK biometric. No passwords, no classical crypto, no single point of seizure.

## Architecture

```
Client (Tauri/Mobile)
    |
    +-- SPARK Auth (ML-DSA-87 biometric)
    |
    +-- Compose/Read (E2E PQ encryption)
    |
    v
eStream Wire Protocol (QUIC/UDP)
    |
    v
ESCIR Mail Router Circuit
    |
    +-- Inbound: SMTP Bridge -> PQ Encrypt -> Scatter Store
    +-- Outbound: Reassemble -> PQ Decrypt -> Render
    +-- External: SMTP Bridge -> Classical re-encrypt for non-Poly recipients
    |
    v
Scatter Storage (k-of-n across providers/jurisdictions)
```

## Key Components

| Component | Location | Purpose |
|-----------|----------|---------|
| Mail Router | circuits/ | ESCIR circuit for mail routing, filtering, delivery |
| SMTP Bridge | crates/smtp-bridge/ | Local IMAP/SMTP server for conventional email clients |
| Client SDK | crates/poly-mail-core/ | Rust core for encryption, composition, search |
| Desktop App | apps/desktop/ | Tauri-based desktop client |
| Mobile App | apps/mobile/ | React Native with Rust FFI |
| ESLM Spam Filter | circuits/ | AI-powered spam/phishing detection |

## No REST API

All communication uses the eStream Wire Protocol. There are no REST/HTTP endpoints. The SMTP/IMAP bridge runs locally on the user's device to support conventional email clients.

## Stream Topics

```
polymail.{user_id}.inbox          # Incoming mail delivery
polymail.{user_id}.outbox         # Outgoing mail queue
polymail.{user_id}.sent           # Sent mail confirmation
polymail.{user_id}.draft          # Draft sync across devices
polymail.{user_id}.search.index   # Client-side search index sync
polymail.{user_id}.spam.verdict   # ESLM spam classification
polymail.bridge.smtp.inbound      # External SMTP -> Poly Mail
polymail.bridge.smtp.outbound     # Poly Mail -> External SMTP
```

## Enterprise Features

- Custom domains with PQ-signed DKIM
- Admin console for organization management
- Compliance: message retention policies via ESCIR FSM
- Data residency: region-locked scatter storage
- Poly OAuth integration for SSO
- Audit logs (encrypted, scatter-stored)

## Platform

- eStream v0.8.1
- ESCIR SmartCircuits for all routing/processing logic
- ML-KEM-1024 (encryption), ML-DSA-87 (signatures), SHA3-256 (hashing)
- 8-Dimension metering for resource accounting
- L2 multi-token payments (USDC, SOL, cbBTC)
