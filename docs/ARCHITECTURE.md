# Poly Mail Architecture

**Version**: 2.0
**Date**: February 2026
**Platform**: eStream v0.9.1
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

Accounts, folders, labels, and contacts form a Stratum graph with Cortex AI governance. Node types use `data` declarations with `store graph` and field-level visibility controls. Replaces flat mailbox tables with a relational model supporting hierarchical folders, multi-label assignment, contact resolution, quota enforcement, and folder sharing.

```fastlang
data MailboxNode : app v1 {
    mailbox_id: bytes(16),
    owner_id: bytes(16),
    display_name: bytes(128),
    domain: bytes(253),
    created_at: u64,
    quota_bytes: u64,
    tier: u8,
}
    store graph
    govern lex esn/global/org/polylabs/mail
    cortex {
        obfuscate [owner_id]
        infer on_write
        on_anomaly alert "mail-team"
    }

data FolderNode : app v1 { ... }
    store graph
    govern lex esn/global/org/polylabs/mail
    cortex { infer on_write }

data LabelNode : app v1 { ... }
    store graph
    govern lex esn/global/org/polylabs/mail
    cortex { infer on_read }

data MailContactNode : app v1 {
    contact_id: bytes(16),
    user_id: bytes(16),
    display_name: bytes(128),
    email_address: bytes(254),
    signing_pubkey: bytes(2592),
    encryption_pubkey: bytes(1568),
    is_poly_user: bool,
    last_contacted_at: u64,
    trust_score: f32,
}
    store graph
    govern lex esn/global/org/polylabs/mail
    cortex {
        redact [email_address]
        obfuscate [display_name, user_id]
        infer on_write
        on_anomaly alert "mail-security"
    }

graph mailbox_registry {
    node MailboxNode
    node FolderNode
    node LabelNode
    node MailContactNode
    edge ContainsEdge
    edge LabeledWithEdge
    edge SentToEdge
    edge SharedWithEdge

    overlay unread_count: u32 bitmask delta_curate
    overlay folder_size: u64 bitmask delta_curate
    overlay spam_score: f32 curate delta_curate
    overlay thread_count: u32 bitmask delta_curate
    overlay contact_frequency: u32 bitmask delta_curate
    overlay storage_used: u64 bitmask delta_curate

    storage csr {
        hot @bram,
        warm @ddr,
        cold @nvme,
    }

    ai_feed mailbox_anomaly
    ai_feed contact_trust_scoring

    observe mailbox_registry: [unread_count, spam_score, thread_count, storage_used] threshold: {
        anomaly_score 0.85
        baseline_window 120
    }
}

series mailbox_series: mailbox_registry
    merkle_chain true
    lattice_imprint true
    witness_attest true
```

Key circuits: `create_mailbox`, `create_folder`, `move_to_folder`, `create_label`, `apply_label`, `add_contact`, `resolve_contact`, `share_folder`, `check_quota`, `snapshot_mailbox`.

### Email Thread DAG (`polymail_thread_dag.fl`)

Emails form a Stratum DAG with Cortex AI governance, ML-DSA-87 signing, merkle CSR storage, and PoVC attestation. Replies create parent edges. Forwards branch. Cortex classifies incoming email on write (spam, phishing, priority) and stores suggestions for inbox intelligence.

```fastlang
data EmailNode : app v1 {
    email_id: bytes(16),
    thread_id: bytes(16),
    sender_id: bytes(16),
    recipient_ids: list<bytes(16)>,
    subject_preview: bytes(256),
    content_hash: bytes(32),
    timestamp: u64,
    in_reply_to: bytes(16),
    attachment_count: u8,
    size_bytes: u64,
    scatter_manifest: bytes(64),
    pq_signature: bytes(4627),
}
    store dag
    govern lex esn/global/org/polylabs/mail
    cortex {
        redact [content_hash, subject_preview]
        obfuscate [sender_id, recipient_ids]
        infer on_write
        on_anomaly alert "mail-security"
        on_classification auto_apply
        on_suggestion store "cortex/mail/suggestions"
    }

state_machine email_lifecycle {
    initial DRAFT
    persistence wal
    terminal [SPAM, ARCHIVED, DELETED, PURGED]
    li_anomaly_detection true

    DRAFT -> QUEUED when user_send
    QUEUED -> SENDING when relay_picked_up
    SENDING -> SENT when relay_confirmed
    SENDING -> BOUNCED when send_failed guard retry_limit_reached
    SENT -> DELIVERED when recipient_acked
    DELIVERED -> READ when recipient_read
    ... (full transitions in .fl source)
    DELETED -> PURGED when retention_expired guard past_retention_window
}

dag email_thread {
    node EmailNode
    edge ReplyToEdge
    edge ForwardEdge
    edge AttachmentRefEdge

    enforce acyclic
    sign ml_dsa_87

    overlay read_status: u8 curate delta_curate
    overlay star: bool curate delta_curate
    overlay label_mask: u64 bitmask delta_curate
    overlay spam_verdict: u8 curate delta_curate
    overlay priority_score: f32 curate delta_curate
    overlay attachment_count: u8 bitmask delta_curate

    storage merkle_csr {
        hot @bram,
        warm @ddr,
        cold @nvme,
    }

    attest povc {
        witness threshold(2, 3)
    }

    ai_feed email_classification
    ai_feed thread_priority_scoring
    ai_feed phishing_detection

    observe email_thread: [read_status, label_mask, spam_verdict, priority_score] threshold: {
        anomaly_score 0.8
        baseline_window 60
    }
}

series email_series: email_thread
    merkle_chain true
    lattice_imprint true
    witness_attest true
```

Key circuits: `compose_draft`, `send_email`, `receive_email`, `mark_read`, `star_email`, `set_label_mask`, `archive_email`, `delete_email`, `move_to_spam`, `mark_not_spam`, `forward_email`, `get_thread`, `get_thread_participants`, `get_cortex_suggestions`, `apply_cortex_suggestion`, `snapshot_thread`.

---

## Stratum & Cortex Integration

Poly Mail fully composes Stratum storage and Cortex AI governance via the v0.10.0 `data` declaration pattern (`store graph/dag`, `govern lex`, `cortex {}`).

### Stratum Storage Bindings

| Construct | Storage Type | Purpose |
|-----------|-------------|---------|
| `mailbox_registry` | `store graph` + CSR tiered | Accounts, folders, labels, contacts. Hot overlays (unread_count, spam_score) in BRAM for sub-microsecond reads. Warm node data in DDR. Cold archived mailboxes on NVMe. |
| `email_thread` | `store dag` + merkle CSR | Conversation threading with acyclic enforcement. Merkle CSR provides tamper-evident storage. ML-DSA-87 signing on every DAG mutation. PoVC attestation with 2-of-3 witness threshold. |
| Email content blobs | KV (scatter-CAS) | Encrypted email bodies stored via scatter-CAS (`scatter_manifest` on EmailNode). Content-addressed, erasure-coded across providers. Not in the graph/DAG — referenced by `content_hash`. |

**Tiering policy**: All graph/DAG storage uses `{ hot @bram, warm @ddr, cold @nvme }`. Overlay data (counters, scores, bitmasks) stays hot. Node/edge structural data is warm. Nodes in terminal states (DELETED, PURGED, ARCHIVED >90d) migrate to cold.

### Cortex Visibility Policies

Field-level visibility is enforced per data type via Cortex declarations. These policies apply to all consumers (StreamSight, ai_feed models, audit, lex governance) — raw field values are never exposed beyond the policy.

| Data Type | Redacted Fields | Obfuscated Fields | Exposed Fields |
|-----------|----------------|-------------------|---------------|
| `MailboxNode` | — | `owner_id` | `mailbox_id`, `domain`, `tier`, `created_at`, `quota_bytes` |
| `FolderNode` | — | — | All fields (structural data only) |
| `LabelNode` | — | — | All fields (structural data only) |
| `MailContactNode` | `email_address` | `display_name`, `user_id` | `contact_id`, `is_poly_user`, `trust_score`, `signing_pubkey`, `encryption_pubkey` |
| `EmailNode` | `content_hash`, `subject_preview` | `sender_id`, `recipient_ids` | `email_id`, `thread_id`, `timestamp`, `attachment_count`, `size_bytes` |

**Redact** = field zeroed in all non-owner contexts (governance, audit, AI feeds see zeros).
**Obfuscate** = field replaced with a deterministic pseudonym (consistent within a session, unlinkable across sessions).
**Expose** = field visible to all authorized consumers.

### Cortex Inference Triggers & Feedback

| Trigger | Data Type | When | Action |
|---------|-----------|------|--------|
| `infer on_write` | `MailboxNode` | Mailbox created/updated | Anomaly detection on creation patterns (burst creation = abuse signal) |
| `infer on_write` | `FolderNode` | Folder created | Folder hierarchy depth check |
| `infer on_read` | `LabelNode` | Label queried | Auto-apply rule evaluation against incoming emails |
| `infer on_write` | `MailContactNode` | Contact added/updated | Trust score computation, anomaly alert on trust_score drop |
| `infer on_write` | `EmailNode` | Email received/composed | Full classification pipeline (see below) |
| `on_classification auto_apply` | `EmailNode` | After `infer on_write` | Spam verdict written to `spam_verdict` overlay; SPAM state transition if verdict is Spam/Phishing/Malware |
| `on_suggestion store` | `EmailNode` | After classification | Priority suggestions, label suggestions, reply suggestions stored to `cortex/mail/suggestions` |
| `on_anomaly alert` | `MailboxNode`, `MailContactNode`, `EmailNode` | Anomaly score exceeds threshold | Alert to `mail-team` or `mail-security` via StreamSight |

**Classification pipeline** (on `receive_email`):
1. `cortex_classify()` runs ESLM spam model → `spam_verdict` overlay
2. If verdict is Spam/Phishing/Malware → auto-transition to SPAM state
3. `cortex_score()` runs priority model → `priority_score` overlay
4. Suggestions (label, reply, priority) written to `cortex/mail/suggestions`
5. User actions (`mark_not_spam`, `apply_cortex_suggestion`) feed back via `cortex_feedback()` for model improvement

**Feedback loop**: Every user correction (not-spam, accepted/rejected suggestion) calls `cortex_feedback()` or `cortex_feedback_suggestion()`, which updates the local ESLM model weights via federated learning. No raw email content is shared — only verdict labels and confidence scores.

### Quantum State Snapshots (.q)

Both constructs support `.q` quantum state snapshots for mailbox backup, migration, and disaster recovery:

- **`snapshot_mailbox`**: Extracts the full subgraph rooted at a mailbox (3-hop depth: mailbox → folders/labels/contacts → edges), serializes all overlays, Blake3-hashes the state, and appends to `mailbox_series`. The hash is the `.q` state identifier.
- **`snapshot_thread`**: Extracts the full sub-DAG for a thread, serializes overlays + state machine states for all emails in the thread, Blake3-hashes, and appends to `email_series`.
- **Migration**: To migrate a mailbox, export all `.q` snapshots for the mailbox + its threads, transfer to the target instance, and replay from the series. Merkle chain verification ensures integrity. Lattice imprint + witness attestation prove provenance.
- **Backup cadence**: Snapshots are taken on-demand (user-triggered export) and on significant state changes (>100 mutations since last snapshot). Series retention follows tier policy.

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
