# Poly Mail Architecture

**Version**: 2.0
**Date**: February 2026
**Platform**: eStream v0.8.3
**Upstream**: PolyKit v0.3.0, eStream graph/DAG constructs
**Build Pipeline**: FastLang (.fl) → ESCIR → Rust/WASM codegen → .escd

---

## Overview

Poly Mail is a post-quantum encrypted email service built on the eStream platform. Every email is E2E encrypted with ML-KEM-1024, scatter-distributed across multiple providers and jurisdictions, and authenticated via SPARK biometric keys. All cryptographic operations run in WASM (Rust). TypeScript is a DOM binding layer only.

This document supersedes the v1.0 scaffold architecture. The email model is now expressed as eStream graph/DAG constructs with typed nodes, edges, overlays, CSR tiered storage, AI feeds, anomaly detection, and series attestation.

---

## Zero-Linkage Privacy

- **HKDF context**: `poly-mail-v1` — independent from all other Poly products
- **Lex namespace**: `esn/global/org/polylabs/mail`
- **user_id**: Derived from Poly Mail-specific ML-DSA-87 public key. Cannot be linked to Poly Messenger, Poly Data, or any other product identity.
- **StreamSight**: `polylabs.mail.*` — no cross-product telemetry
- **Metering**: Own `metering_graph` instance under `polylabs.mail.metering`
- **Billing**: Blinded payment tokens. Backend cannot correlate which SPARK identity uses which products.

### Enterprise Lex Bridge

Enterprise admins can **opt-in** to cross-product visibility via an explicit lex bridge between `esn/global/org/polylabs/mail` and other product namespaces. The bridge is gated by k-of-n admin witness attestation and is revocable. Even with the bridge, individual user-level data is not cross-linked — only org-level aggregates and RBAC policy flow across products.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Poly Mail Client                               │
│                                                                       │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │  React / Tauri UI                                               │  │
│  │  Compose │ Inbox │ Thread View │ Search │ Settings │ Admin     │  │
│  └────────────────────────────┬───────────────────────────────────┘  │
│                                │                                      │
│  ┌────────────────────────────┴───────────────────────────────────┐  │
│  │  Graph/DAG Layer (WASM)                                          │  │
│  │                                                                   │  │
│  │  graph mailbox_registry  — accounts, folders, labels, contacts  │  │
│  │  dag email_thread        — conversation threading + lifecycle   │  │
│  │  graph user_graph        — per-product identity (from PolyKit) │  │
│  │  graph metering_graph    — per-product metering (from PolyKit) │  │
│  └────────────────────────────┬───────────────────────────────────┘  │
│                                │                                      │
│  ┌────────────────────────────┴───────────────────────────────────┐  │
│  │  FastLang Circuits (WASM via .escd)                              │  │
│  │  polymail_encrypt │ polymail_route │ polymail_classify           │  │
│  │  polymail_metering │ polymail_smtp_bridge                       │  │
│  └────────────────────────────┬───────────────────────────────────┘  │
│                                │                                      │
│  ┌────────────────────────────┴───────────────────────────────────┐  │
│  │  SMTP/IMAP Bridge (Local)                                        │  │
│  │  For conventional email clients (Outlook, Thunderbird)           │  │
│  └────────────────────────────┬───────────────────────────────────┘  │
│                                │                                      │
│  ┌────────────────────────────┴───────────────────────────────────┐  │
│  │  eStream SDK (@estream/tauri, @estream/react-native)             │  │
│  │  Wire protocol: QUIC/UDP :5000 │ WebTransport :4433             │  │
│  └─────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Graph/DAG Constructs

### Mailbox Registry (`polymail_mailbox_graph.fl`)

Accounts, folders, labels, and contacts form a graph. This replaces flat mailbox tables with a relational model supporting hierarchical folders, multi-label assignment, and contact resolution.

```fastlang
type MailboxNode = struct {
    mailbox_id: bytes(16),
    owner_id: bytes(16),
    display_name: bytes(128),
    domain: bytes(253),
    created_at: u64,
}

type FolderNode = struct {
    folder_id: bytes(16),
    name: bytes(128),
    parent_folder_id: bytes(16),
    sort_order: u32,
    system_folder: u8,
}

type LabelNode = struct {
    label_id: bytes(16),
    name: bytes(64),
    color: u32,
}

type MailContactNode = struct {
    contact_id: bytes(16),
    display_name: bytes(128),
    email_address: bytes(254),
    signing_pubkey: bytes(2592),
    encryption_pubkey: bytes(1568),
    is_poly_user: bool,
    last_contacted_at: u64,
}

type ContainsEdge = struct {
    added_at: u64,
}

type LabeledWithEdge = struct {
    labeled_at: u64,
}

type SentToEdge = struct {
    sent_at: u64,
    message_count: u32,
}

graph mailbox_registry {
    node MailboxNode
    node FolderNode
    node LabelNode
    node MailContactNode
    edge ContainsEdge
    edge LabeledWithEdge
    edge SentToEdge

    overlay unread_count: u32 bitmask delta_curate
    overlay folder_size: u64 bitmask delta_curate
    overlay spam_score: f32 curate delta_curate
    overlay thread_count: u32 bitmask delta_curate

    storage csr {
        hot @bram,
        warm @ddr,
        cold @nvme,
    }

    ai_feed spam_detection

    observe mailbox_registry: [unread_count, spam_score, thread_count] threshold: {
        anomaly_score 0.85
        baseline_window 120
    }
}

series mailbox_series: mailbox_registry
    merkle_chain true
    lattice_imprint true
    witness_attest true
```

Key circuits: `create_mailbox`, `create_folder`, `move_to_folder`, `apply_label`, `add_contact`, `resolve_contact`.

### Email Thread DAG (`polymail_thread_dag.fl`)

Emails form a DAG within each conversation thread. Replies create parent edges. Forwards branch. This enables threading, ordering, and causal consistency for offline/CRDT scenarios.

```fastlang
type EmailNode = struct {
    email_id: bytes(16),
    thread_id: bytes(16),
    sender_id: bytes(16),
    subject: bytes(998),
    timestamp: u64,
    encrypted_body_hash: bytes(32),
    attachment_count: u8,
    size_bytes: u64,
}

type ReplyToEdge = struct {
    reply_type: u8,
}

type ForwardEdge = struct {
    forwarded_at: u64,
    forward_note_hash: bytes(32),
}

state_machine email_lifecycle {
    initial DRAFT
    persistence wal
    terminal [SPAM, ARCHIVED, DELETED]
    li_anomaly_detection true

    DRAFT -> SENDING when user_send
    SENDING -> SENT when relay_confirmed
    SENDING -> DRAFT when send_failed guard retry_limit_reached
    SENT -> DELIVERED when recipient_acked
    DELIVERED -> READ when recipient_read
    READ -> ARCHIVED when user_archive
    DELIVERED -> ARCHIVED when user_archive
    SENT -> ARCHIVED when user_archive
    READ -> DELETED when user_delete
    DELIVERED -> DELETED when user_delete
    SENT -> DELETED when user_delete
    DRAFT -> DELETED when user_delete
    DELIVERED -> SPAM when spam_detected
    SENT -> SPAM when spam_detected
    SPAM -> DELETED when user_delete
    SPAM -> DELIVERED when user_not_spam
    ARCHIVED -> DELETED when user_delete
}

dag email_thread {
    node EmailNode
    edge ReplyToEdge
    edge ForwardEdge

    enforce acyclic

    overlay read_status: u8 curate delta_curate
    overlay star: bool curate
    overlay label_mask: u64 bitmask delta_curate
    overlay attachment_count: u8 bitmask

    storage csr {
        hot @bram,
        warm @ddr,
        cold @nvme,
    }

    observe email_thread: [read_status, label_mask] threshold: {
        anomaly_score 0.8
        baseline_window 60
    }
}

series email_series: email_thread
    merkle_chain true
    lattice_imprint true
    witness_attest true
```

Key circuits: `compose_draft`, `send_email`, `receive_email`, `mark_read`, `star_email`, `archive_email`, `delete_email`, `move_to_spam`, `mark_not_spam`.

---

## SMTP Bridge (`polymail_smtp_bridge.fl`)

Classical email gateway circuit for inbound/outbound SMTP/IMAP interoperability. This runs as a local bridge on the user's device (like Proton Mail Bridge) and as a server-side gateway for custom domains.

### Local Bridge

```
Conventional Client <-> IMAP/SMTP <-> Local Bridge <-> eStream Wire Protocol
```

- **IMAP server** (localhost:1143): Presents decrypted mailbox to local clients
- **SMTP server** (localhost:1025): Accepts outgoing mail, PQ-encrypts, routes via eStream
- **Authentication**: SPARK biometric (bridge auto-authenticated, client uses local token)

### Server-Side Gateway (Custom Domains)

```
External Internet <-> MX records -> Poly Mail SMTP Gateway
                                        |
                                        v
                              polymail_smtp_bridge.fl
                                        |
                    +-------------------+-------------------+
                    |                                       |
              Inbound:                                Outbound:
              SMTP/TLS receive                        Decrypt in memory
              DKIM/SPF/DMARC verify                   Re-encrypt TLS
              ESLM spam classify                      Classical DKIM sign
              ML-KEM-1024 re-encrypt                  SMTP deliver
              Scatter-store                            Purge plaintext
```

**Security note**: Emails to/from external (non-Poly) recipients lose PQ E2E encryption at the gateway boundary. Users are warned when composing to external recipients.

---

## ESLM Spam/Phishing Detection

All classification runs client-side in WASM. Email content never leaves the user's device for spam analysis.

- **Model**: ESLM (eStream Language Model) fine-tuned for email classification
- **Categories**: Ham, Spam, Phishing, Malware, Promotional, Social
- **Execution**: Decrypted email body → WASM ESLM inference → classification tag
- **Feedback loop**: User spam/not-spam actions update local model weights (federated learning, no raw content shared)
- **Integration**: Feeds `spam_score` overlay on `mailbox_registry` graph and triggers `spam_detected` event on `email_lifecycle` state machine

---

## PolyKit Composition

All circuits compose eStream upstream primitives via PolyKit profiles:

| PolyKit Profile | Usage |
|-----------------|-------|
| `poly_framework_standard` | Mail routing, folder management, contact resolution, search |
| `poly_framework_sensitive` | Encryption, SMTP bridge, spam classification, key management |

### RBAC

Composes `rbac.fl` via PolyKit for per-mailbox and per-folder access control:

- `mailbox:owner` — full control
- `mailbox:delegate` — send on behalf, read inbox
- `folder:viewer` — read-only access to shared folder
- `folder:editor` — read/write/organize

### Enterprise Group Hierarchy

Composes `group_hierarchy.fl` for enterprise organizations:

- Org → Department → Team → User containment
- Per-department email policies (retention, classification, DLP)
- Admin delegation via hierarchy levels

---

## Tiers

| Tier | Price | Storage | Custom Domains | Scatter | Features |
|------|-------|---------|----------------|---------|----------|
| FREE | $0 | 500 MB | No | 2-of-3 | Basic email, ESLM spam |
| PREMIUM | $4.99/mo | 5 GB | 1 domain | 3-of-5 | Labels, filters, IMAP bridge |
| PRO | $9.99/mo | 50 GB | 5 domains | 5-of-7 | Priority relay, advanced search |
| ENTERPRISE | Custom | Custom | Unlimited | 7-of-9+ | Admin console, compliance, SLA |

Tier enforcement via PolyKit `metering_graph` + `subscription_lifecycle` state machine.

---

## Search

Email search is entirely client-side:

1. Incoming emails are decrypted on-device
2. `poly-mail-core` builds a local encrypted search index
3. Index is PQ-encrypted and scatter-synced across devices
4. Search queries execute locally against the decrypted index
5. No search queries are sent to any server

---

## Directory Structure

```
polymail/
├── circuits/fl/
│   ├── polymail_encrypt.fl
│   ├── polymail_route.fl
│   ├── polymail_classify.fl
│   ├── polymail_metering.fl
│   ├── polymail_smtp_bridge.fl
│   └── graphs/
│       ├── polymail_mailbox_graph.fl
│       └── polymail_thread_dag.fl
├── crates/
│   ├── poly-mail-core/
│   ├── poly-smtp-bridge/
│   └── poly-sdk-backend/
├── packages/
│   ├── sdk-browser/
│   └── poly-mail-widget/
├── apps/
│   ├── desktop/          Tauri-based desktop client
│   └── mobile/           React Native with Rust FFI
├── docs/
│   └── ARCHITECTURE.md
├── CLAUDE.md
└── Cargo.toml
```

---

## Roadmap

### Phase 1: Core Email (Q2-Q3 2026)
- `mailbox_registry` graph + `email_thread` DAG
- FastLang circuits for encryption, routing, classification
- Tauri desktop client
- SPARK auth (`poly-mail-v1`)
- Poly-to-Poly encrypted email
- Client-side search

### Phase 2: Interop (Q3-Q4 2026)
- `polymail_smtp_bridge.fl` local bridge
- External email send/receive
- Custom domains with PQ-signed DKIM
- Mobile apps (iOS, Android)
- Import/migration tools (Gmail, Outlook, Exchange)

### Phase 3: Enterprise (Q1 2027)
- Enterprise admin via lex bridge (opt-in, k-of-n witness gating)
- Compliance: retention policies, legal hold
- `group_hierarchy.fl` org containment
- DLP/classification
- Enterprise SLA (99.99%)

### Phase 4: Advanced (2027+)
- ESLM smart inbox (priority sorting, auto-categorization)
- Calendar integration (Poly Calendar)
- Poly Mind integration (email corpus ingestion)
- FPGA-accelerated encryption for high-volume enterprise
