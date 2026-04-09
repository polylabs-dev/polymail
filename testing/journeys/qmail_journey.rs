use estream_test::{
    Journey, JourneyParty, JourneyStep, StepAction, JourneyMetrics,
    assert_metric_emitted, assert_blinded, assert_povc_witness,
};
use estream_test::convoy::{ConvoyContext, ConvoyResult};
use estream_test::stratum::{StratumVerifier, CsrTier, SeriesMerkleChain};
use estream_test::cortex::{CortexVisibility, RedactPolicy, ObfuscatePolicy};
use estream_test::mail::{SmtpBridge, ThreadDag, SpamClassifier};

pub struct PolymailJourney;

impl Journey for PolymailJourney {
    fn name(&self) -> &str {
        "qmail_e2e"
    }

    fn description(&self) -> &str {
        "End-to-end journey for Polymail: compose, PQ encrypt, SMTP bridge, receive, thread DAG, spam classification"
    }

    fn parties(&self) -> Vec<JourneyParty> {
        vec![
            JourneyParty::new("alice")
                .with_spark_context("q-mail-v1")
                .with_role("sender"),
            JourneyParty::new("bob")
                .with_spark_context("q-mail-v1")
                .with_role("recipient"),
            JourneyParty::new("charlie")
                .with_spark_context("q-mail-v1")
                .with_role("cc_recipient"),
        ]
    }

    fn steps(&self) -> Vec<JourneyStep> {
        vec![
            // Step 1: Alice composes and encrypts an email
            JourneyStep::new("alice_composes_email")
                .party("alice")
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let bob_id = ctx.party_id("bob");
                    let charlie_id = ctx.party_id("charlie");

                    let email = ctx.qmail().compose(
                        &[&bob_id],
                        &[&charlie_id],
                        "Q4 Lattice Results",
                        "Attached are the benchmark numbers for ML-KEM-1024 throughput.",
                        &[("benchmarks.csv", &ctx.generate_test_payload(4096))],
                    )?;

                    ctx.set("email_id", &email.id);
                    ctx.set("thread_id", &email.thread_id);

                    assert!(email.body_encrypted);
                    assert!(email.attachments_encrypted);
                    assert_eq!(email.kem_algo, "ml-kem-1024");
                    assert_eq!(email.recipients.len(), 2);

                    assert_metric_emitted!(ctx, "qmail.email.composed", {
                        "kem_algo" => "ml-kem-1024",
                        "attachment_count" => "1",
                    });

                    assert_povc_witness!(ctx, "qmail.compose", {
                        witness_type: "email_creation",
                        email_id: &email.id,
                    });

                    Ok(())
                }))
                .timeout_ms(8_000),

            // Step 2: PQ encryption and SMTP bridge delivery
            JourneyStep::new("smtp_bridge_delivery")
                .party("alice")
                .depends_on(&["alice_composes_email"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let email_id = ctx.get::<String>("email_id");

                    let delivery = ctx.qmail().send_via_bridge(&email_id)?;

                    assert!(delivery.smtp_handshake_ok);
                    assert!(delivery.tls_version >= "1.3");
                    assert!(delivery.pq_tunnel_active);
                    assert_eq!(delivery.recipient_count, 2);

                    // Envelope must not leak plaintext metadata
                    assert!(delivery.envelope_sender_blinded);
                    assert!(delivery.envelope_subject_encrypted);

                    ctx.set("delivery_id", &delivery.id);

                    assert_metric_emitted!(ctx, "qmail.smtp.delivered", {
                        "pq_tunnel" => "true",
                        "tls_version" => "1.3",
                    });

                    assert_blinded!(ctx, "qmail.smtp.delivered", {
                        field: "envelope_sender",
                        blinding: "hmac_sha3",
                    });

                    assert_povc_witness!(ctx, "qmail.delivery", {
                        witness_type: "smtp_bridge",
                        email_id: &email_id,
                    });

                    Ok(())
                }))
                .timeout_ms(12_000),

            // Step 3: Bob receives and decrypts the email
            JourneyStep::new("bob_receives_email")
                .party("bob")
                .depends_on(&["smtp_bridge_delivery"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let thread_id = ctx.get::<String>("thread_id");

                    let inbox = ctx.qmail().poll_inbox()?;
                    let email = inbox.find_by_thread(&thread_id)
                        .expect("Email not found in Bob's inbox");

                    let decrypted = ctx.qmail().decrypt(&email)?;

                    assert_eq!(decrypted.subject, "Q4 Lattice Results");
                    assert!(decrypted.body.contains("ML-KEM-1024 throughput"));
                    assert_eq!(decrypted.attachments.len(), 1);
                    assert_eq!(decrypted.attachments[0].name, "benchmarks.csv");
                    assert!(decrypted.signature_valid);

                    assert_metric_emitted!(ctx, "qmail.email.received", {
                        "decrypted" => "true",
                        "signature_valid" => "true",
                    });

                    assert_blinded!(ctx, "qmail.email.received", {
                        field: "recipient_id",
                        blinding: "hmac_sha3",
                    });

                    Ok(())
                }))
                .timeout_ms(10_000),

            // Step 4: Verify thread DAG integrity
            JourneyStep::new("verify_thread_dag")
                .party("bob")
                .depends_on(&["bob_receives_email"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let thread_id = ctx.get::<String>("thread_id");

                    // Bob replies to build a thread
                    let reply = ctx.qmail().reply(
                        &thread_id,
                        "Thanks — numbers look great. Will share with the board.",
                    )?;

                    ctx.set("reply_id", &reply.id);

                    let dag = ctx.qmail().thread_dag(&thread_id)?;

                    assert!(dag.is_valid_dag());
                    assert_eq!(dag.node_count(), 2); // original + reply
                    assert!(dag.root().id == ctx.get::<String>("email_id"));
                    assert!(dag.edges_valid());

                    for node in dag.nodes() {
                        assert!(node.pq_signed);
                        assert!(node.hash_chain_valid);
                    }

                    assert_metric_emitted!(ctx, "qmail.thread.dag_verified", {
                        "node_count" => "2",
                        "dag_valid" => "true",
                    });

                    assert_povc_witness!(ctx, "qmail.thread", {
                        witness_type: "dag_integrity",
                        thread_id: &thread_id,
                    });

                    Ok(())
                }))
                .timeout_ms(10_000),

            // Step 5: Spam classification on inbound messages
            JourneyStep::new("spam_classification")
                .party("charlie")
                .depends_on(&["verify_thread_dag"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let inbox = ctx.qmail().poll_inbox()?;
                    let thread_id = ctx.get::<String>("thread_id");

                    let email = inbox.find_by_thread(&thread_id)
                        .expect("Email not found in Charlie's inbox");

                    let classification = ctx.qmail().classify_spam(&email)?;

                    assert_eq!(classification.verdict, "ham");
                    assert!(classification.confidence > 0.95);
                    assert!(classification.ran_locally, "Spam classification must run on-device");

                    // Classification must not leak content to any server
                    assert_blinded!(ctx, "qmail.spam.classified", {
                        field: "email_body",
                        blinding: "absent",
                    });

                    assert_blinded!(ctx, "qmail.spam.classified", {
                        field: "email_subject",
                        blinding: "absent",
                    });

                    assert_metric_emitted!(ctx, "qmail.spam.classified", {
                        "verdict" => "ham",
                        "local_only" => "true",
                    });

                    Ok(())
                }))
                .timeout_ms(8_000),

            // Step 6: Stratum storage verification
            JourneyStep::new("verify_stratum_storage")
                .party("alice")
                .depends_on(&["spam_classification"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let email_id = ctx.get::<String>("email_id");
                    let thread_id = ctx.get::<String>("thread_id");

                    let stratum = StratumVerifier::new(ctx);

                    let csr = stratum.verify_csr_tiers(&email_id)?;
                    assert!(csr.tier_matches(CsrTier::Warm));
                    assert!(csr.shard_distribution_valid);
                    assert!(csr.erasure_recoverable);

                    let merkle = stratum.verify_series_merkle_chain(&thread_id)?;
                    assert!(merkle.chain_intact);
                    assert!(merkle.root_hash_valid);
                    assert!(merkle.series_count >= 2);

                    assert_metric_emitted!(ctx, "qmail.stratum.verified", {
                        "csr_tier" => "warm",
                        "chain_intact" => "true",
                    });

                    Ok(())
                }))
                .timeout_ms(10_000),

            // Step 7: Verify blind telemetry and Cortex visibility
            JourneyStep::new("verify_blind_telemetry")
                .party("alice")
                .depends_on(&["verify_stratum_storage"])
                .action(StepAction::Execute(|ctx: &mut ConvoyContext| {
                    let telemetry = ctx.streamsight().drain_telemetry("q-mail-v1");

                    for event in &telemetry {
                        assert_blinded!(ctx, &event.event_type, {
                            field: "user_id",
                            blinding: "hmac_sha3",
                        });

                        assert_blinded!(ctx, &event.event_type, {
                            field: "email_body",
                            blinding: "absent",
                        });

                        assert_blinded!(ctx, &event.event_type, {
                            field: "attachment_content",
                            blinding: "absent",
                        });
                    }

                    let cortex = CortexVisibility::new(ctx);
                    cortex.assert_redacted("qmail", RedactPolicy::ContentFields)?;
                    cortex.assert_obfuscated("qmail", ObfuscatePolicy::PartyIdentifiers)?;

                    assert!(telemetry.len() >= 6, "Expected at least 6 telemetry events");

                    for event in &telemetry {
                        assert!(
                            event.namespace.starts_with("q-mail-v1"),
                            "Telemetry leaked outside q-mail-v1 namespace: {}",
                            event.namespace
                        );
                    }

                    Ok(())
                }))
                .timeout_ms(5_000),
        ]
    }

    fn metrics(&self) -> JourneyMetrics {
        JourneyMetrics {
            expected_events: vec![
                "qmail.email.composed",
                "qmail.smtp.delivered",
                "qmail.email.received",
                "qmail.thread.dag_verified",
                "qmail.spam.classified",
                "qmail.stratum.verified",
            ],
            max_duration_ms: 75_000,
            required_povc_witnesses: 4,
            lex_namespace: "q-mail-v1",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use estream_test::convoy::ConvoyRunner;

    #[tokio::test]
    async fn run_qmail_journey() {
        let runner = ConvoyRunner::new()
            .with_smtp_bridge()
            .with_streamsight("q-mail-v1")
            .with_stratum()
            .with_cortex();

        runner.run(PolymailJourney).await.expect("Polymail journey failed");
    }
}
