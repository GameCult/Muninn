//! Muninn's typed state records: the wire and store contract that `muninn` and
//! `sleipnir` share. Library only: no sockets, threads, devices or Odin.

mod records;

pub use records::*;

cultmesh_rs::cultmesh_documents!(MuninnRecords {
    MuninnTelemetrySurfaceRecord => MUNINN_TELEMETRY_SURFACE_SCHEMA,
    MuninnCaptureStreamRecord => MUNINN_CAPTURE_STREAM_SCHEMA,
    MuninnCaptureStreamCommandRecord => MUNINN_CAPTURE_STREAM_COMMAND_SCHEMA,
    MuninnObsStreamCatalogRecord => MUNINN_OBS_STREAM_CATALOG_SCHEMA,
    MuninnMoveMarkerCandidateRecord => MUNINN_MOVE_MARKER_CANDIDATE_SCHEMA,
    MuninnMoveControllerStateRecord => MUNINN_MOVE_CONTROLLER_STATE_SCHEMA,
    MuninnHidControllerStateRecord => MUNINN_HID_CONTROLLER_STATE_SCHEMA,
    MuninnMoveIdentityRecord => MUNINN_MOVE_IDENTITY_SCHEMA,
    MuninnMoveLightCommandRecord => MUNINN_MOVE_LIGHT_COMMAND_SCHEMA,
    MuninnMoveHueProgramRecord => MUNINN_MOVE_HUE_PROGRAM_SCHEMA,
    MuninnMoveTrackerHealthRecord => MUNINN_MOVE_TRACKER_HEALTH_SCHEMA,
    MuninnMoveEvidenceTransportHealthRecord => MUNINN_MOVE_EVIDENCE_TRANSPORT_HEALTH_SCHEMA,
    MuninnQuestAccessRecord => MUNINN_QUEST_ACCESS_SCHEMA,
    MuninnCommandBoundaryCompatRecord => MUNINN_COMMAND_BOUNDARY_SCHEMA,
    MuninnTransportProfileCompatRecord => MUNINN_TRANSPORT_PROFILE_SCHEMA,
});

#[cfg(test)]
mod tests {
    use super::*;
    use cultcache_rs::{CultCache, DatabaseEntry};
    use cultmesh_rs::CultMeshDocumentSet;
    use cultnet_rs::CultNetDocumentRegistry;

    /// Every Muninn record: its type name, its schema id, the schema constant
    /// the document set binds it under, and its Rust name.
    fn contract() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
        macro_rules! row {
            ($record:ty, $constant:expr) => {
                (
                    <$record as DatabaseEntry>::TYPE,
                    <$record as DatabaseEntry>::SCHEMA_NAME,
                    $constant,
                    stringify!($record),
                )
            };
        }
        vec![
            row!(MuninnTelemetrySurfaceRecord, MUNINN_TELEMETRY_SURFACE_SCHEMA),
            row!(MuninnCaptureStreamRecord, MUNINN_CAPTURE_STREAM_SCHEMA),
            row!(MuninnCaptureStreamCommandRecord, MUNINN_CAPTURE_STREAM_COMMAND_SCHEMA),
            row!(MuninnObsStreamCatalogRecord, MUNINN_OBS_STREAM_CATALOG_SCHEMA),
            row!(MuninnMoveMarkerCandidateRecord, MUNINN_MOVE_MARKER_CANDIDATE_SCHEMA),
            row!(MuninnMoveControllerStateRecord, MUNINN_MOVE_CONTROLLER_STATE_SCHEMA),
            row!(MuninnHidControllerStateRecord, MUNINN_HID_CONTROLLER_STATE_SCHEMA),
            row!(MuninnMoveIdentityRecord, MUNINN_MOVE_IDENTITY_SCHEMA),
            row!(MuninnMoveLightCommandRecord, MUNINN_MOVE_LIGHT_COMMAND_SCHEMA),
            row!(MuninnMoveHueProgramRecord, MUNINN_MOVE_HUE_PROGRAM_SCHEMA),
            row!(MuninnMoveTrackerHealthRecord, MUNINN_MOVE_TRACKER_HEALTH_SCHEMA),
            row!(
                MuninnMoveEvidenceTransportHealthRecord,
                MUNINN_MOVE_EVIDENCE_TRANSPORT_HEALTH_SCHEMA
            ),
            row!(MuninnQuestAccessRecord, MUNINN_QUEST_ACCESS_SCHEMA),
            row!(MuninnCommandBoundaryCompatRecord, MUNINN_COMMAND_BOUNDARY_SCHEMA),
            row!(MuninnTransportProfileCompatRecord, MUNINN_TRANSPORT_PROFILE_SCHEMA),
        ]
    }

    #[test]
    fn record_names_are_the_published_schema_ids() {
        let expected = [
            ("muninn.telemetry_surface", "muninn.telemetry_surface.v1"),
            ("muninn.capture_stream", "muninn.capture_stream.v1"),
            ("muninn.capture_stream_command", "muninn.capture_stream_command.v1"),
            ("muninn.obs_stream_catalog", "muninn.obs_stream_catalog.v1"),
            ("muninn.move_marker_candidate", "muninn.move_marker_candidate.v1"),
            ("muninn.move_controller_state", "muninn.move_controller_state.v1"),
            ("muninn.hid_controller_state", "muninn.hid_controller_state.v1"),
            ("muninn.move_identity", "muninn.move_identity.v1"),
            ("muninn.move_light_command", "muninn.move_light_command.v1"),
            ("muninn.move_hue_program", "muninn.move_hue_program.v1"),
            ("muninn.move_tracker_health", "muninn.move_tracker_health.v1"),
            (
                "muninn.move_evidence_transport_health",
                "muninn.move_evidence_transport_health.v1",
            ),
            ("muninn.quest_access", "muninn.quest_access.v1"),
            ("muninn.command_boundary", "muninn.command_boundary.v1"),
            ("muninn.transport_profile", "muninn.transport_profile.v1"),
        ];
        let actual: Vec<_> = contract()
            .into_iter()
            .map(|(record_type, schema, constant, _)| {
                assert_eq!(schema, constant, "schema constant and derive disagree");
                (record_type, schema)
            })
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn document_set_binds_every_record_under_its_schema() {
        let mut cache = CultCache::new();
        MuninnRecords.register_cache(&mut cache).unwrap();
        let mut registry = CultNetDocumentRegistry::new();
        MuninnRecords.register_documents(&mut registry).unwrap();

        let rows = contract();
        let mut registered = cache.registered_entry_types();
        registered.sort();
        let mut expected: Vec<String> = rows.iter().map(|row| row.0.to_string()).collect();
        expected.sort();
        assert_eq!(registered, expected);
        for (record_type, schema, _, record) in rows {
            let binding = registry
                .binding_by_schema_id(schema)
                .unwrap_or_else(|| panic!("{record} is not bound under {schema}"));
            assert_eq!(binding.document_type, record_type, "{record}");
        }
    }

    #[test]
    fn hid_controller_state_wire_layout_is_positional_by_key() {
        let record = MuninnHidControllerStateRecord {
            stream_id: "muninn.raven.hid".into(),
            host_id: "raven".into(),
            device_id: "xbox-0".into(),
            device_kind: "xinput".into(),
            sequence: 7,
            source_timestamp_ns: -3,
            axes: vec![0.5, -1.0],
            buttons: vec!["a".into(), "b".into()],
            battery01: 1.0,
            observed_at: "unix:100".into(),
            source_path: "xinput://0".into(),
        };
        let bytes = rmp_serde::to_vec(&record).unwrap();
        assert_eq!(
            hex(&bytes),
            "9bb06d756e696e6e2e726176656e2e686964a5726176656ea678626f782d30a678696e70757407fd92ca3f000000cabf80000092a161a162ca3f800000a8756e69783a313030aa78696e7075743a2f2f30"
        );
        assert_eq!(
            rmp_serde::from_slice::<MuninnHidControllerStateRecord>(&bytes).unwrap(),
            record
        );
    }

    #[test]
    fn hue_program_wire_layout_carries_its_defaulted_tail() {
        let record = MuninnMoveHueProgramRecord {
            program_id: "prog".into(),
            host_id: "raven".into(),
            mode: "cycle".into(),
            cycle_ms: 20,
            epoch_ns: -1,
            hold_at_ns: 0,
            requested_by: "mimir".into(),
            updated_at: "unix:1".into(),
            order_mode: "ring".into(),
            transition_percent: 50,
            transition_percent_explicit: true,
        };
        let bytes = rmp_serde::to_vec(&record).unwrap();
        assert_eq!(
            hex(&bytes),
            "9ba470726f67a5726176656ea56379636c6514ff00a56d696d6972a6756e69783a31a472696e6732c3"
        );
        assert_eq!(
            rmp_serde::from_slice::<MuninnMoveHueProgramRecord>(&bytes).unwrap(),
            record
        );
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
