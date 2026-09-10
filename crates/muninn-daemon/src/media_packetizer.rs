use anyhow::{Context, Result, anyhow};
use cultnet_rs::{
    CultNetMessage, CultNetWireContract, GAMECULT_MEDIA_AUDIO_PACKET_SCHEMA,
    GameCultMediaAudioPacketRecord,
    GameCultMediaReceiverFeedbackRecord, GameCultMediaVideoAccessUnitRecord,
    GameCultMediaVideoParityShardRecord, decode_cultnet_message_from_slice,
    encode_cultnet_message_to_vec,
};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;

/// Re-exported so this module stays the one place Muninn's media code imports
/// from, while the definitions live in CultLib where the consumer can reach
/// them too.
pub use cultnet_rs::{
    GAMECULT_MEDIA_CHANNEL as MUNINN_MEDIA_RUDP_CHANNEL, GameCultMediaWireRecord,
    MediaWireProvenance, VideoChunkKey, decode_media_wire_record, encode_media_wire_record,
    normalize_video_chunk_feedback_keys, validate_video_record, video_chunk_feedback_key,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoAccessUnit {
    pub bytes: Vec<u8>,
    pub keyframe: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NalUnit<'a> {
    start: usize,
    end: usize,
    payload: &'a [u8],
    nal_type: u8,
}

pub fn video_annex_b_access_units(codec: &str, input: &[u8]) -> Result<Vec<VideoAccessUnit>> {
    match normalized_video_codec(codec).as_deref() {
        Some("h264") => h264_annex_b_access_units(input),
        Some("h265") => h265_annex_b_access_units(input),
        Some("av1") => Err(anyhow!(
            "AV1 access unit splitting is not Annex B; provide an AV1 OBU packetizer"
        )),
        _ => Err(anyhow!("unsupported Annex B video codec {codec}")),
    }
}

pub fn h264_annex_b_access_units(input: &[u8]) -> Result<Vec<VideoAccessUnit>> {
    let nal_units = annex_b_nal_units(input, "H.264", h264_nal_type)?;
    let mut access_units = Vec::new();
    let mut current_start = None;
    let mut current_end = 0_usize;
    let mut current_has_vcl = false;
    let mut current_keyframe = false;

    for nal in nal_units {
        let starts_new = if nal.nal_type == 9 {
            current_start.is_some()
        } else if is_h264_vcl_nal(nal.nal_type) {
            current_has_vcl && h264_first_mb_in_slice(nal.payload).unwrap_or(1) == 0
        } else {
            false
        };

        if starts_new {
            if let Some(start) = current_start {
                access_units.push(VideoAccessUnit {
                    bytes: input[start..current_end].to_vec(),
                    keyframe: current_keyframe,
                });
            }
            current_start = None;
            current_has_vcl = false;
            current_keyframe = false;
        }

        if current_start.is_none() {
            current_start = Some(nal.start);
        }
        current_end = nal.end;
        if is_h264_vcl_nal(nal.nal_type) {
            current_has_vcl = true;
        }
        if nal.nal_type == 5 {
            current_keyframe = true;
        }
    }

    if let Some(start) = current_start {
        access_units.push(VideoAccessUnit {
            bytes: input[start..current_end].to_vec(),
            keyframe: current_keyframe,
        });
    }

    Ok(access_units)
}

pub fn h265_annex_b_access_units(input: &[u8]) -> Result<Vec<VideoAccessUnit>> {
    let nal_units = annex_b_nal_units(input, "H.265", h265_nal_type)?;
    let mut access_units = Vec::new();
    let mut current_start = None;
    let mut current_end = 0_usize;
    let mut current_has_vcl = false;
    let mut current_keyframe = false;

    for nal in nal_units {
        let starts_new = if nal.nal_type == 35 {
            current_start.is_some()
        } else if is_h265_vcl_nal(nal.nal_type) {
            current_has_vcl && h265_first_slice_segment_in_pic(nal.payload).unwrap_or(false)
        } else {
            current_has_vcl && is_h265_pre_vcl_boundary_nal(nal.nal_type)
        };

        if starts_new {
            if let Some(start) = current_start {
                access_units.push(VideoAccessUnit {
                    bytes: input[start..current_end].to_vec(),
                    keyframe: current_keyframe,
                });
            }
            current_start = None;
            current_has_vcl = false;
            current_keyframe = false;
        }

        if current_start.is_none() {
            current_start = Some(nal.start);
        }
        current_end = nal.end;
        if is_h265_vcl_nal(nal.nal_type) {
            current_has_vcl = true;
        }
        if is_h265_irap_nal(nal.nal_type) {
            current_keyframe = true;
        }
    }

    if let Some(start) = current_start {
        access_units.push(VideoAccessUnit {
            bytes: input[start..current_end].to_vec(),
            keyframe: current_keyframe,
        });
    }

    Ok(access_units)
}

pub struct VideoFramePacketizeOptions<'a> {
    pub stream_id: &'a str,
    pub session_id: &'a str,
    pub codec: &'a str,
    pub frame_id: u64,
    pub pts_ticks: i64,
    pub duration_ticks: u32,
    pub timebase_num: u32,
    pub timebase_den: u32,
    pub deadline_ticks: i64,
    pub max_payload_bytes: usize,
}

pub struct VideoAnnexBStreamPacketizeOptions<'a> {
    pub stream_id: &'a str,
    pub session_id: &'a str,
    pub codec: &'a str,
    pub first_frame_id: u64,
    pub first_pts_ticks: i64,
    pub frame_duration_ticks: u32,
    pub timebase_num: u32,
    pub timebase_den: u32,
    pub deadline_delay_ticks: i64,
    pub max_payload_bytes: usize,
}

pub struct VideoAnnexBStreamWireOptions<'a> {
    pub packetize: VideoAnnexBStreamPacketizeOptions<'a>,
    pub stored_at: &'a str,
    pub source_runtime_id: &'a str,
    pub source_role: &'a str,
}

pub struct VideoAnnexBStreamSendConfig {
    pub stream_id: String,
    pub session_id: String,
    pub codec: String,
    pub first_frame_id: u64,
    pub first_pts_ticks: i64,
    pub frame_duration_ticks: u32,
    pub timebase_num: u32,
    pub timebase_den: u32,
    pub deadline_delay_ticks: i64,
    pub max_payload_bytes: usize,
    pub max_pending_bytes: usize,
    pub source_runtime_id: String,
    pub source_role: String,
}

pub struct AudioPacketizeOptions<'a> {
    pub stream_id: &'a str,
    pub session_id: &'a str,
    pub codec: &'a str,
    pub packet_id: u64,
    pub pts_ticks: i64,
    pub duration_ticks: u32,
    pub timebase_num: u32,
    pub timebase_den: u32,
    pub deadline_ticks: i64,
}

pub struct AudioPacketWireOptions<'a> {
    pub packetize: AudioPacketizeOptions<'a>,
    pub stored_at: &'a str,
    pub source_runtime_id: &'a str,
    pub source_role: &'a str,
}


pub struct AudioPcmStreamSendConfig {
    pub stream_id: String,
    pub session_id: String,
    pub codec: String,
    pub first_packet_id: u64,
    pub first_pts_ticks: i64,
    pub packet_duration_ticks: u32,
    pub timebase_num: u32,
    pub timebase_den: u32,
    pub deadline_delay_ticks: i64,
    pub channels: u32,
    pub bytes_per_sample: u32,
    pub max_pending_bytes: usize,
    pub source_runtime_id: String,
    pub source_role: String,
}

pub struct ReceiverFeedbackOptions<'a> {
    pub stream_id: &'a str,
    pub session_id: &'a str,
    pub receiver_id: &'a str,
    pub highest_decodable_frame_id: Option<u64>,
    pub missing_frame_ids: Vec<u64>,
    pub missing_video_chunk_keys: Vec<String>,
    pub late_frame_ids: Vec<u64>,
    pub requested_keyframe: bool,
    pub jitter_us: i64,
    pub decode_queue_us: i64,
    pub observed_at: &'a str,
}






#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MuninnMediaSendPayload {
    pub channel_id: &'static str,
    pub payload: Vec<u8>,
}

pub struct VideoAnnexBStreamSendState {
    config: VideoAnnexBStreamSendConfig,
    pending: Vec<u8>,
    next_frame_id: u64,
    next_pts_ticks: i64,
}

impl VideoAnnexBStreamSendState {
    pub fn new(config: VideoAnnexBStreamSendConfig) -> Result<Self> {
        if matches!(normalized_video_codec(&config.codec), None | Some("av1")) {
            return Err(anyhow!(
                "Annex B stream sender requires H.264/AVC or H.265/HEVC codec"
            ));
        }
        if config.stream_id.is_empty() {
            return Err(anyhow!("stream_id must be non-empty"));
        }
        if config.session_id.is_empty() {
            return Err(anyhow!("session_id must be non-empty"));
        }
        if config.frame_duration_ticks == 0 {
            return Err(anyhow!("frame_duration_ticks must be greater than zero"));
        }
        if config.timebase_num == 0 || config.timebase_den == 0 {
            return Err(anyhow!("video timebase must be non-zero"));
        }
        if config.deadline_delay_ticks < 0 {
            return Err(anyhow!("deadline_delay_ticks must be non-negative"));
        }
        if config.max_payload_bytes == 0 {
            return Err(anyhow!("max_payload_bytes must be greater than zero"));
        }
        if config.max_pending_bytes == 0 {
            return Err(anyhow!("max_pending_bytes must be greater than zero"));
        }
        if config.source_runtime_id.is_empty() {
            return Err(anyhow!("source_runtime_id must be non-empty"));
        }
        if config.source_role.is_empty() {
            return Err(anyhow!("source_role must be non-empty"));
        }

        Ok(Self {
            next_frame_id: config.first_frame_id,
            next_pts_ticks: config.first_pts_ticks,
            config,
            pending: Vec::new(),
        })
    }

    pub fn push(&mut self, stored_at: &str, bytes: &[u8]) -> Result<Vec<MuninnMediaSendPayload>> {
        if !bytes.is_empty() {
            self.pending.extend_from_slice(bytes);
        }
        if self.pending.len() > self.config.max_pending_bytes {
            return Err(anyhow!(
                "Annex B stream sender pending buffer exceeded {} bytes without a complete frame",
                self.config.max_pending_bytes
            ));
        }
        self.emit_available(stored_at, false)
    }

    pub fn finish(&mut self, stored_at: &str) -> Result<Vec<MuninnMediaSendPayload>> {
        self.emit_available(stored_at, true)
    }

    pub fn pending_bytes(&self) -> usize {
        self.pending.len()
    }

    pub fn next_frame_id(&self) -> u64 {
        self.next_frame_id
    }

    fn emit_available(
        &mut self,
        stored_at: &str,
        flush_last: bool,
    ) -> Result<Vec<MuninnMediaSendPayload>> {
        if stored_at.is_empty() {
            return Err(anyhow!("stored_at must be non-empty"));
        }
        if self.pending.is_empty() {
            return Ok(Vec::new());
        }

        let access_units = match video_annex_b_access_units(&self.config.codec, &self.pending) {
            Ok(access_units) => access_units,
            Err(error)
                if error.to_string().contains("has no start codes")
                    || error.to_string().contains("has no NAL payloads") =>
            {
                return Ok(Vec::new());
            }
            Err(error) => return Err(error),
        };
        let emit_count = if flush_last {
            access_units.len()
        } else {
            access_units.len().saturating_sub(1)
        };
        if emit_count == 0 {
            return Ok(Vec::new());
        }

        let mut send_payloads = Vec::new();
        let mut emitted_bytes = 0_usize;
        for access_unit in access_units.iter().take(emit_count) {
            emitted_bytes = emitted_bytes
                .checked_add(access_unit.bytes.len())
                .ok_or_else(|| anyhow!("emitted Annex B byte count overflow"))?;
            let deadline_ticks = self
                .next_pts_ticks
                .checked_add(self.config.deadline_delay_ticks)
                .ok_or_else(|| anyhow!("video deadline_ticks overflow"))?;
            let records = packetize_video_access_unit(
                VideoFramePacketizeOptions {
                    stream_id: &self.config.stream_id,
                    session_id: &self.config.session_id,
                    codec: &self.config.codec,
                    frame_id: self.next_frame_id,
                    pts_ticks: self.next_pts_ticks,
                    duration_ticks: self.config.frame_duration_ticks,
                    timebase_num: self.config.timebase_num,
                    timebase_den: self.config.timebase_den,
                    deadline_ticks,
                    max_payload_bytes: self.config.max_payload_bytes,
                },
                access_unit,
            )?;
            let wire_records = video_wire_records_with_parity(&records)?;
            send_payloads.extend(wire_payloads_to_media_send(encode_media_wire_records(
                &wire_records,
                stored_at,
                &self.config.source_runtime_id,
                &self.config.source_role,
            )?));
            self.advance_frame_clock()?;
        }

        self.pending.drain(..emitted_bytes);
        Ok(send_payloads)
    }

    fn advance_frame_clock(&mut self) -> Result<()> {
        self.next_frame_id = self
            .next_frame_id
            .checked_add(1)
            .ok_or_else(|| anyhow!("video frame_id overflow"))?;
        self.next_pts_ticks = self
            .next_pts_ticks
            .checked_add(i64::from(self.config.frame_duration_ticks))
            .ok_or_else(|| anyhow!("video pts_ticks overflow"))?;
        Ok(())
    }
}



pub struct AudioPcmStreamSendState {
    config: AudioPcmStreamSendConfig,
    pending: Vec<u8>,
    next_packet_id: u64,
    next_pts_ticks: i64,
}

impl AudioPcmStreamSendState {
    pub fn new(config: AudioPcmStreamSendConfig) -> Result<Self> {
        if config.stream_id.is_empty() {
            return Err(anyhow!("stream_id must be non-empty"));
        }
        if config.session_id.is_empty() {
            return Err(anyhow!("session_id must be non-empty"));
        }
        if config.codec.is_empty() {
            return Err(anyhow!("codec must be non-empty"));
        }
        if config.packet_duration_ticks == 0 {
            return Err(anyhow!("packet_duration_ticks must be greater than zero"));
        }
        if config.timebase_num == 0 || config.timebase_den == 0 {
            return Err(anyhow!("audio timebase must be non-zero"));
        }
        if config.deadline_delay_ticks < 0 {
            return Err(anyhow!("deadline_delay_ticks must be non-negative"));
        }
        if config.channels == 0 {
            return Err(anyhow!("audio channels must be non-zero"));
        }
        if config.bytes_per_sample == 0 {
            return Err(anyhow!("audio bytes_per_sample must be non-zero"));
        }
        if config.max_pending_bytes == 0 {
            return Err(anyhow!("max_pending_bytes must be greater than zero"));
        }
        if config.source_runtime_id.is_empty() {
            return Err(anyhow!("source_runtime_id must be non-empty"));
        }
        if config.source_role.is_empty() {
            return Err(anyhow!("source_role must be non-empty"));
        }

        Ok(Self {
            next_packet_id: config.first_packet_id,
            next_pts_ticks: config.first_pts_ticks,
            config,
            pending: Vec::new(),
        })
    }

    pub fn push(&mut self, stored_at: &str, bytes: &[u8]) -> Result<Vec<MuninnMediaSendPayload>> {
        if !bytes.is_empty() {
            self.pending.extend_from_slice(bytes);
        }
        if self.pending.len() > self.config.max_pending_bytes {
            return Err(anyhow!(
                "PCM stream sender pending buffer exceeded {} bytes without a complete packet",
                self.config.max_pending_bytes
            ));
        }
        self.emit_available(stored_at)
    }

    pub fn finish(&mut self, stored_at: &str) -> Result<Vec<MuninnMediaSendPayload>> {
        let payloads = self.emit_available(stored_at)?;
        if !self.pending.is_empty() {
            return Err(anyhow!(
                "PCM stream sender finished with {} trailing bytes",
                self.pending.len()
            ));
        }
        Ok(payloads)
    }

    fn emit_available(&mut self, stored_at: &str) -> Result<Vec<MuninnMediaSendPayload>> {
        if stored_at.is_empty() {
            return Err(anyhow!("stored_at must be non-empty"));
        }

        let bytes_per_frame = usize::try_from(self.config.channels)
            .ok()
            .and_then(|channels| {
                usize::try_from(self.config.bytes_per_sample)
                    .ok()
                    .map(|bytes_per_sample| channels.saturating_mul(bytes_per_sample))
            })
            .ok_or_else(|| anyhow!("PCM stream sender bytes_per_frame overflow"))?;
        let bytes_per_packet = bytes_per_frame
            .checked_mul(self.config.packet_duration_ticks as usize)
            .ok_or_else(|| anyhow!("PCM stream sender bytes_per_packet overflow"))?;
        if bytes_per_packet == 0 {
            return Err(anyhow!(
                "PCM stream sender bytes_per_packet must be non-zero"
            ));
        }
        let complete_packets = self.pending.len() / bytes_per_packet;
        if complete_packets == 0 {
            return Ok(Vec::new());
        }

        let mut payloads = Vec::with_capacity(complete_packets);
        for packet_index in 0..complete_packets {
            let start = packet_index * bytes_per_packet;
            let end = start + bytes_per_packet;
            let deadline_ticks = self
                .next_pts_ticks
                .checked_add(self.config.deadline_delay_ticks)
                .ok_or_else(|| anyhow!("audio deadline_ticks overflow"))?;
            payloads.push(audio_packet_send_payload(
                AudioPacketWireOptions {
                    packetize: AudioPacketizeOptions {
                        stream_id: &self.config.stream_id,
                        session_id: &self.config.session_id,
                        codec: &self.config.codec,
                        packet_id: self.next_packet_id,
                        pts_ticks: self.next_pts_ticks,
                        duration_ticks: self.config.packet_duration_ticks,
                        timebase_num: self.config.timebase_num,
                        timebase_den: self.config.timebase_den,
                        deadline_ticks,
                    },
                    stored_at,
                    source_runtime_id: &self.config.source_runtime_id,
                    source_role: &self.config.source_role,
                },
                &self.pending[start..end],
            )?);
            self.advance_packet_clock()?;
        }

        self.pending.drain(..complete_packets * bytes_per_packet);
        Ok(payloads)
    }

    fn advance_packet_clock(&mut self) -> Result<()> {
        self.next_packet_id = self
            .next_packet_id
            .checked_add(1)
            .ok_or_else(|| anyhow!("audio packet_id overflow"))?;
        self.next_pts_ticks = self
            .next_pts_ticks
            .checked_add(i64::from(self.config.packet_duration_ticks))
            .ok_or_else(|| anyhow!("audio pts_ticks overflow"))?;
        Ok(())
    }
}




pub fn packetize_video_access_unit(
    options: VideoFramePacketizeOptions<'_>,
    access_unit: &VideoAccessUnit,
) -> Result<Vec<GameCultMediaVideoAccessUnitRecord>> {
    if options.stream_id.is_empty() {
        return Err(anyhow!("stream_id must be non-empty"));
    }
    if options.session_id.is_empty() {
        return Err(anyhow!("session_id must be non-empty"));
    }
    if options.codec.is_empty() {
        return Err(anyhow!("codec must be non-empty"));
    }
    if options.timebase_num == 0 || options.timebase_den == 0 {
        return Err(anyhow!("video timebase must be non-zero"));
    }
    if options.duration_ticks == 0 {
        return Err(anyhow!("video duration_ticks must be greater than zero"));
    }
    if options.deadline_ticks < options.pts_ticks {
        return Err(anyhow!("video deadline_ticks must not precede pts_ticks"));
    }
    if options.max_payload_bytes == 0 {
        return Err(anyhow!("max_payload_bytes must be greater than zero"));
    }
    if access_unit.bytes.is_empty() {
        return Err(anyhow!("access unit payload must be non-empty"));
    }

    let chunk_count = access_unit.bytes.len().div_ceil(options.max_payload_bytes);
    if chunk_count > u16::MAX as usize {
        return Err(anyhow!("video access unit requires more than 65535 chunks"));
    }

    let mut records = Vec::with_capacity(chunk_count);
    for (chunk_index, payload) in access_unit
        .bytes
        .chunks(options.max_payload_bytes)
        .enumerate()
    {
        records.push(GameCultMediaVideoAccessUnitRecord {
            stream_id: options.stream_id.to_string(),
            session_id: options.session_id.to_string(),
            frame_id: options.frame_id,
            codec: options.codec.to_string(),
            pts_ticks: options.pts_ticks,
            duration_ticks: options.duration_ticks,
            timebase_num: options.timebase_num,
            timebase_den: options.timebase_den,
            keyframe: access_unit.keyframe,
            dependency_frame_id: if access_unit.keyframe
                || !access_unit_references_previous_frame(options.codec, access_unit)
            {
                None
            } else {
                options.frame_id.checked_sub(1)
            },
            deadline_ticks: options.deadline_ticks,
            chunk_index: chunk_index as u16,
            chunk_count: chunk_count as u16,
            payload: payload.to_vec(),
        });
    }

    Ok(records)
}

const MUNINN_VIDEO_PARITY_STRIPES: u16 = 16;

pub fn build_video_parity_shards(
    chunks: &[GameCultMediaVideoAccessUnitRecord],
) -> Result<Vec<GameCultMediaVideoParityShardRecord>> {
    if chunks.len() <= 1 {
        return Ok(Vec::new());
    }
    let first = chunks
        .first()
        .ok_or_else(|| anyhow!("video parity requires at least one chunk"))?;
    validate_video_record(first)?;
    if chunks.len() != first.chunk_count as usize {
        return Err(anyhow!(
            "video parity requires all chunks: expected {}, received {}",
            first.chunk_count,
            chunks.len()
        ));
    }

    let mut by_index = BTreeMap::new();
    for chunk in chunks {
        validate_video_record(chunk)?;
        if chunk.stream_id != first.stream_id
            || chunk.session_id != first.session_id
            || chunk.frame_id != first.frame_id
            || chunk.codec != first.codec
            || chunk.pts_ticks != first.pts_ticks
            || chunk.duration_ticks != first.duration_ticks
            || chunk.timebase_num != first.timebase_num
            || chunk.timebase_den != first.timebase_den
            || chunk.keyframe != first.keyframe
            || chunk.dependency_frame_id != first.dependency_frame_id
            || chunk.deadline_ticks != first.deadline_ticks
            || chunk.chunk_count != first.chunk_count
        {
            return Err(anyhow!("video parity chunks have mixed metadata"));
        }
        if by_index.insert(chunk.chunk_index, chunk).is_some() {
            return Err(anyhow!(
                "video parity received duplicate chunk_index {}",
                chunk.chunk_index
            ));
        }
    }

    let mut chunk_payload_len = 0_usize;
    let mut last_chunk_payload_bytes = 0_u32;
    for index in 0..first.chunk_count {
        let chunk = by_index
            .get(&index)
            .ok_or_else(|| anyhow!("video parity missing chunk_index {index}"))?;
        chunk_payload_len = chunk_payload_len.max(chunk.payload.len());
        if index == first.chunk_count - 1 {
            last_chunk_payload_bytes = u32::try_from(chunk.payload.len())
                .context("video parity last chunk payload length exceeds u32")?;
        }
    }
    let chunk_payload_bytes = u32::try_from(chunk_payload_len)
        .context("video parity chunk payload length exceeds u32")?;
    let parity_count = first.chunk_count.min(MUNINN_VIDEO_PARITY_STRIPES);
    let mut records = Vec::with_capacity(parity_count as usize);
    for parity_index in 0..parity_count {
        let mut parity_len = 0_usize;
        for index in (parity_index..first.chunk_count).step_by(parity_count as usize) {
            let chunk = by_index.get(&index).expect("chunk index was checked above");
            parity_len = parity_len.max(chunk.payload.len());
        }
        if parity_len == 0 {
            continue;
        }
        let mut parity = vec![0_u8; parity_len];
        for index in (parity_index..first.chunk_count).step_by(parity_count as usize) {
            let chunk = by_index.get(&index).expect("chunk index was checked above");
            for (offset, byte) in chunk.payload.iter().enumerate() {
                parity[offset] ^= *byte;
            }
        }
        records.push(GameCultMediaVideoParityShardRecord {
            stream_id: first.stream_id.clone(),
            session_id: first.session_id.clone(),
            frame_id: first.frame_id,
            codec: first.codec.clone(),
            pts_ticks: first.pts_ticks,
            duration_ticks: first.duration_ticks,
            timebase_num: first.timebase_num,
            timebase_den: first.timebase_den,
            keyframe: first.keyframe,
            dependency_frame_id: first.dependency_frame_id,
            deadline_ticks: first.deadline_ticks,
            chunk_count: first.chunk_count,
            parity_index,
            parity_count,
            chunk_payload_bytes,
            last_chunk_payload_bytes,
            payload: parity,
        });
    }
    Ok(records)
}

fn video_wire_records_with_parity(
    records: &[GameCultMediaVideoAccessUnitRecord],
) -> Result<Vec<GameCultMediaWireRecord>> {
    let mut wire_records = Vec::new();
    let mut offset = 0_usize;
    while offset < records.len() {
        let first = &records[offset];
        let mut end = offset + 1;
        while end < records.len()
            && records[end].stream_id == first.stream_id
            && records[end].session_id == first.session_id
            && records[end].frame_id == first.frame_id
        {
            end += 1;
        }
        let frame_records = &records[offset..end];
        wire_records.extend(
            frame_records
                .iter()
                .cloned()
                .map(GameCultMediaWireRecord::Video),
        );
        for parity in build_video_parity_shards(frame_records)? {
            wire_records.push(GameCultMediaWireRecord::VideoParity(parity));
        }
        offset = end;
    }
    Ok(wire_records)
}

pub fn packetize_video_annex_b_stream(
    options: VideoAnnexBStreamPacketizeOptions<'_>,
    input: &[u8],
) -> Result<Vec<GameCultMediaVideoAccessUnitRecord>> {
    if options.frame_duration_ticks == 0 {
        return Err(anyhow!("frame_duration_ticks must be greater than zero"));
    }
    if options.deadline_delay_ticks < 0 {
        return Err(anyhow!("deadline_delay_ticks must be non-negative"));
    }

    let access_units = video_annex_b_access_units(options.codec, input)?;
    let mut records = Vec::new();
    for (index, access_unit) in access_units.iter().enumerate() {
        let frame_offset = u64::try_from(index).context("video frame index overflow")?;
        let frame_id = options
            .first_frame_id
            .checked_add(frame_offset)
            .ok_or_else(|| anyhow!("video frame_id overflow"))?;
        let pts_offset = i64::try_from(index)
            .ok()
            .and_then(|index| index.checked_mul(i64::from(options.frame_duration_ticks)))
            .ok_or_else(|| anyhow!("video pts_ticks overflow"))?;
        let pts_ticks = options
            .first_pts_ticks
            .checked_add(pts_offset)
            .ok_or_else(|| anyhow!("video pts_ticks overflow"))?;
        let deadline_ticks = pts_ticks
            .checked_add(options.deadline_delay_ticks)
            .ok_or_else(|| anyhow!("video deadline_ticks overflow"))?;

        records.extend(packetize_video_access_unit(
            VideoFramePacketizeOptions {
                stream_id: options.stream_id,
                session_id: options.session_id,
                codec: options.codec,
                frame_id,
                pts_ticks,
                duration_ticks: options.frame_duration_ticks,
                timebase_num: options.timebase_num,
                timebase_den: options.timebase_den,
                deadline_ticks,
                max_payload_bytes: options.max_payload_bytes,
            },
            access_unit,
        )?);
    }

    Ok(records)
}

pub fn packetize_audio_packet(
    options: AudioPacketizeOptions<'_>,
    payload: &[u8],
) -> Result<GameCultMediaAudioPacketRecord> {
    if options.stream_id.is_empty() {
        return Err(anyhow!("stream_id must be non-empty"));
    }
    if options.session_id.is_empty() {
        return Err(anyhow!("session_id must be non-empty"));
    }
    if options.codec.is_empty() {
        return Err(anyhow!("codec must be non-empty"));
    }
    if options.timebase_num == 0 || options.timebase_den == 0 {
        return Err(anyhow!("audio timebase must be non-zero"));
    }
    if options.duration_ticks == 0 {
        return Err(anyhow!("audio duration_ticks must be greater than zero"));
    }
    if options.deadline_ticks < options.pts_ticks {
        return Err(anyhow!("audio deadline_ticks must not precede pts_ticks"));
    }
    if payload.is_empty() {
        return Err(anyhow!("audio packet payload must be non-empty"));
    }

    Ok(GameCultMediaAudioPacketRecord {
        stream_id: options.stream_id.to_string(),
        session_id: options.session_id.to_string(),
        packet_id: options.packet_id,
        codec: options.codec.to_string(),
        pts_ticks: options.pts_ticks,
        duration_ticks: options.duration_ticks,
        timebase_num: options.timebase_num,
        timebase_den: options.timebase_den,
        deadline_ticks: options.deadline_ticks,
        payload: payload.to_vec(),
    })
}

#[derive(Default)]
pub struct AudioPacketBuffer {
    stream_id: Option<String>,
    session_id: Option<String>,
    codec: Option<String>,
    timebase_num: Option<u32>,
    timebase_den: Option<u32>,
    next_packet_id: Option<u64>,
    emitted_any: bool,
    packets: BTreeMap<u64, GameCultMediaAudioPacketRecord>,
}

impl AudioPacketBuffer {
    pub fn insert(&mut self, packet: GameCultMediaAudioPacketRecord) -> Result<()> {
        self.require_matching_audio_stream(&packet)?;
        if packet.payload.is_empty() {
            return Err(anyhow!("audio packet buffer payload must be non-empty"));
        }
        if self.packets.contains_key(&packet.packet_id) {
            return Err(anyhow!(
                "audio packet buffer has duplicate packet_id {}",
                packet.packet_id
            ));
        }
        match self.next_packet_id {
            None => self.next_packet_id = Some(packet.packet_id),
            Some(next_packet_id) if packet.packet_id < next_packet_id && !self.emitted_any => {
                self.next_packet_id = Some(packet.packet_id);
            }
            Some(next_packet_id) if packet.packet_id < next_packet_id => {
                return Err(anyhow!(
                    "audio packet buffer received stale packet_id {} before next expected {}",
                    packet.packet_id,
                    next_packet_id
                ));
            }
            _ => {}
        }
        self.packets.insert(packet.packet_id, packet);
        Ok(())
    }

    pub fn pop_ready_packets(&mut self) -> Vec<GameCultMediaAudioPacketRecord> {
        let Some(mut next_packet_id) = self.next_packet_id else {
            return Vec::new();
        };
        let mut ready = Vec::new();
        while let Some(packet) = self.packets.remove(&next_packet_id) {
            ready.push(packet);
            next_packet_id = next_packet_id.saturating_add(1);
        }
        if !ready.is_empty() {
            self.emitted_any = true;
        }
        self.next_packet_id = Some(next_packet_id);
        ready
    }

    pub fn expire_late_packets(&mut self, now_ticks: i64) -> Vec<u64> {
        let expired_ids = self
            .packets
            .iter()
            .filter_map(|(packet_id, packet)| {
                if packet.deadline_ticks <= now_ticks {
                    Some(*packet_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for packet_id in &expired_ids {
            self.packets.remove(packet_id);
        }
        if let Some(next_packet_id) = self.next_packet_id {
            if expired_ids.contains(&next_packet_id) {
                self.next_packet_id = self.packets.keys().next().copied();
            }
        }
        expired_ids
    }

    pub fn pending_packet_count(&self) -> usize {
        self.packets.len()
    }

    fn require_matching_audio_stream(
        &mut self,
        packet: &GameCultMediaAudioPacketRecord,
    ) -> Result<()> {
        if packet.stream_id.is_empty() {
            return Err(anyhow!("audio packet buffer stream_id must be non-empty"));
        }
        if packet.session_id.is_empty() {
            return Err(anyhow!("audio packet buffer session_id must be non-empty"));
        }
        if packet.codec.is_empty() {
            return Err(anyhow!("audio packet buffer codec must be non-empty"));
        }
        if packet.timebase_num == 0 || packet.timebase_den == 0 {
            return Err(anyhow!("audio packet buffer timebase must be non-zero"));
        }

        match (
            self.stream_id.as_deref(),
            self.session_id.as_deref(),
            self.codec.as_deref(),
            self.timebase_num,
            self.timebase_den,
        ) {
            (None, None, None, None, None) => {
                self.stream_id = Some(packet.stream_id.clone());
                self.session_id = Some(packet.session_id.clone());
                self.codec = Some(packet.codec.clone());
                self.timebase_num = Some(packet.timebase_num);
                self.timebase_den = Some(packet.timebase_den);
                Ok(())
            }
            (
                Some(stream_id),
                Some(session_id),
                Some(codec),
                Some(timebase_num),
                Some(timebase_den),
            ) if stream_id == packet.stream_id
                && session_id == packet.session_id
                && codec == packet.codec
                && timebase_num == packet.timebase_num
                && timebase_den == packet.timebase_den =>
            {
                Ok(())
            }
            _ => Err(anyhow!(
                "audio packet buffer received mixed stream metadata"
            )),
        }
    }
}

pub fn build_receiver_feedback(
    options: ReceiverFeedbackOptions<'_>,
) -> Result<GameCultMediaReceiverFeedbackRecord> {
    if options.stream_id.is_empty() {
        return Err(anyhow!("stream_id must be non-empty"));
    }
    if options.session_id.is_empty() {
        return Err(anyhow!("session_id must be non-empty"));
    }
    if options.receiver_id.is_empty() {
        return Err(anyhow!("receiver_id must be non-empty"));
    }
    if options.observed_at.is_empty() {
        return Err(anyhow!("observed_at must be non-empty"));
    }
    if options.jitter_us < 0 {
        return Err(anyhow!("jitter_us must be non-negative"));
    }
    if options.decode_queue_us < 0 {
        return Err(anyhow!("decode_queue_us must be non-negative"));
    }

    let mut missing_frame_ids = options.missing_frame_ids;
    missing_frame_ids.sort_unstable();
    missing_frame_ids.dedup();

    let missing_video_chunk_keys =
        normalize_video_chunk_feedback_keys(options.missing_video_chunk_keys)?;

    let mut late_frame_ids = options.late_frame_ids;
    late_frame_ids.sort_unstable();
    late_frame_ids.dedup();

    let requested_keyframe = options.requested_keyframe
        || !missing_frame_ids.is_empty()
        || !missing_video_chunk_keys.is_empty();

    Ok(GameCultMediaReceiverFeedbackRecord {
        stream_id: options.stream_id.to_string(),
        session_id: options.session_id.to_string(),
        receiver_id: options.receiver_id.to_string(),
        highest_decodable_frame_id: options.highest_decodable_frame_id,
        missing_frame_ids,
        late_frame_ids,
        requested_keyframe,
        jitter_us: options.jitter_us,
        decode_queue_us: options.decode_queue_us,
        observed_at: options.observed_at.to_string(),
        missing_video_chunk_keys,
    })
}

/// Muninn's producer name on the wire. The envelope in CultLib takes this as a
/// parameter so it never has to know any producer; this is where Muninn says
/// who it is, once.
pub const MUNINN_MEDIA_PRODUCER: &str = "muninn";

fn muninn_provenance<'a>(
    stored_at: &'a str,
    source_runtime_id: &'a str,
    source_role: &'a str,
) -> MediaWireProvenance<'a> {
    MediaWireProvenance {
        stored_at,
        runtime_id: source_runtime_id,
        role: source_role,
        producer: MUNINN_MEDIA_PRODUCER,
    }
}

pub fn encode_media_wire_records(
    records: &[GameCultMediaWireRecord],
    stored_at: &str,
    source_runtime_id: &str,
    source_role: &str,
) -> Result<Vec<Vec<u8>>> {
    let provenance = muninn_provenance(stored_at, source_runtime_id, source_role);
    records
        .iter()
        .map(|record| encode_media_wire_record(record, provenance))
        .collect()
}

pub fn encode_video_annex_b_stream_wire_records(
    options: VideoAnnexBStreamWireOptions<'_>,
    input: &[u8],
) -> Result<Vec<Vec<u8>>> {
    let records = packetize_video_annex_b_stream(options.packetize, input)?;
    let wire_records = video_wire_records_with_parity(&records)?;
    encode_media_wire_records(
        &wire_records,
        options.stored_at,
        options.source_runtime_id,
        options.source_role,
    )
}

pub fn video_annex_b_stream_send_payloads(
    options: VideoAnnexBStreamWireOptions<'_>,
    input: &[u8],
) -> Result<Vec<MuninnMediaSendPayload>> {
    encode_video_annex_b_stream_wire_records(options, input).map(wire_payloads_to_media_send)
}

pub fn encode_audio_packet_wire_record(
    options: AudioPacketWireOptions<'_>,
    payload: &[u8],
) -> Result<Vec<u8>> {
    let record = packetize_audio_packet(options.packetize, payload)?;
    encode_media_wire_record(
        &GameCultMediaWireRecord::Audio(record),
        muninn_provenance(
            options.stored_at,
            options.source_runtime_id,
            options.source_role,
        ),
    )
}

pub fn audio_packet_send_payload(
    options: AudioPacketWireOptions<'_>,
    payload: &[u8],
) -> Result<MuninnMediaSendPayload> {
    encode_audio_packet_wire_record(options, payload).map(wire_payload_to_media_send)
}

fn wire_payloads_to_media_send(payloads: Vec<Vec<u8>>) -> Vec<MuninnMediaSendPayload> {
    payloads
        .into_iter()
        .map(wire_payload_to_media_send)
        .collect()
}

fn wire_payload_to_media_send(payload: Vec<u8>) -> MuninnMediaSendPayload {
    MuninnMediaSendPayload {
        channel_id: MUNINN_MEDIA_RUDP_CHANNEL,
        payload,
    }
}


fn encode_record_payload<T: Serialize>(record: &T) -> Result<Vec<u8>> {
    rmp_serde::to_vec(record).map_err(Into::into)
}




fn decode_record_payload<T: DeserializeOwned>(payload: &[u8]) -> Result<T> {
    rmp_serde::from_slice(payload).map_err(Into::into)
}










fn normalized_video_codec(codec: &str) -> Option<&'static str> {
    match codec.trim().to_ascii_lowercase().as_str() {
        "h264" | "h.264" | "avc" | "avc1" | "video/avc" => Some("h264"),
        "h265" | "h.265" | "hevc" | "hev1" | "hvc1" | "video/hevc" => Some("h265"),
        "av1" | "av01" | "video/av1" => Some("av1"),
        _ => None,
    }
}



fn annex_b_nal_units<'a>(
    input: &'a [u8],
    codec_name: &str,
    nal_type: fn(&[u8]) -> Option<u8>,
) -> Result<Vec<NalUnit<'a>>> {
    let mut starts = Vec::new();
    let mut offset = 0_usize;
    while let Some((start, prefix_len)) = find_annex_b_start_code(input, offset) {
        starts.push((start, prefix_len));
        offset = start + prefix_len;
    }

    if starts.is_empty() {
        return Err(anyhow!("{codec_name} Annex B stream has no start codes"));
    }

    let mut nal_units = Vec::new();
    for index in 0..starts.len() {
        let (start, prefix_len) = starts[index];
        let payload_start = start + prefix_len;
        let end = starts
            .get(index + 1)
            .map(|(next_start, _)| *next_start)
            .unwrap_or(input.len());
        if payload_start >= end {
            continue;
        }
        let payload = &input[payload_start..end];
        if let Some(nal_type) = nal_type(payload) {
            nal_units.push(NalUnit {
                start,
                end,
                payload,
                nal_type,
            });
        }
    }

    if nal_units.is_empty() {
        return Err(anyhow!("{codec_name} Annex B stream has no NAL payloads"));
    }

    Ok(nal_units)
}

fn find_annex_b_start_code(input: &[u8], from: usize) -> Option<(usize, usize)> {
    let mut index = from;
    while index + 3 <= input.len() {
        if input[index..].starts_with(&[0, 0, 1]) {
            return Some((index, 3));
        }
        if index + 4 <= input.len() && input[index..].starts_with(&[0, 0, 0, 1]) {
            return Some((index, 4));
        }
        index += 1;
    }
    None
}

fn is_h264_vcl_nal(nal_type: u8) -> bool {
    matches!(nal_type, 1..=5)
}

fn access_unit_references_previous_frame(codec: &str, access_unit: &VideoAccessUnit) -> bool {
    if access_unit.keyframe {
        return false;
    }
    match normalized_video_codec(codec).as_deref() {
        Some("h264") => h264_access_unit_has_reference_vcl(&access_unit.bytes).unwrap_or(true),
        _ => true,
    }
}

fn h264_access_unit_has_reference_vcl(input: &[u8]) -> Result<bool> {
    let nal_units = annex_b_nal_units(input, "H.264", h264_nal_type)?;
    Ok(nal_units
        .iter()
        .any(|nal| is_h264_vcl_nal(nal.nal_type) && h264_nal_ref_idc(nal.payload) > 0))
}

fn h264_nal_type(payload: &[u8]) -> Option<u8> {
    payload.first().map(|byte| byte & 0x1f)
}

fn h264_nal_ref_idc(payload: &[u8]) -> u8 {
    payload
        .first()
        .map(|byte| (byte >> 5) & 0x03)
        .unwrap_or_default()
}

fn h264_first_mb_in_slice(nal_payload: &[u8]) -> Option<u64> {
    if nal_payload.len() < 2 {
        return None;
    }
    let rbsp = h264_ebsp_to_rbsp(&nal_payload[1..]);
    read_unsigned_exp_golomb(&rbsp)
}

fn is_h265_vcl_nal(nal_type: u8) -> bool {
    nal_type <= 31
}

fn is_h265_irap_nal(nal_type: u8) -> bool {
    matches!(nal_type, 16..=21)
}

fn is_h265_pre_vcl_boundary_nal(nal_type: u8) -> bool {
    matches!(nal_type, 32..=34 | 39 | 40)
}

fn h265_nal_type(payload: &[u8]) -> Option<u8> {
    if payload.len() < 2 {
        return None;
    }
    Some((payload[0] >> 1) & 0x3f)
}

fn h265_first_slice_segment_in_pic(nal_payload: &[u8]) -> Option<bool> {
    if nal_payload.len() < 3 {
        return None;
    }
    let rbsp = h264_ebsp_to_rbsp(&nal_payload[2..]);
    rbsp.first().map(|byte| (byte & 0x80) != 0)
}

fn h264_ebsp_to_rbsp(payload: &[u8]) -> Vec<u8> {
    let mut rbsp = Vec::with_capacity(payload.len());
    let mut zero_count = 0_u8;
    for &byte in payload {
        if zero_count >= 2 && byte == 0x03 {
            zero_count = 0;
            continue;
        }
        rbsp.push(byte);
        zero_count = if byte == 0 { zero_count + 1 } else { 0 };
    }
    rbsp
}

fn read_unsigned_exp_golomb(payload: &[u8]) -> Option<u64> {
    let mut reader = BitReader::new(payload);
    let mut leading_zero_bits = 0_u32;
    while reader.read_bit()? == 0 {
        leading_zero_bits += 1;
        if leading_zero_bits > 63 {
            return None;
        }
    }

    let mut value = 1_u64;
    for _ in 0..leading_zero_bits {
        value = (value << 1) | u64::from(reader.read_bit()?);
    }
    Some(value - 1)
}

struct BitReader<'a> {
    bytes: &'a [u8],
    bit_index: usize,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            bit_index: 0,
        }
    }

    fn read_bit(&mut self) -> Option<u8> {
        if self.bit_index >= self.bytes.len() * 8 {
            return None;
        }
        let byte = self.bytes[self.bit_index / 8];
        let shift = 7 - (self.bit_index % 8);
        self.bit_index += 1;
        Some((byte >> shift) & 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start_code() -> [u8; 4] {
        [0, 0, 0, 1]
    }

    #[test]
    fn splits_h264_annex_b_access_units_on_aud_boundaries() -> Result<()> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x09, 0xf0]);
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x67, 0x42, 0x00, 0x1f]);
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x65, 0x80]);
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x09, 0xf0]);
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x41, 0x80]);

        let access_units = h264_annex_b_access_units(&stream)?;

        assert_eq!(access_units.len(), 2);
        assert!(access_units[0].keyframe);
        assert!(!access_units[1].keyframe);
        assert!(access_units[0].bytes.starts_with(&start_code()));
        assert!(access_units[1].bytes.starts_with(&start_code()));
        Ok(())
    }

    #[test]
    fn splits_h264_annex_b_access_units_on_new_slice_zero() -> Result<()> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x65, 0x80]);
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x41, 0x80]);

        let access_units = h264_annex_b_access_units(&stream)?;

        assert_eq!(access_units.len(), 2);
        assert!(access_units[0].keyframe);
        assert!(!access_units[1].keyframe);
        Ok(())
    }

    #[test]
    fn video_annex_b_dispatches_h264_aliases() -> Result<()> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x65, 0x80]);

        let access_units = video_annex_b_access_units("AVC", &stream)?;

        assert_eq!(access_units.len(), 1);
        assert!(access_units[0].keyframe);
        Ok(())
    }

    fn h265_nal(nal_type: u8, slice_first: Option<bool>) -> Vec<u8> {
        let mut nal = vec![nal_type << 1, 0x01];
        if let Some(first) = slice_first {
            nal.push(if first { 0x80 } else { 0x00 });
        } else {
            nal.push(0x00);
        }
        nal
    }

    #[test]
    fn splits_h265_annex_b_access_units_on_aud_boundaries() -> Result<()> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&h265_nal(35, None));
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&h265_nal(32, None));
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&h265_nal(33, None));
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&h265_nal(34, None));
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&h265_nal(19, Some(true)));
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&h265_nal(35, None));
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&h265_nal(1, Some(true)));

        let access_units = h265_annex_b_access_units(&stream)?;

        assert_eq!(access_units.len(), 2);
        assert!(access_units[0].keyframe);
        assert!(!access_units[1].keyframe);
        assert!(access_units[0].bytes.starts_with(&start_code()));
        assert!(access_units[1].bytes.starts_with(&start_code()));
        Ok(())
    }

    #[test]
    fn splits_h265_annex_b_access_units_on_first_slice_flag() -> Result<()> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&h265_nal(19, Some(true)));
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&h265_nal(1, Some(true)));

        let access_units = h265_annex_b_access_units(&stream)?;

        assert_eq!(access_units.len(), 2);
        assert!(access_units[0].keyframe);
        assert!(!access_units[1].keyframe);
        Ok(())
    }

    #[test]
    fn video_annex_b_dispatches_hevc_aliases() -> Result<()> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&h265_nal(19, Some(true)));

        let access_units = video_annex_b_access_units("HEVC", &stream)?;

        assert_eq!(access_units.len(), 1);
        assert!(access_units[0].keyframe);
        Ok(())
    }

    #[test]
    fn video_annex_b_rejects_av1_without_obu_packetizer() {
        let error = video_annex_b_access_units("av1", &[1, 2, 3]).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("AV1 access unit splitting is not Annex B")
        );
    }

    #[test]
    fn video_annex_b_rejects_unknown_codec() {
        let error = video_annex_b_access_units("vp9", &[1, 2, 3]).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unsupported Annex B video codec vp9")
        );
    }

    #[test]
    fn packetizes_access_unit_into_typed_video_chunks() -> Result<()> {
        let access_unit = VideoAccessUnit {
            bytes: vec![1, 2, 3, 4, 5],
            keyframe: false,
        };

        let records = packetize_video_access_unit(
            VideoFramePacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "h264",
                frame_id: 9,
                pts_ticks: 27_000,
                duration_ticks: 3_000,
                timebase_num: 1,
                timebase_den: 90_000,
                deadline_ticks: 28_800,
                max_payload_bytes: 2,
            },
            &access_unit,
        )?;

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].chunk_index, 0);
        assert_eq!(records[0].chunk_count, 3);
        assert_eq!(records[0].dependency_frame_id, Some(8));
        assert_eq!(records[2].payload, vec![5]);
        Ok(())
    }

    #[test]
    fn builds_video_parity_shards_for_burst_loss_recovery() -> Result<()> {
        let access_unit = VideoAccessUnit {
            bytes: (1_u8..=40).collect(),
            keyframe: false,
        };
        let records = packetize_video_access_unit(
            VideoFramePacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "h264",
                frame_id: 9,
                pts_ticks: 27_000,
                duration_ticks: 3_000,
                timebase_num: 1,
                timebase_den: 90_000,
                deadline_ticks: 28_800,
                max_payload_bytes: 2,
            },
            &access_unit,
        )?;

        let parity = build_video_parity_shards(&records)?;

        assert_eq!(records.len(), 20);
        assert_eq!(parity.len(), 16);
        assert!(
            parity.iter().enumerate().all(
                |(index, shard)| shard.parity_index == index as u16 && shard.parity_count == 16
            )
        );
        assert!(parity.iter().all(|shard| shard.chunk_payload_bytes == 2));

        let mut recovered_tail = Vec::new();
        for missing_index in 16_u16..20 {
            let shard = parity
                .iter()
                .find(|shard| shard.parity_index == missing_index % shard.parity_count)
                .expect("tail chunk stripe exists");
            let mut recovered = shard.payload.clone();
            for chunk in records.iter().filter(|chunk| {
                chunk.chunk_index != missing_index
                    && chunk.chunk_index % shard.parity_count == shard.parity_index
            }) {
                for (offset, byte) in chunk.payload.iter().enumerate() {
                    recovered[offset] ^= *byte;
                }
            }
            recovered.truncate(records[missing_index as usize].payload.len());
            recovered_tail.extend(recovered);
        }
        let expected_tail: Vec<u8> = records[16..20]
            .iter()
            .flat_map(|record| record.payload.iter().copied())
            .collect();
        assert_eq!(recovered_tail, expected_tail);
        Ok(())
    }

    #[test]
    fn packetizes_annex_b_stream_into_timed_video_records() -> Result<()> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x65, 0x80]);
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x41, 0x80]);

        let records = packetize_video_annex_b_stream(
            VideoAnnexBStreamPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "avc",
                first_frame_id: 9,
                first_pts_ticks: 27_000,
                frame_duration_ticks: 3_000,
                timebase_num: 1,
                timebase_den: 90_000,
                deadline_delay_ticks: 1_800,
                max_payload_bytes: 16,
            },
            &stream,
        )?;

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].frame_id, 9);
        assert_eq!(records[0].pts_ticks, 27_000);
        assert_eq!(records[0].deadline_ticks, 28_800);
        assert!(records[0].keyframe);
        assert_eq!(records[1].frame_id, 10);
        assert_eq!(records[1].pts_ticks, 30_000);
        assert_eq!(records[1].deadline_ticks, 31_800);
        assert_eq!(records[1].dependency_frame_id, Some(9));
        assert_eq!(records[1].codec, "avc");
        Ok(())
    }

    #[test]
    fn packetizes_non_reference_h264_p_frames_without_dependency() -> Result<()> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x65, 0x80]);
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x01, 0x80]);

        let records = packetize_video_annex_b_stream(
            VideoAnnexBStreamPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "h264",
                first_frame_id: 9,
                first_pts_ticks: 27_000,
                frame_duration_ticks: 3_000,
                timebase_num: 1,
                timebase_den: 90_000,
                deadline_delay_ticks: 1_800,
                max_payload_bytes: 16,
            },
            &stream,
        )?;

        assert_eq!(records.len(), 2);
        assert!(records[0].keyframe);
        assert_eq!(records[0].dependency_frame_id, None);
        assert!(!records[1].keyframe);
        assert_eq!(records[1].dependency_frame_id, None);
        Ok(())
    }

    #[test]
    fn rejects_negative_annex_b_stream_deadline_delay() {
        let error = packetize_video_annex_b_stream(
            VideoAnnexBStreamPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "h264",
                first_frame_id: 9,
                first_pts_ticks: 27_000,
                frame_duration_ticks: 3_000,
                timebase_num: 1,
                timebase_den: 90_000,
                deadline_delay_ticks: -1,
                max_payload_bytes: 4,
            },
            &[0, 0, 0, 1, 0x65, 0x80],
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("deadline_delay_ticks must be non-negative")
        );
    }

    #[test]
    fn rejects_zero_duration_video_access_units() {
        let access_unit = VideoAccessUnit {
            bytes: vec![1, 2, 3],
            keyframe: true,
        };

        let error = packetize_video_access_unit(
            VideoFramePacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "h264",
                frame_id: 9,
                pts_ticks: 27_000,
                duration_ticks: 0,
                timebase_num: 1,
                timebase_den: 90_000,
                deadline_ticks: 28_800,
                max_payload_bytes: 2,
            },
            &access_unit,
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("video duration_ticks must be greater than zero")
        );
    }

    #[test]
    fn rejects_video_deadline_before_pts() {
        let access_unit = VideoAccessUnit {
            bytes: vec![1, 2, 3],
            keyframe: true,
        };

        let error = packetize_video_access_unit(
            VideoFramePacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "h264",
                frame_id: 9,
                pts_ticks: 27_000,
                duration_ticks: 3_000,
                timebase_num: 1,
                timebase_den: 90_000,
                deadline_ticks: 26_999,
                max_payload_bytes: 2,
            },
            &access_unit,
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("video deadline_ticks must not precede pts_ticks")
        );
    }

    #[test]
    fn video_chunk_key_round_trips_feedback_key() -> Result<()> {
        let key = VideoChunkKey::parse("42:7")?;

        assert_eq!(key, VideoChunkKey::new(42, 7));
        assert_eq!(key.as_feedback_key(), "42:7");
        assert_eq!(video_chunk_feedback_key(42, 7), "42:7");
        Ok(())
    }

    #[test]
    fn rejects_malformed_video_chunk_keys() {
        let error = VideoChunkKey::parse("42:7:extra").unwrap_err();

        assert!(error.to_string().contains("exactly one"));
    }

    #[test]
    fn packetizes_audio_payload_into_typed_packet() -> Result<()> {
        let record = packetize_audio_packet(
            AudioPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "opus",
                packet_id: 12,
                pts_ticks: 48_000,
                duration_ticks: 960,
                timebase_num: 1,
                timebase_den: 48_000,
                deadline_ticks: 48_960,
            },
            &[0xf8, 0xff, 0xfe],
        )?;

        assert_eq!(record.stream_id, "muninn.raven.av.rudp");
        assert_eq!(record.codec, "opus");
        assert_eq!(record.packet_id, 12);
        assert_eq!(record.payload, vec![0xf8, 0xff, 0xfe]);
        Ok(())
    }

    #[test]
    fn rejects_empty_audio_payloads() {
        let error = packetize_audio_packet(
            AudioPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "opus",
                packet_id: 12,
                pts_ticks: 48_000,
                duration_ticks: 960,
                timebase_num: 1,
                timebase_den: 48_000,
                deadline_ticks: 48_960,
            },
            &[],
        )
        .unwrap_err();

        assert!(error.to_string().contains("payload must be non-empty"));
    }

    #[test]
    fn rejects_zero_duration_audio_packets() {
        let error = packetize_audio_packet(
            AudioPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "opus",
                packet_id: 12,
                pts_ticks: 48_000,
                duration_ticks: 0,
                timebase_num: 1,
                timebase_den: 48_000,
                deadline_ticks: 48_960,
            },
            &[0xf8, 0xff, 0xfe],
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("audio duration_ticks must be greater than zero")
        );
    }

    #[test]
    fn rejects_audio_deadline_before_pts() {
        let error = packetize_audio_packet(
            AudioPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "opus",
                packet_id: 12,
                pts_ticks: 48_000,
                duration_ticks: 960,
                timebase_num: 1,
                timebase_den: 48_000,
                deadline_ticks: 47_999,
            },
            &[0xf8, 0xff, 0xfe],
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("audio deadline_ticks must not precede pts_ticks")
        );
    }

    fn audio_packet(packet_id: u64, deadline_ticks: i64) -> GameCultMediaAudioPacketRecord {
        packetize_audio_packet(
            AudioPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "opus",
                packet_id,
                pts_ticks: (packet_id as i64) * 960,
                duration_ticks: 960,
                timebase_num: 1,
                timebase_den: 48_000,
                deadline_ticks,
            },
            &[0xf8, 0xff, packet_id as u8],
        )
        .unwrap()
    }

    #[test]
    fn audio_packet_buffer_emits_contiguous_packets_in_order() -> Result<()> {
        let mut buffer = AudioPacketBuffer::default();

        buffer.insert(audio_packet(7, 10_000))?;
        buffer.insert(audio_packet(9, 12_000))?;
        let ready = buffer.pop_ready_packets();

        assert_eq!(
            ready
                .iter()
                .map(|packet| packet.packet_id)
                .collect::<Vec<_>>(),
            vec![7]
        );
        assert_eq!(buffer.pending_packet_count(), 1);

        buffer.insert(audio_packet(8, 11_000))?;
        let ready = buffer.pop_ready_packets();

        assert_eq!(
            ready
                .iter()
                .map(|packet| packet.packet_id)
                .collect::<Vec<_>>(),
            vec![8, 9]
        );
        assert_eq!(buffer.pending_packet_count(), 0);
        Ok(())
    }

    #[test]
    fn audio_packet_buffer_tracks_lowest_packet_before_first_emit() -> Result<()> {
        let mut buffer = AudioPacketBuffer::default();

        buffer.insert(audio_packet(9, 12_000))?;
        buffer.insert(audio_packet(7, 10_000))?;
        buffer.insert(audio_packet(8, 11_000))?;

        let ready = buffer.pop_ready_packets();

        assert_eq!(
            ready
                .iter()
                .map(|packet| packet.packet_id)
                .collect::<Vec<_>>(),
            vec![7, 8, 9]
        );
        assert_eq!(buffer.pending_packet_count(), 0);
        Ok(())
    }

    #[test]
    fn audio_packet_buffer_rejects_stale_packets_after_emit() -> Result<()> {
        let mut buffer = AudioPacketBuffer::default();

        buffer.insert(audio_packet(7, 10_000))?;
        assert_eq!(buffer.pop_ready_packets().len(), 1);
        let error = buffer.insert(audio_packet(6, 9_000)).unwrap_err();

        assert!(error.to_string().contains("stale packet_id"));
        Ok(())
    }

    #[test]
    fn audio_packet_buffer_expires_late_packets() -> Result<()> {
        let mut buffer = AudioPacketBuffer::default();

        buffer.insert(audio_packet(7, 10_000))?;
        buffer.insert(audio_packet(8, 12_000))?;

        let expired = buffer.expire_late_packets(10_000);

        assert_eq!(expired, vec![7]);
        assert_eq!(buffer.pending_packet_count(), 1);
        assert_eq!(
            buffer
                .pop_ready_packets()
                .into_iter()
                .map(|packet| packet.packet_id)
                .collect::<Vec<_>>(),
            vec![8]
        );
        Ok(())
    }

    #[test]
    fn audio_packet_buffer_rejects_mixed_stream_metadata() -> Result<()> {
        let mut buffer = AudioPacketBuffer::default();
        let mut mixed = audio_packet(8, 12_000);
        mixed.session_id = "session-2".to_string();

        buffer.insert(audio_packet(7, 10_000))?;
        let error = buffer.insert(mixed).unwrap_err();

        assert!(error.to_string().contains("mixed stream metadata"));
        Ok(())
    }

    #[test]
    fn builds_receiver_feedback_with_sorted_unique_damage_lists() -> Result<()> {
        let feedback = build_receiver_feedback(ReceiverFeedbackOptions {
            stream_id: "muninn.raven.av.rudp",
            session_id: "session-1",
            receiver_id: "starfire.obs",
            highest_decodable_frame_id: Some(40),
            missing_frame_ids: vec![43, 42, 43],
            missing_video_chunk_keys: vec![
                video_chunk_feedback_key(43, 2),
                video_chunk_feedback_key(42, 1),
                video_chunk_feedback_key(42, 1),
            ],
            late_frame_ids: vec![39, 39, 38],
            requested_keyframe: false,
            jitter_us: 700,
            decode_queue_us: 2_000,
            observed_at: "2026-06-18T00:00:00Z",
        })?;

        assert_eq!(feedback.missing_frame_ids, vec![42, 43]);
        assert_eq!(feedback.missing_video_chunk_keys, vec!["42:1", "43:2"]);
        assert_eq!(feedback.late_frame_ids, vec![38, 39]);
        assert!(feedback.requested_keyframe);
        Ok(())
    }

    #[test]
    fn rejects_negative_receiver_feedback_timing() {
        let error = build_receiver_feedback(ReceiverFeedbackOptions {
            stream_id: "muninn.raven.av.rudp",
            session_id: "session-1",
            receiver_id: "starfire.obs",
            highest_decodable_frame_id: Some(40),
            missing_frame_ids: Vec::new(),
            missing_video_chunk_keys: Vec::new(),
            late_frame_ids: Vec::new(),
            requested_keyframe: false,
            jitter_us: -1,
            decode_queue_us: 2_000,
            observed_at: "2026-06-18T00:00:00Z",
        })
        .unwrap_err();

        assert!(error.to_string().contains("jitter_us"));
    }

    #[test]
    fn rejects_malformed_receiver_feedback_chunk_keys() {
        let error = build_receiver_feedback(ReceiverFeedbackOptions {
            stream_id: "muninn.raven.av.rudp",
            session_id: "session-1",
            receiver_id: "starfire.obs",
            highest_decodable_frame_id: Some(40),
            missing_frame_ids: Vec::new(),
            missing_video_chunk_keys: vec!["frame:chunk".to_string()],
            late_frame_ids: Vec::new(),
            requested_keyframe: false,
            jitter_us: 0,
            decode_queue_us: 2_000,
            observed_at: "2026-06-18T00:00:00Z",
        })
        .unwrap_err();

        assert!(error.to_string().contains("frame_id"));
    }

    #[test]
    fn media_wire_round_trips_video_document() -> Result<()> {
        let access_unit = VideoAccessUnit {
            bytes: vec![1, 2, 3],
            keyframe: true,
        };
        let record = packetize_video_access_unit(
            VideoFramePacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "h264",
                frame_id: 9,
                pts_ticks: 27_000,
                duration_ticks: 3_000,
                timebase_num: 1,
                timebase_den: 90_000,
                deadline_ticks: 28_800,
                max_payload_bytes: 4,
            },
            &access_unit,
        )?
        .remove(0);

        let wire = encode_media_wire_record(
            &GameCultMediaWireRecord::Video(record.clone()),
            muninn_provenance("2026-06-18T00:00:00Z", "muninn-test", "media-test"),
        )?;

        assert_eq!(
            decode_media_wire_record(&wire)?,
            GameCultMediaWireRecord::Video(record)
        );
        Ok(())
    }

    #[test]
    fn media_wire_round_trips_video_parity_document() -> Result<()> {
        let access_unit = VideoAccessUnit {
            bytes: vec![1, 2, 3, 4, 5],
            keyframe: false,
        };
        let records = packetize_video_access_unit(
            VideoFramePacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "h264",
                frame_id: 9,
                pts_ticks: 27_000,
                duration_ticks: 3_000,
                timebase_num: 1,
                timebase_den: 90_000,
                deadline_ticks: 28_800,
                max_payload_bytes: 2,
            },
            &access_unit,
        )?;
        let parity = build_video_parity_shards(&records)?
            .into_iter()
            .next()
            .expect("multi-chunk frame gets parity");

        let wire = encode_media_wire_record(
            &GameCultMediaWireRecord::VideoParity(parity.clone()),
            muninn_provenance("2026-06-18T00:00:00Z", "muninn-test", "media-test"),
        )?;

        assert_eq!(
            decode_media_wire_record(&wire)?,
            GameCultMediaWireRecord::VideoParity(parity)
        );
        Ok(())
    }

    #[test]
    fn media_wire_batches_annex_b_video_records() -> Result<()> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x65, 0x80]);
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x41, 0x80]);
        let records = packetize_video_annex_b_stream(
            VideoAnnexBStreamPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "h264",
                first_frame_id: 9,
                first_pts_ticks: 27_000,
                frame_duration_ticks: 3_000,
                timebase_num: 1,
                timebase_den: 90_000,
                deadline_delay_ticks: 1_800,
                max_payload_bytes: 16,
            },
            &stream,
        )?;
        let wire_records = records
            .iter()
            .cloned()
            .map(GameCultMediaWireRecord::Video)
            .collect::<Vec<_>>();

        let wire = encode_media_wire_records(
            &wire_records,
            "2026-06-18T00:00:00Z",
            "muninn-test",
            "media-test",
        )?;

        assert_eq!(wire.len(), 2);
        assert_eq!(decode_media_wire_record(&wire[0])?, wire_records[0]);
        assert_eq!(decode_media_wire_record(&wire[1])?, wire_records[1]);
        Ok(())
    }

    #[test]
    fn media_wire_encodes_annex_b_stream_for_sender() -> Result<()> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x65, 0x80]);
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x41, 0x80]);

        let wire = encode_video_annex_b_stream_wire_records(
            VideoAnnexBStreamWireOptions {
                packetize: VideoAnnexBStreamPacketizeOptions {
                    stream_id: "muninn.raven.av.rudp",
                    session_id: "session-1",
                    codec: "h264",
                    first_frame_id: 9,
                    first_pts_ticks: 27_000,
                    frame_duration_ticks: 3_000,
                    timebase_num: 1,
                    timebase_den: 90_000,
                    deadline_delay_ticks: 1_800,
                    max_payload_bytes: 16,
                },
                stored_at: "2026-06-18T00:00:00Z",
                source_runtime_id: "muninn-test",
                source_role: "media-test",
            },
            &stream,
        )?;

        assert_eq!(wire.len(), 2);
        let first = decode_media_wire_record(&wire[0])?;
        let second = decode_media_wire_record(&wire[1])?;
        let GameCultMediaWireRecord::Video(first) = first else {
            panic!("expected video media record");
        };
        let GameCultMediaWireRecord::Video(second) = second else {
            panic!("expected video media record");
        };
        assert_eq!(first.frame_id, 9);
        assert_eq!(first.deadline_ticks, 28_800);
        assert_eq!(second.frame_id, 10);
        assert_eq!(second.deadline_ticks, 31_800);
        Ok(())
    }

    #[test]
    fn media_send_payloads_pin_video_to_media_channel() -> Result<()> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&start_code());
        stream.extend_from_slice(&[0x65, 0x80]);

        let payloads = video_annex_b_stream_send_payloads(
            VideoAnnexBStreamWireOptions {
                packetize: VideoAnnexBStreamPacketizeOptions {
                    stream_id: "muninn.raven.av.rudp",
                    session_id: "session-1",
                    codec: "h264",
                    first_frame_id: 9,
                    first_pts_ticks: 27_000,
                    frame_duration_ticks: 3_000,
                    timebase_num: 1,
                    timebase_den: 90_000,
                    deadline_delay_ticks: 1_800,
                    max_payload_bytes: 16,
                },
                stored_at: "2026-06-18T00:00:00Z",
                source_runtime_id: "muninn-test",
                source_role: "media-test",
            },
            &stream,
        )?;

        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].channel_id, MUNINN_MEDIA_RUDP_CHANNEL);
        let GameCultMediaWireRecord::Video(record) = decode_media_wire_record(&payloads[0].payload)?
        else {
            panic!("expected video media record");
        };
        assert_eq!(record.frame_id, 9);
        Ok(())
    }

    fn stream_send_config() -> VideoAnnexBStreamSendConfig {
        VideoAnnexBStreamSendConfig {
            stream_id: "muninn.raven.av.rudp".to_string(),
            session_id: "session-1".to_string(),
            codec: "h264".to_string(),
            first_frame_id: 9,
            first_pts_ticks: 27_000,
            frame_duration_ticks: 3_000,
            timebase_num: 1,
            timebase_den: 90_000,
            deadline_delay_ticks: 1_800,
            max_payload_bytes: 16,
            max_pending_bytes: 64,
            source_runtime_id: "muninn-test".to_string(),
            source_role: "media-test".to_string(),
        }
    }


    #[test]
    fn annex_b_stream_send_state_emits_only_completed_frames() -> Result<()> {
        let mut first_frame = Vec::new();
        first_frame.extend_from_slice(&start_code());
        first_frame.extend_from_slice(&[0x65, 0x80]);
        let mut second_frame = Vec::new();
        second_frame.extend_from_slice(&start_code());
        second_frame.extend_from_slice(&[0x41, 0x80]);
        let mut sender = VideoAnnexBStreamSendState::new(stream_send_config())?;

        let first_push = sender.push("2026-06-18T00:00:00Z", &first_frame)?;

        assert!(first_push.is_empty());
        assert_eq!(sender.next_frame_id(), 9);
        assert_eq!(sender.pending_bytes(), first_frame.len());

        let second_push = sender.push("2026-06-18T00:00:00Z", &second_frame)?;

        assert_eq!(second_push.len(), 1);
        assert_eq!(second_push[0].channel_id, MUNINN_MEDIA_RUDP_CHANNEL);
        let GameCultMediaWireRecord::Video(first) =
            decode_media_wire_record(&second_push[0].payload)?
        else {
            panic!("expected video media record");
        };
        assert_eq!(first.frame_id, 9);
        assert_eq!(first.pts_ticks, 27_000);
        assert!(first.keyframe);
        assert_eq!(sender.next_frame_id(), 10);
        assert_eq!(sender.pending_bytes(), second_frame.len());

        let tail = sender.finish("2026-06-18T00:00:00Z")?;

        assert_eq!(tail.len(), 1);
        let GameCultMediaWireRecord::Video(second) = decode_media_wire_record(&tail[0].payload)?
        else {
            panic!("expected video media record");
        };
        assert_eq!(second.frame_id, 10);
        assert_eq!(second.pts_ticks, 30_000);
        assert_eq!(second.dependency_frame_id, Some(9));
        assert_eq!(sender.pending_bytes(), 0);
        assert_eq!(sender.next_frame_id(), 11);
        Ok(())
    }

    #[test]
    fn annex_b_stream_send_state_rejects_non_annex_b_codecs() {
        let mut config = stream_send_config();
        config.codec = "av1".to_string();

        let error = match VideoAnnexBStreamSendState::new(config) {
            Ok(_) => panic!("AV1 must not create an Annex B stream sender"),
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("Annex B stream sender requires H.264/AVC or H.265/HEVC codec")
        );
    }

    #[test]
    fn annex_b_stream_send_state_rejects_unbounded_pending_bytes() -> Result<()> {
        let mut config = stream_send_config();
        config.max_pending_bytes = 4;
        let mut sender = VideoAnnexBStreamSendState::new(config)?;

        let error = sender
            .push("2026-06-18T00:00:00Z", &[0x47, 0x40, 0x00, 0x10, 0x00])
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("pending buffer exceeded 4 bytes")
        );
        Ok(())
    }





    #[test]
    fn media_wire_encodes_audio_packet_for_sender() -> Result<()> {
        let wire = encode_audio_packet_wire_record(
            AudioPacketWireOptions {
                packetize: AudioPacketizeOptions {
                    stream_id: "muninn.raven.av.rudp",
                    session_id: "session-1",
                    codec: "opus",
                    packet_id: 12,
                    pts_ticks: 48_000,
                    duration_ticks: 960,
                    timebase_num: 1,
                    timebase_den: 48_000,
                    deadline_ticks: 48_960,
                },
                stored_at: "2026-06-18T00:00:00Z",
                source_runtime_id: "muninn-test",
                source_role: "media-test",
            },
            &[0xf8, 0xff, 0xfe],
        )?;

        let decoded = decode_media_wire_record(&wire)?;
        let GameCultMediaWireRecord::Audio(decoded) = decoded else {
            panic!("expected audio media record");
        };
        assert_eq!(decoded.packet_id, 12);
        assert_eq!(decoded.codec, "opus");
        assert_eq!(decoded.payload, vec![0xf8, 0xff, 0xfe]);
        Ok(())
    }

    #[test]
    fn media_send_payload_pins_audio_to_media_channel() -> Result<()> {
        let payload = audio_packet_send_payload(
            AudioPacketWireOptions {
                packetize: AudioPacketizeOptions {
                    stream_id: "muninn.raven.av.rudp",
                    session_id: "session-1",
                    codec: "opus",
                    packet_id: 12,
                    pts_ticks: 48_000,
                    duration_ticks: 960,
                    timebase_num: 1,
                    timebase_den: 48_000,
                    deadline_ticks: 48_960,
                },
                stored_at: "2026-06-18T00:00:00Z",
                source_runtime_id: "muninn-test",
                source_role: "media-test",
            },
            &[0xf8, 0xff, 0xfe],
        )?;

        assert_eq!(payload.channel_id, MUNINN_MEDIA_RUDP_CHANNEL);
        let GameCultMediaWireRecord::Audio(record) = decode_media_wire_record(&payload.payload)?
        else {
            panic!("expected audio media record");
        };
        assert_eq!(record.packet_id, 12);
        Ok(())
    }

    #[test]
    fn media_wire_round_trips_audio_document() -> Result<()> {
        let record = packetize_audio_packet(
            AudioPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "opus",
                packet_id: 12,
                pts_ticks: 48_000,
                duration_ticks: 960,
                timebase_num: 1,
                timebase_den: 48_000,
                deadline_ticks: 48_960,
            },
            &[0xf8, 0xff, 0xfe],
        )?;

        let wire = encode_media_wire_record(
            &GameCultMediaWireRecord::Audio(record.clone()),
            muninn_provenance("2026-06-18T00:00:00Z", "muninn-test", "media-test"),
        )?;

        assert_eq!(
            decode_media_wire_record(&wire)?,
            GameCultMediaWireRecord::Audio(record)
        );
        Ok(())
    }

    #[test]
    fn media_wire_round_trips_feedback_document() -> Result<()> {
        let record = build_receiver_feedback(ReceiverFeedbackOptions {
            stream_id: "muninn.raven.av.rudp",
            session_id: "session-1",
            receiver_id: "starfire.obs",
            highest_decodable_frame_id: Some(40),
            missing_frame_ids: vec![42],
            missing_video_chunk_keys: vec![video_chunk_feedback_key(42, 2)],
            late_frame_ids: vec![39],
            requested_keyframe: true,
            jitter_us: 700,
            decode_queue_us: 2_000,
            observed_at: "2026-06-18T00:00:00Z",
        })?;

        let wire = encode_media_wire_record(
            &GameCultMediaWireRecord::Feedback(record.clone()),
            muninn_provenance("2026-06-18T00:00:00Z", "muninn-test", "media-test"),
        )?;

        assert_eq!(
            decode_media_wire_record(&wire)?,
            GameCultMediaWireRecord::Feedback(record)
        );
        Ok(())
    }

    /// A real capture from the deployed C++ OBS bridge, kept as the record of a
    /// deliberate break rather than deleted.
    ///
    /// It carries `muninn.media_receiver_feedback.v1`. The media schemas were
    /// renamed to `gamecult.media_*` because they describe media and nothing
    /// about who produced it, and the `muninn.` prefix was the only thing tying
    /// a general contract to one producer. That rename means the deployed bridge
    /// can no longer be understood, which is accepted: it is being replaced by
    /// Ratatoskr, and a compatibility path accepting both names would preserve
    /// exactly the ownership confusion the rename removes.
    ///
    /// If this ever starts passing, someone has reintroduced the old schema id.
    #[test]
    fn the_previous_generation_bridge_is_deliberately_no_longer_understood() -> Result<()> {
        let wire = hex_fixture(
            "83ad736368656d6156657273696f6ebb63756c746e65742e646f63756d656e745f7075745f7261772e7630a96d6573736167654964d96a6d756e696e6e2d6d656469613a6d756e696e6e2e6d656469615f72656365697665725f666565646261636b2e76313a6d756e696e6e2e726176656e2e61762e727564703a726176656e3a746573743a766964656f3a666565646261636b3a73746172666972652e6f6273a8646f63756d656e7489a8736368656d614964d9216d756e696e6e2e6d656469615f72656365697665725f666565646261636b2e7631a97265636f72644b6579d93b6d756e696e6e2e726176656e2e61762e727564703a726176656e3a746573743a766964656f3a666565646261636b3a73746172666972652e6f6273a873746f7265644174ac756e69782d6d733a31303030af7061796c6f6164456e636f64696e67ab6d6573736167657061636ba77061796c6f6164c4539bb46d756e696e6e2e726176656e2e61762e72756470b0726176656e3a746573743a766964656fac73746172666972652e6f62732990912ac30000ac756e69782d6d733a3130303092a434323a31a434323a33af736f7572636552756e74696d654964a87374617266697265ad736f757263654167656e744964c0aa736f75726365526f6c65a96d696d69722e6f6273a47461677391ac6d756e696e6e2e6d65646961",
        )?;

        let error = decode_media_wire_record(&wire)
            .expect_err("the muninn.* media schemas are retired");
        assert!(
            error.to_string().contains("muninn.media_receiver_feedback.v1"),
            "the refusal should name what it refused, got {error}"
        );
        Ok(())
    }

    #[test]
    fn media_wire_rejects_invalid_video_record_timing() -> Result<()> {
        let access_unit = VideoAccessUnit {
            bytes: vec![1, 2, 3],
            keyframe: true,
        };
        let mut record = packetize_video_access_unit(
            VideoFramePacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "h264",
                frame_id: 9,
                pts_ticks: 27_000,
                duration_ticks: 3_000,
                timebase_num: 1,
                timebase_den: 90_000,
                deadline_ticks: 28_800,
                max_payload_bytes: 4,
            },
            &access_unit,
        )?
        .remove(0);
        record.deadline_ticks = 26_999;
        let wire = encode_media_wire_record(
            &GameCultMediaWireRecord::Video(record),
            muninn_provenance("2026-06-18T00:00:00Z", "muninn-test", "media-test"),
        )?;

        let error = decode_media_wire_record(&wire).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("video media record deadline_ticks must not precede pts_ticks")
        );
        Ok(())
    }

    #[test]
    fn media_wire_rejects_invalid_audio_record_payload() -> Result<()> {
        let mut record = packetize_audio_packet(
            AudioPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "opus",
                packet_id: 12,
                pts_ticks: 48_000,
                duration_ticks: 960,
                timebase_num: 1,
                timebase_den: 48_000,
                deadline_ticks: 48_960,
            },
            &[0xf8, 0xff, 0xfe],
        )?;
        record.payload.clear();
        let wire = encode_media_wire_record(
            &GameCultMediaWireRecord::Audio(record),
            muninn_provenance("2026-06-18T00:00:00Z", "muninn-test", "media-test"),
        )?;

        let error = decode_media_wire_record(&wire).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("audio media record payload must be non-empty")
        );
        Ok(())
    }

    #[test]
    fn media_wire_rejects_invalid_feedback_record_pressure() -> Result<()> {
        let mut record = build_receiver_feedback(ReceiverFeedbackOptions {
            stream_id: "muninn.raven.av.rudp",
            session_id: "session-1",
            receiver_id: "starfire.obs",
            highest_decodable_frame_id: Some(40),
            missing_frame_ids: vec![42],
            missing_video_chunk_keys: vec![video_chunk_feedback_key(42, 2)],
            late_frame_ids: vec![39],
            requested_keyframe: true,
            jitter_us: 700,
            decode_queue_us: 2_000,
            observed_at: "2026-06-18T00:00:00Z",
        })?;
        record.decode_queue_us = -1;
        let wire = encode_media_wire_record(
            &GameCultMediaWireRecord::Feedback(record),
            muninn_provenance("2026-06-18T00:00:00Z", "muninn-test", "media-test"),
        )?;

        let error = decode_media_wire_record(&wire).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("receiver feedback media record decode_queue_us must be non-negative")
        );
        Ok(())
    }

    #[test]
    fn feedback_payload_decodes_legacy_record_without_chunk_keys() -> Result<()> {
        #[derive(serde::Serialize)]
        struct LegacyFeedbackRecord {
            stream_id: String,
            session_id: String,
            receiver_id: String,
            highest_decodable_frame_id: Option<u64>,
            missing_frame_ids: Vec<u64>,
            late_frame_ids: Vec<u64>,
            requested_keyframe: bool,
            jitter_us: i64,
            decode_queue_us: i64,
            observed_at: String,
        }

        let payload = encode_record_payload(&LegacyFeedbackRecord {
            stream_id: "muninn.raven.av.rudp".to_string(),
            session_id: "session-1".to_string(),
            receiver_id: "starfire.obs".to_string(),
            highest_decodable_frame_id: Some(41),
            missing_frame_ids: vec![42],
            late_frame_ids: vec![40],
            requested_keyframe: true,
            jitter_us: 750,
            decode_queue_us: 2_000,
            observed_at: "2026-06-18T00:00:00Z".to_string(),
        })?;

        let decoded: GameCultMediaReceiverFeedbackRecord = decode_record_payload(&payload)?;

        assert_eq!(decoded.missing_video_chunk_keys, Vec::<String>::new());
        assert_eq!(decoded.missing_frame_ids, vec![42]);
        assert!(decoded.requested_keyframe);
        Ok(())
    }

    #[test]
    fn media_wire_rejects_mismatched_record_key() -> Result<()> {
        let record = packetize_audio_packet(
            AudioPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "opus",
                packet_id: 12,
                pts_ticks: 48_000,
                duration_ticks: 960,
                timebase_num: 1,
                timebase_den: 48_000,
                deadline_ticks: 48_960,
            },
            &[0xf8, 0xff, 0xfe],
        )?;
        let wire = encode_media_wire_record(
            &GameCultMediaWireRecord::Audio(record),
            muninn_provenance("2026-06-18T00:00:00Z", "muninn-test", "media-test"),
        )?;
        let CultNetMessage::DocumentPutRaw {
            message_id,
            mut document,
        } = decode_cultnet_message_from_slice(&wire, CultNetWireContract::CultNetSchemaV0)?
        else {
            panic!("expected raw document put");
        };
        document.record_key = "wrong:key".to_string();
        let tampered = encode_cultnet_message_to_vec(
            &CultNetMessage::DocumentPutRaw {
                message_id,
                document,
            },
            CultNetWireContract::CultNetSchemaV0,
        )?;

        let error = decode_media_wire_record(&tampered).unwrap_err();

        assert!(error.to_string().contains("record key mismatch"));
        Ok(())
    }

    #[test]
    fn media_wire_rejects_unsupported_schema() -> Result<()> {
        let record = packetize_audio_packet(
            AudioPacketizeOptions {
                stream_id: "muninn.raven.av.rudp",
                session_id: "session-1",
                codec: "opus",
                packet_id: 12,
                pts_ticks: 48_000,
                duration_ticks: 960,
                timebase_num: 1,
                timebase_den: 48_000,
                deadline_ticks: 48_960,
            },
            &[0xf8, 0xff, 0xfe],
        )?;
        let wire = encode_media_wire_record(
            &GameCultMediaWireRecord::Audio(record),
            muninn_provenance("2026-06-18T00:00:00Z", "muninn-test", "media-test"),
        )?;
        let CultNetMessage::DocumentPutRaw {
            message_id,
            mut document,
        } = decode_cultnet_message_from_slice(&wire, CultNetWireContract::CultNetSchemaV0)?
        else {
            panic!("expected raw document put");
        };
        document.schema_id = "muninn.media_unknown.v1".to_string();
        let tampered = encode_cultnet_message_to_vec(
            &CultNetMessage::DocumentPutRaw {
                message_id,
                document,
            },
            CultNetWireContract::CultNetSchemaV0,
        )?;

        let error = decode_media_wire_record(&tampered).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("unsupported media schema")
        );
        Ok(())
    }

    #[test]
    fn media_wire_rejects_non_raw_document_messages() -> Result<()> {
        let wire = encode_cultnet_message_to_vec(
            &CultNetMessage::DocumentPut {
                message_id: "not-raw-media".to_string(),
                document: cultnet_rs::CultNetDocumentRecord {
                    schema_id: GAMECULT_MEDIA_AUDIO_PACKET_SCHEMA.to_string(),
                    record_key: "muninn.raven.av.rudp:session-1:audio:12".to_string(),
                    stored_at: "2026-06-18T00:00:00Z".to_string(),
                    payload: serde_json::json!({ "packet_id": 12 }),
                    source_runtime_id: Some("muninn-test".to_string()),
                    source_agent_id: None,
                    source_role: Some("media-test".to_string()),
                    tags: Some(vec!["muninn.media".to_string()]),
                },
            },
            CultNetWireContract::CultNetSchemaV0,
        )?;

        let error = decode_media_wire_record(&wire).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("expected cultnet.document_put_raw.v0")
        );
        Ok(())
    }

    fn hex_fixture(value: &str) -> Result<Vec<u8>> {
        if value.len() % 2 != 0 {
            return Err(anyhow!("hex fixture must have an even length"));
        }
        let mut bytes = Vec::with_capacity(value.len() / 2);
        for index in (0..value.len()).step_by(2) {
            let byte = u8::from_str_radix(&value[index..index + 2], 16)
                .with_context(|| format!("parsing hex fixture byte at offset {index}"))?;
            bytes.push(byte);
        }
        Ok(bytes)
    }
}
