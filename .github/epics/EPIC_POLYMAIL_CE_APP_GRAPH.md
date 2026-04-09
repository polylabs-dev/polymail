# Epic: PolyMail CE + App Graph Integration

> **Repo**: `qmail`
> **Spec**: `specs/POLYMAIL_CE_APP_GRAPH_SPEC.md`
> **Priority**: P0
> **Status**: Planned

---

## Summary

Integrate the Cognitive Engine and App Graph into the PolyMail PQ-encrypted email platform. This registers all 13 modules (compose 1, transport 3, intelligence 4, platform 3, graphs 2) into the Stratum module graph, wires dependency edges plus 4 cross-graph bridges (QKit metering, RBAC, sanitize, PolyCalendar), and configures 3 CE meaning domains with noise filtering and SME panels for delivery pattern analysis and threat model evolution.

---

## Tasks

### Phase 1: App Graph Registration

- [ ] **P1.1** — Implement `qmail_app_graph.fl` with 13 module definitions and `make_qmail_module` helper
- [ ] **P1.2** — Wire `EDGE_REQUIRES` dependency edges in `qmail_app_graph_register`
- [ ] **P1.3** — Implement `qmail_register_bridge_edges` (4 bridges: QKit metering, QKit RBAC, QKit sanitize, PolyCalendar)
- [ ] **P1.4** — Implement `qmail_register_governance_edges` (13 `EDGE_GOVERNANCE_OBSERVE`)
- [ ] **P1.5** — Add safety/liveness properties and 3 golden tests for graph registration

### Phase 2: CE Meaning Domains

- [ ] **P2.1** — Define `EmailDeliveryPatternsDomain`, `EmailThreatDetectionDomain`, `EmailProductivityDomain` data types with cortex blocks
- [ ] **P2.2** — Implement `register_email_delivery_patterns_domain` circuit (threshold 70, impact 85)
- [ ] **P2.3** — Implement `register_email_threat_detection_domain` circuit (threshold 80, impact 95)
- [ ] **P2.4** — Implement `register_email_productivity_domain` circuit (threshold 65, impact 70)
- [ ] **P2.5** — Add observation streams (`delivery_pattern_obs`, `threat_detection_obs`, `productivity_obs`)

### Phase 3: Noise Filter & SME Panels

- [ ] **P3.1** — Define `PolyMailNoiseFilterConfig` with newsletter/auto-reply suppression and delivery failure signaling
- [ ] **P3.2** — Implement `configure_qmail_noise_filter` (suppress newsletters, auto-replies; signal delivery failures, encryption downgrades)
- [ ] **P3.3** — Define `SmtpBridgeReliabilityPanel` and `ThreatModelEvolutionPanel` data types
- [ ] **P3.4** — Implement `configure_smtp_bridge_reliability_panel` (4 specializations, 75% calibration floor)
- [ ] **P3.5** — Implement `configure_threat_model_evolution_panel` (4 specializations, 80% calibration floor)
- [ ] **P3.6** — Implement `qmail_register_ce` orchestrator and `full_ce_pipeline_setup` golden test

### Phase 4: Integration & Validation

- [ ] **P4.1** — Verify all 13 modules resolve after registration
- [ ] **P4.2** — Verify bridge edges connect to live QKit and PolyCalendar module graphs
- [ ] **P4.3** — Verify governance observer can read all 13 modules
- [ ] **P4.4** — Verify CE domains produce observations that flow through noise filter to cortex
- [ ] **P4.5** — Verify SME panels accept and adjudicate crystallization candidates
- [ ] **P4.6** — Run full FLIR codegen (WASM + Rust targets) on both circuit files

---

## Acceptance Criteria

1. `qmail_app_graph_register` produces a `CsrStorage` with exactly 13 nodes
2. `qmail_register_bridge_edges` adds 4 bridge nodes with `EDGE_BRIDGE_TO` edges
3. `qmail_register_governance_edges` adds 13 `EDGE_GOVERNANCE_OBSERVE` edges
4. All 3 meaning domains register with correct crystallization thresholds
5. Noise filter suppresses newsletter noise and auto-replies; signals delivery failures
6. Both SME panels require min 3 panelists with calibration floors >= 70%
7. All golden tests pass under `fl test --golden`
8. Both files compile to WASM and Rust via `fl build --target wasm,rust`

---

## Files

| File | Description |
|------|-------------|
| `circuits/fl/qmail_app_graph.fl` | 13-module app graph, edges, bridges, governance, tests |
| `circuits/fl/qmail_meaning.fl` | 3 CE domains, noise filter, 2 SME panels, orchestrator, tests |
| `specs/POLYMAIL_CE_APP_GRAPH_SPEC.md` | Integration spec |
