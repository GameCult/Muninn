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

    /// One wire pin per record. `bytes` is the record's positional MessagePack
    /// array, encoded from a sample whose every field holds a distinct,
    /// non-default value (Options: one `None`, one `Some`). `first_default` is
    /// the key of the first defaulted tail field; records without one carry
    /// `None`.
    struct Pin {
        record: &'static str,
        bytes: Vec<u8>,
        first_default: Option<usize>,
        golden: &'static str,
        /// Decode a payload as this record and encode it again.
        reencode: fn(&[u8]) -> Result<Vec<u8>, String>,
    }

    fn pin<T>(
        record: &'static str,
        sample: T,
        first_default: Option<usize>,
        golden: &'static str,
    ) -> Pin
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        Pin {
            record,
            bytes: rmp_serde::to_vec(&sample).unwrap(),
            first_default,
            golden,
            reencode: |bytes| {
                let decoded: T = rmp_serde::from_slice(bytes).map_err(|error| error.to_string())?;
                rmp_serde::to_vec(&decoded).map_err(|error| error.to_string())
            },
        }
    }

    fn text(value: &str) -> String {
        value.to_string()
    }

    fn texts(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn pins() -> Vec<Pin> {
        vec![
            pin(
                "MuninnTelemetrySurfaceRecord",
                MuninnTelemetrySurfaceRecord {
                    surface_id: text("surface"),
                    host_id: text("host"),
                    state: text("live"),
                    available_sources: texts(&["src"]),
                    stream_affordances: texts(&["aff"]),
                    active_streams: texts(&["act"]),
                    activation_authority: text("auth"),
                    detail: text("detail"),
                    updated_at: text("t"),
                    primary_stream_id: text("psid"),
                    primary_stream_label: text("plabel"),
                    command_rudp_target: text("cmd"),
                    media_target_host: text("mth"),
                    media_port: 4001,
                    media_packet_bytes: 1200,
                    rudp_video_bitrate_kbps: 8000,
                    rudp_latency_budget_ms: 40,
                    video_source_ids: texts(&["vid"]),
                    video_source_labels: texts(&["vlab"]),
                    audio_source_ids: texts(&["aid"]),
                    audio_source_labels: texts(&["alab"]),
                },
                Some(9),
                "dc0015a773757266616365a4686f7374a46c69766591a373726391a361666691a3616374a461757468a664657461696ca174a470736964a6706c6162656ca3636d64a36d7468cd0fa1cd04b0cd1f402891a376696491a4766c616291a361696491a4616c6162",
            ),
            pin(
                "MuninnCaptureStreamRecord",
                MuninnCaptureStreamRecord {
                    stream_id: text("stream"),
                    host_id: text("host"),
                    state: text("live"),
                    video_source: text("vsrc"),
                    audio_source: text("asrc"),
                    transport: text("rudp"),
                    targets: texts(&["tgt"]),
                    command_witness: text("wit"),
                    supervisor_pid: None,
                    mux_pid: Some(77),
                    restart_count: 3,
                    detail: text("detail"),
                    updated_at: text("t"),
                },
                None,
                "9da673747265616da4686f7374a46c697665a476737263a461737263a47275647091a3746774a3776974c04d03a664657461696ca174",
            ),
            pin(
                "MuninnCaptureStreamCommandRecord",
                MuninnCaptureStreamCommandRecord {
                    command_id: text("cmd"),
                    host_id: text("host"),
                    stream_id: text("stream"),
                    state: text("live"),
                    action: text("start"),
                    target_host: text("tgt"),
                    port: 4001,
                    obs_target_host: Some(text("obs")),
                    obs_port: 4002,
                    media_transport: text("rudp"),
                    media_packet_bytes: 1200,
                    requested_by: text("who"),
                    detail: text("detail"),
                    updated_at: text("t"),
                    rudp_video_bitrate_kbps: 8000,
                    rudp_latency_budget_ms: 40,
                    video_source_id: text("vid"),
                    audio_source_id: text("aid"),
                },
                Some(14),
                "dc0012a3636d64a4686f7374a673747265616da46c697665a57374617274a3746774cd0fa1a36f6273cd0fa2a472756470cd04b0a377686fa664657461696ca174cd1f4028a3766964a3616964",
            ),
            pin(
                "MuninnObsStreamCatalogRecord",
                MuninnObsStreamCatalogRecord {
                    catalog_id: text("catalog"),
                    host_id: text("host"),
                    stream_ids: texts(&["sid"]),
                    labels: texts(&["lab"]),
                    urls: texts(&["url"]),
                    states: texts(&["st"]),
                    updated_at: text("t"),
                    command_rudp_target: text("cmd"),
                    media_target_host: text("mth"),
                    media_port: 4001,
                    media_packet_bytes: 1200,
                    rudp_video_bitrate_kbps: 8000,
                    rudp_latency_budget_ms: 40,
                    video_source_ids: texts(&["vid"]),
                    video_source_labels: texts(&["vlab"]),
                    audio_source_ids: texts(&["aid"]),
                    audio_source_labels: texts(&["alab"]),
                },
                Some(7),
                "dc0011a7636174616c6f67a4686f737491a373696491a36c616291a375726c91a27374a174a3636d64a36d7468cd0fa1cd04b0cd1f402891a376696491a4766c616291a361696491a4616c6162",
            ),
            pin(
                "MuninnMoveMarkerCandidateRecord",
                MuninnMoveMarkerCandidateRecord {
                    stream_id: text("stream"),
                    host_id: text("host"),
                    camera_id: text("cam"),
                    frame_sequence: 11,
                    source_id_hash: 12,
                    tile_x: 13,
                    tile_y: 14,
                    center_x_px: 1.5,
                    center_y_px: 2.5,
                    radius_px: 3.5,
                    area_px: 15,
                    mean_luma: 4.5,
                    peak_luma: 16,
                    score: 0.5,
                    observed_at: text("t"),
                    move_id: text("move"),
                },
                Some(15),
                "dc0010a673747265616da4686f7374a363616d0b0c0d0eca3fc00000ca40200000ca406000000fca4090000010ca3f000000a174a46d6f7665",
            ),
            pin(
                "MuninnMoveControllerStateRecord",
                MuninnMoveControllerStateRecord {
                    stream_id: text("stream"),
                    host_id: text("host"),
                    move_id: text("move"),
                    sequence: 21,
                    source_timestamp_ns: -22,
                    accelerometer_xyz: vec![1.0, 2.0, 3.0],
                    gyroscope_xyz: vec![4.0, 5.0, 6.0],
                    magnetometer_xyz: vec![7.0, 8.0, 9.0],
                    trigger_value: 0.25,
                    buttons: texts(&["cross"]),
                    battery01: 0.75,
                    observed_at: text("t"),
                    source_path: text("/dev/js0"),
                },
                Some(12),
                "9da673747265616da4686f7374a46d6f766515ea93ca3f800000ca40000000ca4040000093ca40800000ca40a00000ca40c0000093ca40e00000ca41000000ca41100000ca3e80000091a563726f7373ca3f400000a174a82f6465762f6a7330",
            ),
            pin(
                "MuninnHidControllerStateRecord",
                MuninnHidControllerStateRecord {
                    stream_id: text("muninn.raven.hid"),
                    host_id: text("raven"),
                    device_id: text("xbox-0"),
                    device_kind: text("xinput"),
                    sequence: 7,
                    source_timestamp_ns: -3,
                    axes: vec![0.5, -1.0],
                    buttons: texts(&["a", "b"]),
                    battery01: 1.0,
                    observed_at: text("unix:100"),
                    source_path: text("xinput://0"),
                },
                None,
                "9bb06d756e696e6e2e726176656e2e686964a5726176656ea678626f782d30a678696e70757407fd92ca3f000000cabf80000092a161a162ca3f800000a8756e69783a313030aa78696e7075743a2f2f30",
            ),
            pin(
                "MuninnMoveIdentityRecord",
                MuninnMoveIdentityRecord {
                    identity_id: text("identity"),
                    host_id: text("host"),
                    move_id: text("move"),
                    source_path: text("path"),
                    bluetooth_host_address: text("bt"),
                    state: text("live"),
                    detail: text("detail"),
                    observed_at: text("t"),
                },
                None,
                "98a86964656e74697479a4686f7374a46d6f7665a470617468a26274a46c697665a664657461696ca174",
            ),
            pin(
                "MuninnMoveLightCommandRecord",
                MuninnMoveLightCommandRecord {
                    command_id: text("cmd"),
                    host_id: text("host"),
                    move_id: text("move"),
                    hidraw_path: text("hidraw"),
                    colors: texts(&["red"]),
                    durations_ms: vec![100, 200],
                    repeat_count: 5,
                    authority: text("auth"),
                    state: text("live"),
                    detail: text("detail"),
                    updated_at: text("t"),
                },
                None,
                "9ba3636d64a4686f7374a46d6f7665a668696472617791a37265649264ccc805a461757468a46c697665a664657461696ca174",
            ),
            pin(
                "MuninnMoveHueProgramRecord",
                MuninnMoveHueProgramRecord {
                    program_id: text("prog"),
                    host_id: text("raven"),
                    mode: text("cycle"),
                    cycle_ms: 20,
                    epoch_ns: -1,
                    hold_at_ns: 0,
                    requested_by: text("mimir"),
                    updated_at: text("unix:1"),
                    order_mode: text("ring"),
                    transition_percent: 50,
                    transition_percent_explicit: true,
                },
                Some(8),
                "9ba470726f67a5726176656ea56379636c6514ff00a56d696d6972a6756e69783a31a472696e6732c3",
            ),
            pin(
                "MuninnMoveTrackerHealthRecord",
                MuninnMoveTrackerHealthRecord {
                    health_id: text("health"),
                    host_id: text("host"),
                    camera_id: text("cam"),
                    camera_index: -2,
                    state: text("live"),
                    camera_name: text("name"),
                    camera_api: text("api"),
                    width: 640,
                    height: 480,
                    exposure: 0.5,
                    calibrated_controller_count: 2,
                    update_count: 30,
                    observation_count: 31,
                    latest_observation_count: 32,
                    last_observation_at: text("last"),
                    detail: text("detail"),
                    updated_at: text("t"),
                    image_mean_rgb: vec![1, 2, 3],
                    image_peak_rgb: vec![4, 5, 6],
                    color_evidence_move_ids: texts(&["move"]),
                    color_evidence_pixel_counts: vec![40],
                    rejected_stale_count: 41,
                    rejected_radius_count: 42,
                    rejected_bounds_count: 43,
                    rejected_continuity_count: 44,
                },
                Some(17),
                "dc0019a66865616c7468a4686f7374a363616dfea46c697665a46e616d65a3617069cd0280cd01e0ca3f000000021e1f20a46c617374a664657461696ca174930102039304050691a46d6f76659128292a2b2c",
            ),
            pin(
                "MuninnMoveEvidenceTransportHealthRecord",
                MuninnMoveEvidenceTransportHealthRecord {
                    health_id: text("health"),
                    host_id: text("host"),
                    stream_id: text("stream"),
                    produced_frames: 50,
                    local_ring_admissions: 51,
                    remote_handoffs: 52,
                    remote_sends: 53,
                    updated_at: text("t"),
                },
                None,
                "98a66865616c7468a4686f7374a673747265616d32333435a174",
            ),
            pin(
                "MuninnQuestAccessRecord",
                MuninnQuestAccessRecord {
                    access_id: text("access"),
                    host_id: text("host"),
                    serial: text("serial"),
                    connection_state: text("conn"),
                    product: text("product"),
                    model: text("model"),
                    device: text("device"),
                    transport_id: text("tid"),
                    input_stream_id: text("input"),
                    pose_stream_id: text("pose"),
                    video_input_stream_id: text("video"),
                    video_input_transport: text("vtransport"),
                    state: text("live"),
                    detail: text("detail"),
                    observed_at: text("t"),
                },
                None,
                "9fa6616363657373a4686f7374a673657269616ca4636f6e6ea770726f64756374a56d6f64656ca6646576696365a3746964a5696e707574a4706f7365a5766964656faa767472616e73706f7274a46c697665a664657461696ca174",
            ),
            pin(
                "MuninnCommandBoundaryCompatRecord",
                MuninnCommandBoundaryCompatRecord {
                    value: serde_json::json!({ "daemon_id": "muninn" }),
                },
                None,
                "9181a96461656d6f6e5f6964a66d756e696e6e",
            ),
            pin(
                "MuninnTransportProfileCompatRecord",
                MuninnTransportProfileCompatRecord {
                    value: serde_json::json!({ "state": "live" }),
                },
                None,
                "9181a57374617465a46c697665",
            ),
        ]
    }

    /// Every record's bytes are exactly the pinned positional array: a swapped
    /// key, a changed type or a moved field changes them.
    #[test]
    fn every_record_encodes_to_its_pinned_positional_bytes() {
        let mut wrong = Vec::new();
        for pin in pins() {
            let actual = hex(&pin.bytes);
            if actual != pin.golden {
                wrong.push(format!("{}: {actual}", pin.record));
            }
            let reencoded = (pin.reencode)(&pin.bytes)
                .unwrap_or_else(|error| panic!("{} does not decode its own bytes: {error}", pin.record));
            assert_eq!(reencoded, pin.bytes, "{} round trip", pin.record);
        }
        assert!(
            wrong.is_empty(),
            "unpinned or moved wire layouts:\n{}",
            wrong.join("\n")
        );
    }

    /// A payload written before a defaulted key existed still decodes, once
    /// per defaulted key, and every key from there on reads as its zero value.
    #[test]
    fn every_defaulted_tail_key_decodes_from_a_payload_that_ends_before_it() {
        use serde_json::Value;
        let is_zero = |value: &Value| match value {
            Value::Null => true,
            Value::Bool(flag) => !flag,
            Value::Number(number) => number.as_f64() == Some(0.0),
            Value::String(text) => text.is_empty(),
            Value::Array(items) => items.is_empty(),
            Value::Object(_) => false,
        };
        for pin in pins() {
            let Some(first_default) = pin.first_default else {
                continue;
            };
            let full: Vec<Value> = rmp_serde::from_slice(&pin.bytes).unwrap();
            for end in first_default..full.len() {
                let truncated = rmp_serde::to_vec(&full[..end]).unwrap();
                let decoded = (pin.reencode)(&truncated).unwrap_or_else(|error| {
                    panic!(
                        "{} does not decode a payload ending before key {end}: {error}",
                        pin.record
                    )
                });
                let decoded: Vec<Value> = rmp_serde::from_slice(&decoded).unwrap();
                assert_eq!(decoded.len(), full.len(), "{} ending before key {end}", pin.record);
                assert_eq!(&decoded[..end], &full[..end], "{} ending before key {end}", pin.record);
                assert!(
                    decoded[end..].iter().all(is_zero),
                    "{} ending before key {end} defaulted to {:?}",
                    pin.record,
                    &decoded[end..]
                );
            }
        }
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
