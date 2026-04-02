# PolyMail — CE + App Graph Spec

| Field | Value |
|-------|-------|
| **Version** | v0.1.0 |
| **Status** | Draft |
| **Platform** | eStream v0.22.0 |
| **Lex Namespace** | `polylabs/polymail` |

---

## Overview

PolyMail is PQ-encrypted email with SMTP/IMAP bridge. It exposes 13 FastLang circuits across four subsystems (compose, transport, intelligence, graphs). This spec covers two integration layers:

1. **App Graph** — registers the 13 modules into the Stratum module graph with typed dependency edges, cross-graph bridges to PolyKit and PolyCalendar, and governance observation edges.
2. **Cognitive Engine** — defines 3 meaning domains, 1 noise filter configuration, and 2 SME panels so the mail transport and intelligence subsystems feed operational intelligence into the CE pipeline.

### Design Goals

| Goal | Mechanism |
|------|-----------|
| Unified module registry | All 13 modules registered in a single `CsrStorage` graph via `module_graph_add_module` |
| Typed dependency tracking | `EDGE_REQUIRES` edges encode the internal dependency DAG |
| Cross-graph composability | `EDGE_BRIDGE_TO` edges to PolyKit sanitize, PolyCalendar scheduling |
| Governance observability | `EDGE_GOVERNANCE_OBSERVE` edges — one per module |
| Per-domain CE isolation | Each meaning domain scoped under `polylabs/polymail/cognitive` lex |
| Noise suppression | Newsletter noise, auto-replies filtered before CE ingestion; signal on delivery failures and encryption downgrades |

---

## App Graph — 13 Modules

### Module Inventory

| Group | Module | Partition | SLA | Role |
|-------|--------|-----------|-----|------|
| **Compose** | `polymail_rich_compose` | Head | Premium | Rich text compose with attachments, inline images, templates |
| **Transport** | `polymail_smtp_bridge` | Backend | Premium | SMTP/IMAP protocol bridge, MX resolution, TLS negotiation |
| | `polymail_route` | Backend | Premium | Inbound/outbound routing, alias resolution, forwarding rules |
| | `polymail_encrypt` | Backend | Premium | ML-KEM-1024 + ML-DSA-87 envelope encryption, S/MIME bridge |
| **Intelligence** | `polymail_classify` | Backend | Standard | Spam/phishing/ham classification, Bayesian + CE hybrid |
| | `polymail_filter` | Backend | Standard | User filter rules, label assignment, folder routing |
| | `polymail_search` | Backend | Standard | Full-text encrypted search over scatter-CAS mailbox |
| | `polymail_calendar` | Backend | Standard | Meeting invite parsing, iCal extraction, PolyCalendar bridge |
| **Platform** | `polymail_rbac` | Backend | Standard | Per-mailbox RBAC, delegate access, shared mailbox roles |
| | `polymail_metering` | Backend | Standard | 8D metering: send/receive volume, storage, bandwidth |
| | `polymail_platform_health` | Backend | Standard | Health probes, circuit liveness, SMTP uptime tracking |
| **Graphs** | `mailbox_graph` | Backend | Standard | Mailbox/Folder/Label/Contact relationship registry |
| | `thread_dag` | Backend | Standard | Thread/Message/Reply DAG with conversation threading |

### Dependency Edges (EDGE_REQUIRES)

```
polymail_rich_compose → polymail_encrypt, polymail_route, mailbox_graph
polymail_smtp_bridge → polymail_route, polymail_encrypt
polymail_route → polymail_filter, polymail_classify, mailbox_graph
polymail_encrypt → polymail_rbac
polymail_classify → polymail_filter, thread_dag
polymail_filter → mailbox_graph
polymail_search → mailbox_graph, thread_dag
polymail_calendar → polymail_filter, mailbox_graph
polymail_metering → polymail_smtp_bridge
polymail_platform_health → polymail_metering, polymail_smtp_bridge
thread_dag → mailbox_graph
```

### Bridge Edges (EDGE_BRIDGE_TO)

| Source Module | Target Lex | Target Module | Bridge Type |
|---------------|-----------|---------------|-------------|
| `polymail_metering` | `polykit/metering` | `polykit_metering` | metering_aggregation |
| `polymail_rbac` | `polykit/rbac` | `polykit_rbac` | role_composition |
| `polymail_encrypt` | `polykit/sanitize` | `polykit_sanitize` | content_sanitization |
| `polymail_calendar` | `polylabs/polycalendar` | `polycalendar_scheduling` | calendar_bridge |

---

## Cognitive Engine — 3 Domains, 1 Filter, 2 Panels

### Meaning Domains

| Domain Path | Description | Crystallization | Impact |
|-------------|-------------|-----------------|--------|
| `email/delivery_patterns` | Bounce rate trends, delivery latency, MX resolution failures, TLS downgrade frequency | 70 | 85 |
| `email/threat_detection` | Phishing pattern evolution, spam campaign detection, encryption downgrade attacks, sender reputation shifts | 80 | 95 |
| `email/productivity` | Response time distribution, thread depth patterns, unread accumulation, peak send/receive hours | 65 | 70 |

### Noise Filter

| Config | Value |
|--------|-------|
| Suppress newsletter noise | `true` (List-Unsubscribe header, bulk precedence, marketing sender patterns) |
| Suppress auto-replies | `true` (OOO, delivery receipts, read receipts, vacation responders) |
| Signal on delivery failures | `true` (5xx bounces, DMARC failures, SPF/DKIM misalignment) |
| Signal on encryption downgrades | `true` (TLS fallback, missing DANE, opportunistic-only) |
| Min signal confidence | 55 |
| Dedup window | 600,000 ms |

### SME Panels

| Panel | Domain Scope | Min Panelists | Specializations | Calibration Floor |
|-------|-------------|---------------|-----------------|-------------------|
| SMTP Bridge Reliability | `email/delivery_patterns` | 3 | smtp_protocol, mx_resolution, tls_negotiation, bounce_handling | 75.00% |
| Threat Model Evolution | `email/threat_detection` | 3 | phishing_detection, spam_evolution, sender_reputation, encryption_security | 80.00% |

---

## Strategic Grant Config

| Licensor | Grant | CE Access | Scope |
|----------|-------|-----------|-------|
| eStream (PolyQuantum) | `polylabs-estream-slg-v1` | Full CE primitives (SSM, cortex, observation) | Platform-wide |
| Paragon (PolyQuantum) | `polylabs-paragon-slg-v1` | Compliance CE overlay for email retention | Enterprise tier |

---

## Circuit Files

| File | Lines | Contents |
|------|-------|----------|
| `circuits/fl/polymail_app_graph.fl` | ~200 | 13 module definitions, graph registration, bridge edges, governance edges, golden tests |
| `circuits/fl/polymail_meaning.fl` | ~120 | 3 domain data types, noise filter config, 2 SME panel types, registration orchestrator, golden tests |

---

## Verification

| Property | Type | Assertion |
|----------|------|-----------|
| `all_modules_registered` | Safety | All 13 modules findable by name after registration |
| `graph_registration_completes` | Liveness | `num_nodes >= 13` after `polymail_app_graph_register` |
| `register_all_13_modules` | Golden test | Node count = 13, spot-check 3 modules |
| `bridge_edges_to_polykit` | Golden test | Bridge registration increases node/edge count |
| `governance_edges_all_modules` | Golden test | Governance registration adds >= 13 edges |
| `full_ce_pipeline_setup` | Golden test | All 3 domains + filter + 2 panels initialize |

---

## References

- PolyLabs CE Spec: `polylabs/specs/POLYLABS_CE_APP_GRAPH_SPEC.md`
- PolyKit CE Spec: `polykit/specs/POLYKIT_CE_APP_GRAPH_SPEC.md`
- PolyKit App Graph: `polykit/circuits/fl/polykit_app_graph.fl`
- PolyKit Cognitive: `polykit/circuits/fl/polykit_cognitive.fl`
- eStream CE Spec: `estream/specs/core/intelligence/COGNITIVE_ENGINE_SPEC.md`
