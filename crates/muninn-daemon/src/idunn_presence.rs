//! Idunn runtime presence: what an Idunn-launched `serve` says about itself.
//!
//! When Idunn's host actuator starts `serve`, it hands over a runtime bundle
//! (the Expected incarnation and the activation Idunn issued), a one-shot
//! activation credential, and the path of this host's provider identity.
//! From those, `serve` publishes a dual-signed
//! `gamecult.runtime_presence_health.v2` record to Odin every few seconds.
//! Odin correlates it against Idunn's Expected projection and that
//! correlation is what admits the incarnation, fences the previous one, and
//! keeps this one alive. A `serve` started by an operator has no bundle and
//! publishes nothing here.
//!
//! The record shape and both signatures are CultLib's; this module only
//! carries Muninn's inputs to them.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, ensure};
use cultcache_rs::{DatabaseEntry, SingleFileMessagePackBackingStore};
use cultnet_rs::{
    GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA, GameCultProviderHealthIdentity,
    GameCultRuntimeCapability, GameCultRuntimePresenceHealthPurpose,
    GameCultRuntimePresenceHealthRecord, IDUNN_EXPECTED_INCARNATION_SCHEMA,
    IDUNN_RUNTIME_ACTIVATION_SCHEMA, IdunnExpectedIncarnationRecord, IdunnRuntimeActivationRecord,
    IdunnRuntimeActivationSigner, ServiceIdentitySigner, open_service_identity_at,
};

pub const RUNTIME_BUNDLE_ENVIRONMENT: &str = "GAMECULT_IDUNN_RUNTIME_BUNDLE";
pub const ACTIVATION_CREDENTIAL_ENVIRONMENT: &str = "GAMECULT_IDUNN_ACTIVATION_CREDENTIAL";
pub const PROVIDER_IDENTITY_ENVIRONMENT: &str = "GAMECULT_RUNTIME_PRESENCE_IDENTITY";

/// Everything an Idunn-launched process needs to sign its presence.
pub struct IdunnRuntimeAuthority {
    expected: IdunnExpectedIncarnationRecord,
    expected_sha256: String,
    activation: IdunnRuntimeActivationRecord,
    activation_sha256: String,
    activation_signer: IdunnRuntimeActivationSigner,
    provider_signer: ServiceIdentitySigner<GameCultProviderHealthIdentity>,
    publisher_sequence: u64,
}

impl IdunnRuntimeAuthority {
    /// `None` when this process was not started by Idunn.
    pub fn from_environment() -> Result<Option<Self>> {
        let Some(bundle) = env::var_os(RUNTIME_BUNDLE_ENVIRONMENT) else {
            return Ok(None);
        };
        let bundle = PathBuf::from(bundle);
        let credential = PathBuf::from(
            env::var_os(ACTIVATION_CREDENTIAL_ENVIRONMENT)
                .with_context(|| format!("{RUNTIME_BUNDLE_ENVIRONMENT} is set but {ACTIVATION_CREDENTIAL_ENVIRONMENT} is not"))?,
        );
        let provider_identity =
            PathBuf::from(env::var_os(PROVIDER_IDENTITY_ENVIRONMENT).with_context(|| {
                format!(
                    "{RUNTIME_BUNDLE_ENVIRONMENT} is set but {PROVIDER_IDENTITY_ENVIRONMENT} is not"
                )
            })?);
        Self::open(&bundle, &credential, &provider_identity).map(Some)
    }

    pub fn open(bundle: &Path, credential: &Path, provider_identity: &Path) -> Result<Self> {
        ensure!(bundle.is_absolute(), "Idunn runtime bundle is not absolute");
        let (expected_key, expected_payload) = read_single_runtime_record(
            &bundle.join("expected.cc"),
            IdunnExpectedIncarnationRecord::TYPE,
            IDUNN_EXPECTED_INCARNATION_SCHEMA,
        )?;
        let expected = IdunnExpectedIncarnationRecord::decode_canonical(&expected_payload)?;
        ensure!(
            expected_key == expected.target,
            "Expected key is substituted"
        );
        let (activation_key, activation_payload) = read_single_runtime_record(
            &bundle.join("activation.cc"),
            IdunnRuntimeActivationRecord::TYPE,
            IDUNN_RUNTIME_ACTIVATION_SCHEMA,
        )?;
        let activation = IdunnRuntimeActivationRecord::decode_canonical(&activation_payload)?;
        ensure!(
            activation_key == expected.target,
            "activation key is substituted"
        );
        let expected_sha256 = expected.canonical_sha256()?;
        ensure!(
            activation.expected_projection_sha256 == expected_sha256
                && activation.runtime_id == expected.runtime_id,
            "activation does not bind this Expected projection"
        );
        let seed = fs::read(credential)
            .with_context(|| format!("reading activation credential {}", credential.display()))?;
        let activation_signer =
            IdunnRuntimeActivationSigner::from_credential_reader(seed.as_slice())?;
        // One-shot: the seed lives in this process now and nowhere else.
        fs::remove_file(credential)
            .with_context(|| format!("retiring activation credential {}", credential.display()))?;
        ensure!(
            activation_signer.identity_id() == activation.activation_signer_identity_id
                && activation_signer.public_key() == activation.activation_signer_public_key,
            "activation credential does not belong to this activation"
        );
        let provider_signer =
            open_service_identity_at::<GameCultProviderHealthIdentity>(provider_identity)
                .context("opening the runtime presence identity")?;
        ensure!(
            provider_signer.entry().identity_id == expected.expected_signer_identity_id,
            "provider identity is not the signer selected by Expected"
        );
        Ok(Self {
            expected_sha256,
            activation_sha256: activation.canonical_sha256()?,
            expected,
            activation,
            activation_signer,
            provider_signer,
            publisher_sequence: 0,
        })
    }

    pub fn target(&self) -> &str {
        &self.expected.target
    }

    pub fn runtime_id(&self) -> &str {
        &self.expected.runtime_id
    }

    pub fn runtime_instance_id(&self) -> &str {
        &self.activation.runtime_instance_id
    }

    /// The next presence record, signed by the provider identity and the
    /// activation. `warming` before the daemon can serve, `active` once it
    /// can, `failed` with the reason when it cannot.
    pub fn presence(
        &mut self,
        state: &str,
        detail: &str,
        observed_at_unix_millis: u64,
    ) -> Result<GameCultRuntimePresenceHealthRecord> {
        ensure!(
            self.expected.write_lease_required == false,
            "Muninn declares no process-bound state; a write lease cannot be presented"
        );
        // Odin admits a provider's presence only with a publisher sequence
        // above the last one it stored for this signer, across restarts. This
        // process cannot read Odin's store, so the sequence is the clock:
        // strictly increasing here, and above anything an earlier incarnation
        // of the same signer can have published.
        self.publisher_sequence = observed_at_unix_millis.max(
            self.publisher_sequence
                .checked_add(1)
                .ok_or_else(|| anyhow!("runtime presence publisher sequence exhausted"))?,
        );
        let expected = &self.expected;
        let mut record = GameCultRuntimePresenceHealthRecord {
            schema_version: GAMECULT_RUNTIME_PRESENCE_HEALTH_SCHEMA.into(),
            target: expected.target.clone(),
            expected_projection_sha256: self.expected_sha256.clone(),
            plan_id: expected.plan_id.clone(),
            incarnation_id: expected.incarnation_id.clone(),
            sealed_release_id: expected.sealed_release_id.clone(),
            activation_witness_sha256: self.activation_sha256.clone(),
            state_schema_generation: expected.state_schema_generation.clone(),
            state_contract_sha256: expected.state_contract_sha256.clone(),
            runtime_id: expected.runtime_id.clone(),
            runtime_instance_id: self.activation.runtime_instance_id.clone(),
            bound_endpoint: expected
                .route
                .as_ref()
                .map(|route| route.candidate_endpoint.clone()),
            capabilities: expected
                .capabilities
                .iter()
                .map(|capability| GameCultRuntimeCapability {
                    capability: capability.capability.clone(),
                    schema: capability.schema.clone(),
                    compatibility: capability.compatibility.clone(),
                    capacity: capability.minimum_capacity,
                })
                .collect(),
            health_contract: expected.health_contract.clone(),
            state: state.into(),
            detail: detail.into(),
            write_lease_sha256: None,
            signer_identity_id: self.provider_signer.entry().identity_id.clone(),
            publisher_sequence: self.publisher_sequence,
            observed_at_unix_millis,
            signature_algorithm: "ed25519".into(),
            signature: Vec::new(),
            activation_signer_identity_id: self.activation.activation_signer_identity_id.clone(),
            activation_signature: Vec::new(),
        };
        let proof_payload = record.canonical_proof_payload()?;
        record.signature = self
            .provider_signer
            .sign::<GameCultRuntimePresenceHealthPurpose>(&proof_payload)
            .signature;
        record.activation_signature = self.activation_signer.sign_presence_proof(&record)?;
        record.validate()?;
        Ok(record)
    }
}

fn read_single_runtime_record(
    path: &Path,
    record_type: &str,
    schema: &str,
) -> Result<(String, Vec<u8>)> {
    let entries = SingleFileMessagePackBackingStore::new(path)
        .pull_all_read_only_snapshot()
        .with_context(|| format!("reading runtime record {}", path.display()))?;
    let [envelope] = entries.as_slice() else {
        anyhow::bail!(
            "runtime record {} holds {} entries, expected one",
            path.display(),
            entries.len()
        );
    };
    ensure!(
        envelope.r#type == record_type && envelope.schema_id.as_deref() == Some(schema),
        "runtime record {} is not a {record_type} under {schema}",
        path.display()
    );
    Ok((envelope.key.clone(), envelope.payload.clone()))
}
