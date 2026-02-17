# Poly Mail Architecture

**Version**: 1.0
**Last Updated**: February 2026
**Platform**: eStream v0.8.1

---

## Overview

Poly Mail is a post-quantum encrypted email service built natively on the eStream platform. Every email is E2E encrypted with ML-KEM-1024, scatter-distributed across multiple providers and jurisdictions, and authenticated via SPARK biometric keys.

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Poly Mail Client                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ Compose   │  │ Inbox    │  │ Search   │  │ Settings │   │
│  │ Editor    │  │ View     │  │ Index    │  │ / Admin  │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
│       │              │              │              │         │
│  ┌────┴──────────────┴──────────────┴──────────────┴─────┐  │
│  │              poly-mail-core (Rust)                      │  │
│  │  PQ Encrypt/Decrypt | MIME Parse | Contact Book        │  │
│  │  Local Search Index | Classification | Offline Cache   │  │
│  └──────────────────────┬────────────────────────────────┘  │
│                          │                                   │
│  ┌──────────────────────┴────────────────────────────────┐  │
│  │              SMTP/IMAP Bridge (Local)                  │  │
│  │  For conventional email clients (Outlook, Thunderbird) │  │
│  └──────────────────────┬────────────────────────────────┘  │
└─────────────────────────┼───────────────────────────────────┘
                          │
                   eStream Wire Protocol (QUIC/UDP)
                          │
┌─────────────────────────┼───────────────────────────────────┐
│                    eStream Network                            │
│                          │                                   │
│  ┌──────────────────────┴────────────────────────────────┐  │
│  │           ESCIR Mail Router Circuit                    │  │
│  │  Routing | Filtering | Delivery | Retry | Bounce      │  │
│  └──────┬───────────┬───────────┬───────────┬────────────┘  │
│         │           │           │           │               │
│  ┌──────┴──┐ ┌──────┴──┐ ┌─────┴───┐ ┌─────┴───┐         │
│  │ ESLM    │ │ SMTP    │ │ Scatter │ │ Metering│         │
│  │ Spam    │ │ Gateway │ │ Storage │ │ Circuit │         │
│  │ Filter  │ │ Bridge  │ │ Circuit │ │         │         │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘         │
└─────────────────────────────────────────────────────────────┘
```

---

## Email Flow: Poly-to-Poly

### Sending

1. User composes email in Poly Mail client
2. `poly-mail-core` encrypts body + attachments with recipient's ML-KEM-1024 public key
3. Signs email with sender's ML-DSA-87 key (SPARK biometric)
4. Email published to `polymail.{user_id}.outbox` stream topic
5. Mail Router Circuit:
   a. Validates sender signature
   b. Applies sender's outbound rules (delay, schedule)
   c. Erasure-codes encrypted email into k-of-n shards
   d. Scatter-distributes shards across providers/jurisdictions
   e. Publishes delivery confirmation to `polymail.{user_id}.sent`
6. Recipient's Mail Router:
   a. Collects shards from scatter network
   b. Reassembles encrypted email
   c. Delivers to `polymail.{recipient_id}.inbox`
7. Recipient's client decrypts with their ML-KEM-1024 private key

### Key Property

No single provider, jurisdiction, or network path holds a complete email at any point. The clear-text only exists on the sender's and recipient's devices.

---

## Email Flow: External (SMTP Interop)

### Inbound (External -> Poly Mail)

```
External Sender -> Internet SMTP -> Poly Mail SMTP Gateway
    |
    v
SMTP Gateway:
    1. Receive via standard SMTP/TLS
    2. Verify DKIM/SPF/DMARC
    3. ESLM spam/phishing classification
    4. Re-encrypt with recipient's ML-KEM-1024 key
    5. Scatter-store encrypted email
    6. Deliver to recipient's inbox stream
```

### Outbound (Poly Mail -> External)

```
Poly Mail User -> Compose -> poly-mail-core
    |
    v
Mail Router:
    1. Detect external recipient (not @polymail domain)
    2. Decrypt email (on gateway, in memory only)
    3. Re-encrypt with TLS for SMTP delivery
    4. Sign with DKIM (PQ-signed internally, classical DKIM for compatibility)
    5. Deliver via SMTP to recipient's MX server
    6. Purge plaintext from memory
```

**Security note**: Emails to external (non-Poly) recipients lose PQ E2E encryption at the gateway boundary. The email is encrypted in transit (TLS) but not E2E encrypted. Users are warned when composing to external recipients.

---

## Local SMTP/IMAP Bridge

For users who prefer conventional email clients (Outlook, Thunderbird, Apple Mail):

```
Conventional Client <-> IMAP/SMTP <-> Local Bridge <-> eStream Wire Protocol
```

The bridge runs on the user's device:
- **IMAP server** (localhost:1143): Presents decrypted mailbox to local clients
- **SMTP server** (localhost:1025): Accepts outgoing mail, PQ-encrypts, routes via eStream
- **Authentication**: SPARK biometric (bridge auto-authenticated, client uses local token)

This mirrors Proton Mail Bridge's architecture but with PQ encryption and scatter storage.

---

## Search

Email search is entirely client-side:

1. Incoming emails are decrypted on-device
2. `poly-mail-core` builds a local encrypted search index
3. Index is PQ-encrypted and scatter-synced across devices
4. Search queries execute locally against the decrypted index
5. No search queries are sent to any server

The search index supports:
- Full-text body search
- Header search (from, to, subject, date)
- Attachment name/type search
- Classification tag filtering
- Folder/label filtering

---

## Classification-Aware Storage

Emails inherit or receive classification tags that control scatter policy:

| Classification | Scatter | Offline | Retention |
|---------------|---------|---------|-----------|
| PUBLIC | 2-of-3 | Yes | User-controlled |
| INTERNAL | 3-of-5 | Yes | User-controlled |
| CONFIDENTIAL | 5-of-7 | Selective | Policy-based |
| RESTRICTED | 7-of-9, 3+ jurisdictions | No | Auto-expire |
| SOVEREIGN | 9-of-13, 5+ jurisdictions, HSM | No | Compliance-driven |

---

## Custom Domains

Enterprise customers can use custom domains:

```
@company.com -> Poly Mail infrastructure
```

- MX records point to Poly Mail SMTP Gateway
- PQ-signed DKIM keys (ML-DSA-87 internally, classical RSA/Ed25519 for external compatibility)
- SPF/DMARC configuration via admin console
- Per-domain retention/classification policies
- Admin can set org-wide rules (e.g., all emails classified CONFIDENTIAL minimum)

---

## Enterprise Bundle

Poly Mail anchors the "Poly Labs for Business" enterprise offering:

| Feature | Description |
|---------|-------------|
| Custom domains | Multiple domains per org |
| Admin console | User management, policies, audit |
| Compliance | Retention policies, legal hold, eDiscovery (MPC-based) |
| Data residency | Region-locked scatter storage |
| Poly OAuth SSO | Single sign-on via SPARK biometric |
| DLP | Classification-based data loss prevention |
| Migration tools | Import from Gmail, Outlook, Exchange |
| Programmatic access | eStream Wire Protocol + ESCIR SmartCircuits (no REST API) |
| SLA | 99.99% uptime guarantee |

---

## ESCIR Circuits

### Mail Router Circuit

```yaml
escir: "0.8.1"
name: poly-mail-router
version: "1.0.0"
lex: polylabs.mail

stream:
  - topic: "polylabs.mail.{user_id}.inbox"
    pattern: scatter
    retention: user_policy
    hash_chain: true
    signature_required: true

  - topic: "polylabs.mail.{user_id}.outbox"
    pattern: request_reply
    retention: until_delivered
    signature_required: true

  - topic: "polylabs.mail.bridge.smtp.inbound"
    pattern: fanout
    retention: ephemeral

  - topic: "polylabs.mail.bridge.smtp.outbound"
    pattern: request_reply
    retention: ephemeral

fsm:
  initial_state: received
  states:
    received:
      transitions:
        - event: spam_check_pass
          target: delivering
        - event: spam_check_fail
          target: quarantined
    delivering:
      transitions:
        - event: scatter_complete
          target: delivered
        - event: scatter_failed
          target: retry
    delivered:
      transitions:
        - event: read
          target: read
        - event: archived
          target: archived
    quarantined:
      transitions:
        - event: user_approved
          target: delivering
        - event: user_deleted
          target: deleted
```

### ESLM Spam Filter Circuit

```yaml
escir: "0.8.1"
name: poly-mail-spam-filter
version: "1.0.0"
lex: polylabs.mail.spam

stream:
  - topic: "polylabs.mail.spam.{user_id}.classify"
    pattern: request_reply
    retention: none
    signature_required: true

  - topic: "polylabs.mail.spam.{user_id}.verdict"
    pattern: scatter
    retention: 30d
    hash_chain: true
```

---

## Metering

Email operations consume eStream 8-Dimension resources:

| Operation | Primary Dimensions |
|-----------|-------------------|
| Send email | Bandwidth, Operations, Storage |
| Receive email | Bandwidth, Storage |
| Search | Operations, Memory |
| SMTP bridge | Bandwidth, Operations |
| Attachment store | Storage, Bandwidth |
| Spam classification | Operations, Memory (ESLM) |

---

## Roadmap

### Phase 1: Core (Q2-Q3 2026)
- Poly-to-Poly encrypted email
- Basic web/desktop client (Tauri)
- SPARK biometric authentication
- Scatter storage (3-of-5)
- Client-side search

### Phase 2: Interop (Q3-Q4 2026)
- SMTP/IMAP bridge (local)
- External email send/receive
- Custom domains
- Mobile apps (iOS, Android)
- Import/migration tools

### Phase 3: Enterprise (Q1 2027)
- Admin console
- Compliance/retention policies
- Poly OAuth SSO
- DLP/classification
- Enterprise SLA

### Phase 4: Advanced (2027+)
- ESLM-powered smart inbox
- Calendar integration (Poly Calendar)
- Poly Mind integration (email corpus ingestion)
- FPGA-accelerated encryption for high-volume enterprise

---

## Related Documents

- [polylabs/business/PRODUCT_FAMILY.md] -- Product specifications
- [polylabs/business/PROTON_REFERENCE.md] -- Proton Mail reference analysis
- [polylabs/business/STRATEGY.md] -- Overall strategy
