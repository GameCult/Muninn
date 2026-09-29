use cultcache_rs::DatabaseEntry;
use serde_json::Value;

pub const MUNINN_TELEMETRY_SURFACE_SCHEMA: &str = "muninn.telemetry_surface.v1";
pub const MUNINN_CAPTURE_STREAM_SCHEMA: &str = "muninn.capture_stream.v1";
pub const MUNINN_CAPTURE_STREAM_COMMAND_SCHEMA: &str = "muninn.capture_stream_command.v1";
pub const MUNINN_OBS_STREAM_CATALOG_SCHEMA: &str = "muninn.obs_stream_catalog.v1";
pub const MUNINN_MOVE_MARKER_CANDIDATE_SCHEMA: &str = "muninn.move_marker_candidate.v1";
pub const MUNINN_MOVE_CONTROLLER_STATE_SCHEMA: &str = "muninn.move_controller_state.v1";
pub const MUNINN_HID_CONTROLLER_STATE_SCHEMA: &str = "muninn.hid_controller_state.v1";
pub const MUNINN_MOVE_IDENTITY_SCHEMA: &str = "muninn.move_identity.v1";
pub const MUNINN_MOVE_LIGHT_COMMAND_SCHEMA: &str = "muninn.move_light_command.v1";
pub const MUNINN_MOVE_HUE_PROGRAM_SCHEMA: &str = "muninn.move_hue_program.v1";
pub const MUNINN_MOVE_TRACKER_HEALTH_SCHEMA: &str = "muninn.move_tracker_health.v1";
pub const MUNINN_MOVE_EVIDENCE_TRANSPORT_HEALTH_SCHEMA: &str =
    "muninn.move_evidence_transport_health.v1";
pub const MUNINN_QUEST_ACCESS_SCHEMA: &str = "muninn.quest_access.v1";
pub const MUNINN_COMMAND_BOUNDARY_SCHEMA: &str = "muninn.command_boundary.v1";
pub const MUNINN_TRANSPORT_PROFILE_SCHEMA: &str = "muninn.transport_profile.v1";

#[derive(Clone, Debug, PartialEq, Eq, DatabaseEntry)]
#[cultcache(type = "muninn.capture_stream", schema = "muninn.capture_stream.v1")]
pub struct MuninnCaptureStreamRecord {
    #[cultcache(key = 0)]
    pub stream_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub state: String,
    #[cultcache(key = 3)]
    pub video_source: String,
    #[cultcache(key = 4)]
    pub audio_source: String,
    #[cultcache(key = 5)]
    pub transport: String,
    #[cultcache(key = 6)]
    pub targets: Vec<String>,
    #[cultcache(key = 7)]
    pub command_witness: String,
    #[cultcache(key = 8)]
    pub supervisor_pid: Option<u32>,
    #[cultcache(key = 9)]
    pub mux_pid: Option<u32>,
    #[cultcache(key = 10)]
    pub restart_count: u32,
    #[cultcache(key = 11)]
    pub detail: String,
    #[cultcache(key = 12)]
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, DatabaseEntry)]
#[cultcache(
    type = "muninn.capture_stream_command",
    schema = "muninn.capture_stream_command.v1"
)]
pub struct MuninnCaptureStreamCommandRecord {
    #[cultcache(key = 0)]
    pub command_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub stream_id: String,
    #[cultcache(key = 3)]
    pub state: String,
    #[cultcache(key = 4)]
    pub action: String,
    #[cultcache(key = 5)]
    pub target_host: String,
    #[cultcache(key = 6)]
    pub port: u16,
    #[cultcache(key = 7)]
    pub obs_target_host: Option<String>,
    #[cultcache(key = 8)]
    pub obs_port: u16,
    #[cultcache(key = 9)]
    pub media_transport: String,
    #[cultcache(key = 10)]
    pub media_packet_bytes: u32,
    #[cultcache(key = 11)]
    pub requested_by: String,
    #[cultcache(key = 12)]
    pub detail: String,
    #[cultcache(key = 13)]
    pub updated_at: String,
    #[cultcache(key = 14, default)]
    pub rudp_video_bitrate_kbps: u32,
    #[cultcache(key = 15, default)]
    pub rudp_latency_budget_ms: u32,
    #[cultcache(key = 16, default)]
    pub video_source_id: String,
    #[cultcache(key = 17, default)]
    pub audio_source_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, DatabaseEntry)]
#[cultcache(
    type = "muninn.telemetry_surface",
    schema = "muninn.telemetry_surface.v1"
)]
pub struct MuninnTelemetrySurfaceRecord {
    #[cultcache(key = 0)]
    pub surface_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub state: String,
    #[cultcache(key = 3)]
    pub available_sources: Vec<String>,
    #[cultcache(key = 4)]
    pub stream_affordances: Vec<String>,
    #[cultcache(key = 5)]
    pub active_streams: Vec<String>,
    #[cultcache(key = 6)]
    pub activation_authority: String,
    #[cultcache(key = 7)]
    pub detail: String,
    #[cultcache(key = 8)]
    pub updated_at: String,
    #[cultcache(key = 9, default)]
    pub primary_stream_id: String,
    #[cultcache(key = 10, default)]
    pub primary_stream_label: String,
    #[cultcache(key = 11, default)]
    pub command_rudp_target: String,
    #[cultcache(key = 12, default)]
    pub media_target_host: String,
    #[cultcache(key = 13, default)]
    pub media_port: u16,
    #[cultcache(key = 14, default)]
    pub media_packet_bytes: u32,
    #[cultcache(key = 15, default)]
    pub rudp_video_bitrate_kbps: u32,
    #[cultcache(key = 16, default)]
    pub rudp_latency_budget_ms: u32,
    #[cultcache(key = 17, default)]
    pub video_source_ids: Vec<String>,
    #[cultcache(key = 18, default)]
    pub video_source_labels: Vec<String>,
    #[cultcache(key = 19, default)]
    pub audio_source_ids: Vec<String>,
    #[cultcache(key = 20, default)]
    pub audio_source_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, DatabaseEntry)]
#[cultcache(
    type = "muninn.obs_stream_catalog",
    schema = "muninn.obs_stream_catalog.v1"
)]
pub struct MuninnObsStreamCatalogRecord {
    #[cultcache(key = 0)]
    pub catalog_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub stream_ids: Vec<String>,
    #[cultcache(key = 3)]
    pub labels: Vec<String>,
    #[cultcache(key = 4)]
    pub urls: Vec<String>,
    #[cultcache(key = 5)]
    pub states: Vec<String>,
    #[cultcache(key = 6)]
    pub updated_at: String,
    #[cultcache(key = 7, default)]
    pub command_rudp_target: String,
    #[cultcache(key = 8, default)]
    pub media_target_host: String,
    #[cultcache(key = 9, default)]
    pub media_port: u16,
    #[cultcache(key = 10, default)]
    pub media_packet_bytes: u32,
    #[cultcache(key = 11, default)]
    pub rudp_video_bitrate_kbps: u32,
    #[cultcache(key = 12, default)]
    pub rudp_latency_budget_ms: u32,
    #[cultcache(key = 13, default)]
    pub video_source_ids: Vec<String>,
    #[cultcache(key = 14, default)]
    pub video_source_labels: Vec<String>,
    #[cultcache(key = 15, default)]
    pub audio_source_ids: Vec<String>,
    #[cultcache(key = 16, default)]
    pub audio_source_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "muninn.move_marker_candidate",
    schema = "muninn.move_marker_candidate.v1"
)]
pub struct MuninnMoveMarkerCandidateRecord {
    #[cultcache(key = 0)]
    pub stream_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub camera_id: String,
    #[cultcache(key = 3)]
    pub frame_sequence: u64,
    #[cultcache(key = 4)]
    pub source_id_hash: u64,
    #[cultcache(key = 5)]
    pub tile_x: u32,
    #[cultcache(key = 6)]
    pub tile_y: u32,
    #[cultcache(key = 7)]
    pub center_x_px: f32,
    #[cultcache(key = 8)]
    pub center_y_px: f32,
    #[cultcache(key = 9)]
    pub radius_px: f32,
    #[cultcache(key = 10)]
    pub area_px: u32,
    #[cultcache(key = 11)]
    pub mean_luma: f32,
    #[cultcache(key = 12)]
    pub peak_luma: u32,
    #[cultcache(key = 13)]
    pub score: f32,
    #[cultcache(key = 14)]
    pub observed_at: String,
    #[cultcache(key = 15, default)]
    pub move_id: String,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "muninn.move_controller_state",
    schema = "muninn.move_controller_state.v1"
)]
pub struct MuninnMoveControllerStateRecord {
    #[cultcache(key = 0)]
    pub stream_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub move_id: String,
    #[cultcache(key = 3)]
    pub sequence: u64,
    #[cultcache(key = 4)]
    pub source_timestamp_ns: i64,
    #[cultcache(key = 5)]
    pub accelerometer_xyz: Vec<f32>,
    #[cultcache(key = 6)]
    pub gyroscope_xyz: Vec<f32>,
    #[cultcache(key = 7)]
    pub magnetometer_xyz: Vec<f32>,
    #[cultcache(key = 8)]
    pub trigger_value: f32,
    #[cultcache(key = 9)]
    pub buttons: Vec<String>,
    #[cultcache(key = 10)]
    pub battery01: f32,
    #[cultcache(key = 11)]
    pub observed_at: String,
    #[cultcache(key = 12, default)]
    pub source_path: String,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "muninn.hid_controller_state",
    schema = "muninn.hid_controller_state.v1"
)]
pub struct MuninnHidControllerStateRecord {
    #[cultcache(key = 0)]
    pub stream_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub device_id: String,
    #[cultcache(key = 3)]
    pub device_kind: String,
    #[cultcache(key = 4)]
    pub sequence: u64,
    #[cultcache(key = 5)]
    pub source_timestamp_ns: i64,
    #[cultcache(key = 6)]
    pub axes: Vec<f32>,
    #[cultcache(key = 7)]
    pub buttons: Vec<String>,
    #[cultcache(key = 8)]
    pub battery01: f32,
    #[cultcache(key = 9)]
    pub observed_at: String,
    #[cultcache(key = 10)]
    pub source_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, DatabaseEntry)]
#[cultcache(type = "muninn.move_identity", schema = "muninn.move_identity.v1")]
pub struct MuninnMoveIdentityRecord {
    #[cultcache(key = 0)]
    pub identity_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub move_id: String,
    #[cultcache(key = 3)]
    pub source_path: String,
    #[cultcache(key = 4)]
    pub bluetooth_host_address: String,
    #[cultcache(key = 5)]
    pub state: String,
    #[cultcache(key = 6)]
    pub detail: String,
    #[cultcache(key = 7)]
    pub observed_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, DatabaseEntry)]
#[cultcache(
    type = "muninn.move_light_command",
    schema = "muninn.move_light_command.v1"
)]
pub struct MuninnMoveLightCommandRecord {
    #[cultcache(key = 0)]
    pub command_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub move_id: String,
    #[cultcache(key = 3)]
    pub hidraw_path: String,
    #[cultcache(key = 4)]
    pub colors: Vec<String>,
    #[cultcache(key = 5)]
    pub durations_ms: Vec<u32>,
    #[cultcache(key = 6)]
    pub repeat_count: u32,
    #[cultcache(key = 7)]
    pub authority: String,
    #[cultcache(key = 8)]
    pub state: String,
    #[cultcache(key = 9)]
    pub detail: String,
    #[cultcache(key = 10)]
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, DatabaseEntry)]
#[cultcache(
    type = "muninn.move_hue_program",
    schema = "muninn.move_hue_program.v1"
)]
pub struct MuninnMoveHueProgramRecord {
    #[cultcache(key = 0)]
    pub program_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub mode: String,
    #[cultcache(key = 3)]
    pub cycle_ms: u64,
    #[cultcache(key = 4)]
    pub epoch_ns: i64,
    #[cultcache(key = 5)]
    pub hold_at_ns: i64,
    #[cultcache(key = 6)]
    pub requested_by: String,
    #[cultcache(key = 7)]
    pub updated_at: String,
    #[cultcache(key = 8, default)]
    pub order_mode: String,
    #[cultcache(key = 9, default)]
    pub transition_percent: u8,
    #[cultcache(key = 10, default)]
    pub transition_percent_explicit: bool,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "muninn.move_tracker_health",
    schema = "muninn.move_tracker_health.v1"
)]
pub struct MuninnMoveTrackerHealthRecord {
    #[cultcache(key = 0)]
    pub health_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub camera_id: String,
    #[cultcache(key = 3)]
    pub camera_index: i32,
    #[cultcache(key = 4)]
    pub state: String,
    #[cultcache(key = 5)]
    pub camera_name: String,
    #[cultcache(key = 6)]
    pub camera_api: String,
    #[cultcache(key = 7)]
    pub width: u32,
    #[cultcache(key = 8)]
    pub height: u32,
    #[cultcache(key = 9)]
    pub exposure: f32,
    #[cultcache(key = 10)]
    pub calibrated_controller_count: u32,
    #[cultcache(key = 11)]
    pub update_count: u64,
    #[cultcache(key = 12)]
    pub observation_count: u64,
    #[cultcache(key = 13)]
    pub latest_observation_count: u32,
    #[cultcache(key = 14)]
    pub last_observation_at: String,
    #[cultcache(key = 15)]
    pub detail: String,
    #[cultcache(key = 16)]
    pub updated_at: String,
    #[cultcache(key = 17, default)]
    pub image_mean_rgb: Vec<u32>,
    #[cultcache(key = 18, default)]
    pub image_peak_rgb: Vec<u32>,
    #[cultcache(key = 19, default)]
    pub color_evidence_move_ids: Vec<String>,
    #[cultcache(key = 20, default)]
    pub color_evidence_pixel_counts: Vec<u32>,
    #[cultcache(key = 21, default)]
    pub rejected_stale_count: u64,
    #[cultcache(key = 22, default)]
    pub rejected_radius_count: u64,
    #[cultcache(key = 23, default)]
    pub rejected_bounds_count: u64,
    #[cultcache(key = 24, default)]
    pub rejected_continuity_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, DatabaseEntry)]
#[cultcache(
    type = "muninn.move_evidence_transport_health",
    schema = "muninn.move_evidence_transport_health.v1"
)]
pub struct MuninnMoveEvidenceTransportHealthRecord {
    #[cultcache(key = 0)]
    pub health_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub stream_id: String,
    #[cultcache(key = 3)]
    pub produced_frames: u64,
    #[cultcache(key = 4)]
    pub local_ring_admissions: u64,
    #[cultcache(key = 5)]
    pub remote_handoffs: u64,
    #[cultcache(key = 6)]
    pub remote_sends: u64,
    #[cultcache(key = 7)]
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, DatabaseEntry)]
#[cultcache(type = "muninn.quest_access", schema = "muninn.quest_access.v1")]
pub struct MuninnQuestAccessRecord {
    #[cultcache(key = 0)]
    pub access_id: String,
    #[cultcache(key = 1)]
    pub host_id: String,
    #[cultcache(key = 2)]
    pub serial: String,
    #[cultcache(key = 3)]
    pub connection_state: String,
    #[cultcache(key = 4)]
    pub product: String,
    #[cultcache(key = 5)]
    pub model: String,
    #[cultcache(key = 6)]
    pub device: String,
    #[cultcache(key = 7)]
    pub transport_id: String,
    #[cultcache(key = 8)]
    pub input_stream_id: String,
    #[cultcache(key = 9)]
    pub pose_stream_id: String,
    #[cultcache(key = 10)]
    pub video_input_stream_id: String,
    #[cultcache(key = 11)]
    pub video_input_transport: String,
    #[cultcache(key = 12)]
    pub state: String,
    #[cultcache(key = 13)]
    pub detail: String,
    #[cultcache(key = 14)]
    pub observed_at: String,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "muninn.command_boundary",
    schema = "muninn.command_boundary.v1"
)]
pub struct MuninnCommandBoundaryCompatRecord {
    #[cultcache(key = 0)]
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, DatabaseEntry)]
#[cultcache(
    type = "muninn.transport_profile",
    schema = "muninn.transport_profile.v1"
)]
pub struct MuninnTransportProfileCompatRecord {
    #[cultcache(key = 0)]
    pub value: Value,
}
