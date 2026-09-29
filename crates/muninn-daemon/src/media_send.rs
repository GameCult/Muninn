//! The media send path: the one place that decides when a frame is too old.
//!
//! **Owner.** `MediaSendCore` owns the bounded group queues, the hub, the
//! receiver-feedback intake, the repair cache and its budget, and the progress
//! counters. `run_rudp_mux_once` spawns the encoders and publishes; it feeds
//! this module bytes and never decides a deadline.
//!
//! **One deadline owner.** `MediaSendPolicy::latency_budget` comes from the
//! request's `latency_budget_ms` and is the only source of a deadline. A group
//! is stamped `produced_at + latency_budget` when it leaves the packetizer; a
//! repair inherits the deadline of the frame it repairs; the hub's reliable
//! expiry and the record deadline ticks derive from the same budget.
//!
//! **Priority.** The send loop moves one payload per step and chooses the
//! payload afresh each time: audio first, then repairs, then video. Audio that
//! arrives while a video group is half sent, or while the socket is full,
//! goes out before the video group's next payload; nothing holds audio behind
//! video for longer than one payload. The group deadline is checked before
//! every payload; a group that outlives it mid-send is abandoned and counted
//! `groups_cut_short`.
//!
//! **Bounds.** Between the encoder and the socket every queue has a count
//! bound (`MEDIA_GROUP_CHANNEL_BOUND`, the per-kind group caps) and a time
//! bound (the group deadline). Crossing either drops whole groups, oldest
//! first, and counts them. The video cap is 512 groups of at most one access
//! unit each, and an access unit is at most `max_pending_bytes` (848 * 4096
//! bytes at the default packet size), so the count cap alone implies a worst
//! case of about 1.7 GiB; in practice the deadline empties the queue long
//! before that.

use crate::media_packetizer::{
    AudioPcmStreamSendConfig, AudioPcmStreamSendState, GameCultMediaWireRecord,
    MUNINN_MEDIA_RUDP_CHANNEL, MuninnMediaPayloadGroup, MuninnMediaSendPayload,
    VideoAnnexBStreamSendConfig, VideoAnnexBStreamSendState, decode_media_wire_record,
    video_chunk_feedback_key,
};
use anyhow::{Context, Result};
use cultnet_rs::{
    CultNetRudpServerEvent, CultNetRudpServerHub, CultNetRudpServerSessionContext,
    CultNetTransportFrame, GameCultMediaReceiverFeedbackRecord,
};
use std::collections::{HashMap, VecDeque};
use std::io::{ErrorKind, Read};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

pub const MEDIA_SEND_AUDIO_GROUP_CAP: usize = 256;
pub const MEDIA_SEND_VIDEO_GROUP_CAP: usize = 512;
/// The longest latency budget a request may claim (the old default). Above
/// this the queues stop being bounded in time.
pub const MEDIA_SEND_MAX_LATENCY_BUDGET_MS: u32 = 2_000;
/// Groups the reader threads may have handed over and the send loop not yet
/// taken. Small on purpose: the caps above are where a backlog lives.
pub const MEDIA_GROUP_CHANNEL_BOUND: usize = 8;

const REPAIR_CACHE_CHUNKS: usize = 16_384;
pub const REPAIR_BURST_CHUNKS: usize = 2_048;
pub const REPAIR_INITIAL_CHUNKS_PER_SECOND: usize = 4_096;
const REPAIR_MIN_CHUNKS_PER_SECOND: usize = 8;
const REPAIR_MAX_CHUNKS_PER_SECOND: usize = 16_384;
const REPAIR_ADD_CHUNKS_PER_SECOND: usize = 2_048;
const REPAIR_RECOVERY_INTERVAL_MS: u64 = 2_000;
const REPAIR_MAX_FEEDBACK_PER_POLL: usize = 32;
const REPAIR_MAX_CHUNKS_PER_POLL: usize = 256;
/// A receiver that has said nothing for this long (Ratatoskr pings every
/// second or so) is dropped so its payloads stop being sent into the void.
const RECEIVER_TIMEOUT_MS: u64 = 6_000;
pub const SEND_PACE_EVERY_PAYLOADS: usize = 4;
pub const SEND_PACE_SLEEP_US: u64 = 250;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Video,
    Audio,
}

/// What the request decided about sending. Nothing else mints a deadline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaSendPolicy {
    pub latency_budget: Duration,
    pub video_pace_every_payloads: usize,
    pub video_pace_sleep: Duration,
}

impl MediaSendPolicy {
    pub fn from_request(latency_budget_ms: u32) -> Self {
        Self {
            latency_budget: Duration::from_millis(u64::from(
                latency_budget_ms.clamp(1, MEDIA_SEND_MAX_LATENCY_BUDGET_MS),
            )),
            video_pace_every_payloads: SEND_PACE_EVERY_PAYLOADS,
            video_pace_sleep: Duration::from_micros(SEND_PACE_SLEEP_US),
        }
    }

    pub fn latency_budget_ms(&self) -> u64 {
        self.latency_budget.as_millis() as u64
    }

    /// The video record deadline, in the 90 kHz video timebase.
    pub fn video_deadline_delay_ticks(&self) -> i64 {
        i64::try_from(self.latency_budget_ms().saturating_mul(90)).unwrap_or(i64::MAX)
    }

    /// The audio record deadline, in samples at `sample_rate`.
    pub fn audio_deadline_delay_ticks(&self, sample_rate: u32) -> i64 {
        if sample_rate == 0 {
            return 1_024;
        }
        i64::try_from(
            self.latency_budget_ms()
                .saturating_mul(u64::from(sample_rate))
                / 1_000,
        )
        .unwrap_or(i64::MAX)
        .max(1_024)
    }
}

/// One access unit with its parity, or one audio packet. It is sent whole or
/// not at all, and it dies at `deadline`.
pub struct QueuedMediaGroup {
    pub kind: MediaKind,
    pub deadline: Instant,
    pub payloads: MuninnMediaPayloadGroup,
}

impl QueuedMediaGroup {
    pub fn new(
        kind: MediaKind,
        produced_at: Instant,
        policy: &MediaSendPolicy,
        payloads: MuninnMediaPayloadGroup,
    ) -> Self {
        Self {
            kind,
            deadline: produced_at + policy.latency_budget,
            payloads,
        }
    }

    /// Sendable at the deadline itself, expired one instant after.
    pub fn expired(&self, now: Instant) -> bool {
        now > self.deadline
    }
}

/// What a reader thread hands the send loop.
pub enum MediaIntake {
    Group(QueuedMediaGroup),
    /// Video access units the packetizer dropped for outgrowing its ceiling.
    VideoDropped(u64),
}

pub type MediaIntakeSender = mpsc::SyncSender<Result<MediaIntake>>;
pub type MediaIntakeReceiver = mpsc::Receiver<Result<MediaIntake>>;

pub fn media_intake_channel() -> (MediaIntakeSender, MediaIntakeReceiver) {
    mpsc::sync_channel(MEDIA_GROUP_CHANNEL_BOUND)
}

#[derive(Default)]
struct PendingMediaGroups {
    audio: VecDeque<QueuedMediaGroup>,
    video: VecDeque<QueuedMediaGroup>,
}

impl PendingMediaGroups {
    /// `true` when the queue was at its cap and its oldest group was dropped
    /// to make room.
    fn push(&mut self, group: QueuedMediaGroup) -> bool {
        let (queue, cap) = match group.kind {
            MediaKind::Audio => (&mut self.audio, MEDIA_SEND_AUDIO_GROUP_CAP),
            MediaKind::Video => (&mut self.video, MEDIA_SEND_VIDEO_GROUP_CAP),
        };
        let dropped = queue.len() >= cap;
        if dropped {
            queue.pop_front();
        }
        queue.push_back(group);
        dropped
    }

    fn is_empty(&self) -> bool {
        self.audio.is_empty() && self.video.is_empty()
    }
}

/// Counted where it happens. `groups_dropped_*` is a cap crossing;
/// `groups_expired` is a group whose deadline passed with none of it sent;
/// `groups_cut_short` is one whose deadline passed with part of it already on
/// the wire, which is the only way a partial frame can leave.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MediaSendStats {
    pub groups_sent: u64,
    pub payloads_sent: u64,
    pub groups_dropped_audio: u64,
    pub groups_dropped_video: u64,
    pub groups_expired: u64,
    pub groups_cut_short: u64,
}

impl MediaSendStats {
    pub fn groups_lost(&self) -> u64 {
        self.groups_dropped_audio
            + self.groups_dropped_video
            + self.groups_expired
            + self.groups_cut_short
    }

    /// Accounts for one group the send loop has finished with.
    fn record_group(&mut self, outcome: &GroupSend, group_payloads: usize) {
        match outcome {
            GroupSend::Complete => {
                self.groups_sent += 1;
                self.payloads_sent += group_payloads as u64;
            }
            GroupSend::DeadlinePassed { payloads_out: 0 } => self.groups_expired += 1,
            GroupSend::DeadlinePassed { payloads_out } => {
                self.payloads_sent += *payloads_out as u64;
                self.groups_cut_short += 1;
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MuninnRudpReceiverFeedbackStats {
    pub feedback_records: u64,
    pub requested_keyframes: u64,
    pub late_frames: u64,
    pub missing_video_chunks: u64,
    pub repaired_video_chunks: u64,
    pub deferred_repair_chunks: u64,
    pub repair_chunks_per_second: usize,
    pub highest_decodable_frame_id: Option<u64>,
}

#[derive(Debug)]
pub struct MuninnRudpRepairBudget {
    chunks_per_second: usize,
    min_chunks_per_second: usize,
    max_chunks_per_second: usize,
    add_chunks_per_second: usize,
    recovery_interval: Duration,
    max_available_chunks: usize,
    available_chunks: usize,
    last_refill_at: Instant,
    last_rate_adjust_at: Instant,
    last_queue_dropped: u64,
}

impl MuninnRudpRepairBudget {
    pub fn new(chunks_per_second: usize, max_available_chunks: usize) -> Self {
        let now = Instant::now();
        let max_available_chunks = max_available_chunks.max(1);
        Self {
            chunks_per_second: chunks_per_second.max(1),
            min_chunks_per_second: REPAIR_MIN_CHUNKS_PER_SECOND,
            max_chunks_per_second: REPAIR_MAX_CHUNKS_PER_SECOND,
            add_chunks_per_second: REPAIR_ADD_CHUNKS_PER_SECOND,
            recovery_interval: Duration::from_millis(REPAIR_RECOVERY_INTERVAL_MS),
            max_available_chunks,
            available_chunks: max_available_chunks,
            last_refill_at: now,
            last_rate_adjust_at: now,
            last_queue_dropped: 0,
        }
    }

    pub fn chunks_per_second(&self) -> usize {
        self.chunks_per_second
    }

    pub fn take(&mut self, requested_chunks: usize, now: Instant, queue_dropped: u64) -> usize {
        self.adjust_rate(now, queue_dropped, requested_chunks);
        self.refill(now);
        let allowed = requested_chunks.min(self.available_chunks);
        self.available_chunks -= allowed;
        allowed
    }

    fn adjust_rate(&mut self, now: Instant, queue_dropped: u64, requested_chunks: usize) {
        if queue_dropped > self.last_queue_dropped {
            self.chunks_per_second = (self.chunks_per_second / 2).max(self.min_chunks_per_second);
            self.available_chunks = self.available_chunks.min(self.chunks_per_second);
            self.last_queue_dropped = queue_dropped;
            self.last_rate_adjust_at = now;
            return;
        }
        self.last_queue_dropped = queue_dropped;
        if requested_chunks == 0
            || now.saturating_duration_since(self.last_rate_adjust_at) < self.recovery_interval
        {
            return;
        }
        if self.chunks_per_second < self.max_chunks_per_second {
            self.chunks_per_second = self
                .chunks_per_second
                .saturating_add(self.add_chunks_per_second)
                .min(self.max_chunks_per_second);
            self.last_rate_adjust_at = now;
        }
    }

    fn refill(&mut self, now: Instant) {
        let elapsed_ms = now
            .saturating_duration_since(self.last_refill_at)
            .as_millis() as usize;
        if elapsed_ms == 0 {
            return;
        }
        let refill_chunks = elapsed_ms.saturating_mul(self.chunks_per_second) / 1_000;
        if refill_chunks == 0 {
            return;
        }
        self.available_chunks = self
            .available_chunks
            .saturating_add(refill_chunks)
            .min(self.max_available_chunks);
        self.last_refill_at = now;
    }
}

#[derive(Debug)]
struct RepairEntry {
    payload: MuninnMediaSendPayload,
    /// The deadline of the frame group the chunk left in. A repair never
    /// gets a deadline of its own.
    deadline: Instant,
}

#[derive(Debug)]
pub struct RecentVideoChunkRepairCache {
    max_entries: usize,
    order: VecDeque<String>,
    entries: HashMap<String, RepairEntry>,
}

impl RecentVideoChunkRepairCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            order: VecDeque::new(),
            entries: HashMap::new(),
        }
    }

    pub fn remember(&mut self, payload: &MuninnMediaSendPayload, deadline: Instant) -> Result<()> {
        let Some(key) = video_repair_cache_key_from_payload(payload)? else {
            return Ok(());
        };
        let entry = RepairEntry {
            payload: payload.clone(),
            deadline,
        };
        if self.entries.contains_key(&key) {
            self.entries.insert(key, entry);
            return Ok(());
        }
        self.order.push_back(key.clone());
        self.entries.insert(key, entry);
        while self.entries.len() > self.max_entries {
            let Some(expired) = self.order.pop_front() else {
                break;
            };
            self.entries.remove(&expired);
        }
        Ok(())
    }

    /// The chunks the receiver is missing that are still inside their frame's
    /// deadline at `now`, each with that deadline.
    pub fn repair_payloads_for_feedback(
        &self,
        feedback: &GameCultMediaReceiverFeedbackRecord,
        now: Instant,
    ) -> Vec<(MuninnMediaSendPayload, Instant)> {
        feedback
            .missing_video_chunk_keys
            .iter()
            .filter_map(|chunk_key| {
                self.entries.get(&video_repair_cache_key(
                    &feedback.stream_id,
                    &feedback.session_id,
                    chunk_key,
                ))
            })
            .filter(|entry| now <= entry.deadline)
            .map(|entry| (entry.payload.clone(), entry.deadline))
            .collect()
    }
}

fn video_repair_cache_key(stream_id: &str, session_id: &str, chunk_key: &str) -> String {
    format!("{stream_id}:{session_id}:video:{chunk_key}")
}

fn video_repair_cache_key_from_payload(payload: &MuninnMediaSendPayload) -> Result<Option<String>> {
    if payload.channel_id != MUNINN_MEDIA_RUDP_CHANNEL {
        return Ok(None);
    }
    let GameCultMediaWireRecord::Video(video) = decode_media_wire_record(&payload.payload)? else {
        return Ok(None);
    };
    Ok(Some(video_repair_cache_key(
        &video.stream_id,
        &video.session_id,
        &video_chunk_feedback_key(video.frame_id, video.chunk_index),
    )))
}

#[derive(Debug)]
pub struct MuninnRudpMediaSendPacer {
    payloads_since_pause: usize,
    every_payloads: usize,
    sleep_for: Duration,
}

impl MuninnRudpMediaSendPacer {
    pub fn new(every_payloads: usize, sleep_for: Duration) -> Self {
        Self {
            payloads_since_pause: 0,
            every_payloads: every_payloads.max(1),
            sleep_for,
        }
    }

    fn observe_sent_payload(&mut self) {
        if self.sleep_for.is_zero() {
            return;
        }
        self.payloads_since_pause = self.payloads_since_pause.saturating_add(1);
        if self.payloads_since_pause < self.every_payloads {
            return;
        }
        self.payloads_since_pause = 0;
        thread::sleep(self.sleep_for);
    }
}

pub fn is_would_block_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == ErrorKind::WouldBlock)
            || cause.to_string().contains("os error 10035")
    })
}

/// One payload to one receiver, once. `false` means the socket is full: resends
/// are polled and the caller tries again on its next step, choosing again what
/// is most urgent. It never waits, so a full socket cannot hold a queued audio
/// packet behind video.
fn try_send(
    hub: &mut CultNetRudpServerHub,
    receiver: &CultNetRudpServerSessionContext,
    payload: &MuninnMediaSendPayload,
    socket_full: bool,
) -> Result<bool> {
    let sent = if socket_full {
        Err(std::io::Error::from(ErrorKind::WouldBlock).into())
    } else {
        hub.send(receiver, payload.channel_id, payload.payload.clone())
    };
    match sent {
        Ok(()) => Ok(true),
        Err(error) if is_would_block_error(&error) => {
            poll_resends(hub)?;
            thread::yield_now();
            Ok(false)
        }
        Err(error) => Err(error).context("sending typed Muninn media payload"),
    }
}

fn poll_resends(hub: &mut CultNetRudpServerHub) -> Result<()> {
    match hub.poll_resends() {
        Ok(()) => Ok(()),
        Err(error) if is_would_block_error(&error) => Ok(()),
        Err(error) => Err(error).context("polling Muninn RUDP media resends"),
    }
}

#[derive(Debug)]
enum GroupSend {
    Complete,
    /// The deadline passed after `payloads_out` payloads had gone out.
    DeadlinePassed { payloads_out: usize },
}

/// A group being sent, one payload at a time, to the receivers there were when
/// it started. A payload with nobody to send it to is not a failure; the
/// progress line says how many receivers there are.
struct InFlight {
    group: QueuedMediaGroup,
    receivers: Vec<CultNetRudpServerSessionContext>,
    payload: usize,
    receiver: usize,
}

impl InFlight {
    /// Payloads that have reached at least one receiver.
    fn payloads_out(&self) -> usize {
        self.payload + usize::from(self.receiver > 0)
    }
}

/// A repair chunk waiting for the socket: one payload to the receiver that
/// asked, dead at the deadline of the frame it repairs.
struct PendingRepair {
    receiver: CultNetRudpServerSessionContext,
    payload: MuninnMediaSendPayload,
    deadline: Instant,
}

pub struct MediaSendCore {
    hub: CultNetRudpServerHub,
    queues: PendingMediaGroups,
    repair_cache: RecentVideoChunkRepairCache,
    repair_budget: MuninnRudpRepairBudget,
    video_pacer: MuninnRudpMediaSendPacer,
    audio_pacer: MuninnRudpMediaSendPacer,
    audio_in_flight: Option<InFlight>,
    video_in_flight: Option<InFlight>,
    repairs: VecDeque<PendingRepair>,
    /// Test seam: while set, the socket reports itself full.
    #[cfg(test)]
    pub(crate) socket_full: std::sync::Arc<std::sync::atomic::AtomicBool>,
    stats: MediaSendStats,
    feedback: MuninnRudpReceiverFeedbackStats,
    handled_keyframe_requests: u64,
    disconnected: bool,
}

impl MediaSendCore {
    pub fn new(hub: CultNetRudpServerHub, policy: &MediaSendPolicy) -> Self {
        Self {
            hub,
            queues: PendingMediaGroups::default(),
            repair_cache: RecentVideoChunkRepairCache::new(REPAIR_CACHE_CHUNKS),
            repair_budget: MuninnRudpRepairBudget::new(
                REPAIR_INITIAL_CHUNKS_PER_SECOND,
                REPAIR_BURST_CHUNKS,
            ),
            video_pacer: MuninnRudpMediaSendPacer::new(
                policy.video_pace_every_payloads,
                policy.video_pace_sleep,
            ),
            audio_pacer: MuninnRudpMediaSendPacer::new(0, Duration::ZERO),
            audio_in_flight: None,
            video_in_flight: None,
            repairs: VecDeque::new(),
            #[cfg(test)]
            socket_full: Default::default(),
            stats: MediaSendStats::default(),
            feedback: MuninnRudpReceiverFeedbackStats::default(),
            handled_keyframe_requests: 0,
            disconnected: false,
        }
    }

    #[cfg(test)]
    pub fn stats(&self) -> &MediaSendStats {
        &self.stats
    }

    #[cfg(test)]
    pub fn feedback(&self) -> &MuninnRudpReceiverFeedbackStats {
        &self.feedback
    }

    #[cfg(test)]
    pub fn hub(&self) -> &CultNetRudpServerHub {
        &self.hub
    }

    /// Nothing queued and every reader gone: the encoders have ended.
    pub fn finished(&self) -> bool {
        self.disconnected && !self.has_work()
    }

    fn has_work(&self) -> bool {
        !self.queues.is_empty()
            || self.audio_in_flight.is_some()
            || self.video_in_flight.is_some()
            || !self.repairs.is_empty()
    }

    /// Takes what the reader threads have produced. Waits up to `wait` for the
    /// first group only when nothing is queued to send.
    pub fn intake(&mut self, rx: &MediaIntakeReceiver, wait: Duration) -> Result<()> {
        if self.disconnected {
            return Ok(());
        }
        if !self.has_work() {
            match rx.recv_timeout(wait) {
                Ok(item) => self.accept(item?),
                Err(mpsc::RecvTimeoutError::Timeout) => return Ok(()),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    self.disconnected = true;
                    return Ok(());
                }
            }
        }
        loop {
            match rx.try_recv() {
                Ok(item) => self.accept(item?),
                Err(mpsc::TryRecvError::Empty) => return Ok(()),
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.disconnected = true;
                    return Ok(());
                }
            }
        }
    }

    fn accept(&mut self, item: MediaIntake) {
        match item {
            MediaIntake::Group(group) => self.enqueue(group),
            MediaIntake::VideoDropped(count) => {
                self.stats.groups_dropped_video += count;
                self.log_loss_milestone();
            }
        }
    }

    pub fn enqueue(&mut self, group: QueuedMediaGroup) {
        let kind = group.kind;
        if self.queues.push(group) {
            match kind {
                MediaKind::Audio => self.stats.groups_dropped_audio += 1,
                MediaKind::Video => self.stats.groups_dropped_video += 1,
            }
            self.log_loss_milestone();
        }
    }

    /// One step of the send loop: moves at most one payload toward the wire and
    /// says whether there was anything to do. The next payload is chosen afresh
    /// every step, audio first, then repairs, then video, so a video group in
    /// progress or a full socket never holds audio back. A group whose
    /// deadline has passed at `now`, before or after part of it went out, is
    /// abandoned whole.
    pub fn send_next(&mut self, now: Instant) -> Result<bool> {
        #[cfg(test)]
        let socket_full = self.socket_full.load(std::sync::atomic::Ordering::Relaxed);
        #[cfg(not(test))]
        let socket_full = false;
        if self.step_group(MediaKind::Audio, now, socket_full)?
            || self.step_repair(now, socket_full)?
        {
            return Ok(true);
        }
        self.step_group(MediaKind::Video, now, socket_full)
    }

    fn step_group(&mut self, kind: MediaKind, now: Instant, socket_full: bool) -> Result<bool> {
        let in_flight = match kind {
            MediaKind::Audio => &mut self.audio_in_flight,
            MediaKind::Video => &mut self.video_in_flight,
        };
        if in_flight.is_none() {
            let queue = match kind {
                MediaKind::Audio => &mut self.queues.audio,
                MediaKind::Video => &mut self.queues.video,
            };
            let Some(group) = queue.pop_front() else {
                return Ok(false);
            };
            *in_flight = Some(InFlight {
                group,
                receivers: self.hub.sessions(),
                payload: 0,
                receiver: 0,
            });
        }
        let flight = in_flight.as_mut().expect("in flight was just filled");

        if flight.group.expired(now) {
            let outcome = GroupSend::DeadlinePassed {
                payloads_out: flight.payloads_out(),
            };
            let group_payloads = flight.group.payloads.len();
            *in_flight = None;
            self.stats.record_group(&outcome, group_payloads);
            self.log_loss_milestone();
            return Ok(true);
        }

        if let Some(payload) = flight.group.payloads.get(flight.payload) {
            if let Some(receiver) = flight.receivers.get(flight.receiver) {
                if !try_send(&mut self.hub, receiver, payload, socket_full)? {
                    return Ok(true);
                }
                flight.receiver += 1;
                if flight.receiver < flight.receivers.len() {
                    return Ok(true);
                }
            }
            flight.receiver = 0;
            flight.payload += 1;
            match kind {
                MediaKind::Video => self.video_pacer.observe_sent_payload(),
                MediaKind::Audio => self.audio_pacer.observe_sent_payload(),
            }
            if flight.payload < flight.group.payloads.len() {
                return Ok(true);
            }
        }

        let flight = in_flight.take().expect("in flight is set");
        self.stats
            .record_group(&GroupSend::Complete, flight.group.payloads.len());
        if kind == MediaKind::Video {
            for payload in &flight.group.payloads {
                self.repair_cache.remember(payload, flight.group.deadline)?;
            }
        }
        if self.stats.groups_sent == 1 || self.stats.groups_sent % 900 == 0 {
            eprintln!("{}", self.progress_detail());
        }
        Ok(true)
    }

    /// Sends the oldest queued repair chunk that is still inside its frame's
    /// deadline at `now`; the ones already past it are discarded.
    fn step_repair(&mut self, now: Instant, socket_full: bool) -> Result<bool> {
        while let Some(repair) = self.repairs.front() {
            if now > repair.deadline {
                self.repairs.pop_front();
                continue;
            }
            if try_send(&mut self.hub, &repair.receiver, &repair.payload, socket_full)? {
                self.repairs.pop_front();
                self.feedback.repaired_video_chunks =
                    self.feedback.repaired_video_chunks.saturating_add(1);
                self.video_pacer.observe_sent_payload();
            }
            return Ok(true);
        }
        Ok(false)
    }

    /// Receiver feedback, repairs, keyframe pressure and resends.
    pub fn service_control(&mut self, now: Instant) -> Result<()> {
        self.poll_feedback(now)?;
        if self.feedback.requested_keyframes > self.handled_keyframe_requests {
            self.handled_keyframe_requests = self.feedback.requested_keyframes;
            eprintln!(
                "Muninn RUDP receiver requested a fresh keyframe; continuing current low-latency encoder session until explicit encoder control exists."
            );
        }
        poll_resends(&mut self.hub)
    }

    fn poll_feedback(&mut self, now: Instant) -> Result<()> {
        for gone in self.hub.remove_timed_out_sessions(RECEIVER_TIMEOUT_MS) {
            eprintln!(
                "Muninn media receiver {} at {} went silent and was dropped.",
                String::from_utf8_lossy(&gone.connect_payload),
                gone.remote_addr
            );
        }
        let mut feedback_processed = 0_usize;
        loop {
            match self.hub.receive_event_once() {
                Ok(Some(CultNetRudpServerEvent::Connected { session })) => {
                    eprintln!(
                        "Muninn media receiver {} attached from {}.",
                        String::from_utf8_lossy(&session.connect_payload),
                        session.remote_addr
                    );
                }
                Ok(Some(CultNetRudpServerEvent::Disconnected { session, reason })) => {
                    eprintln!(
                        "Muninn media receiver {} at {} disconnected: {}.",
                        String::from_utf8_lossy(&session.connect_payload),
                        session.remote_addr,
                        String::from_utf8_lossy(&reason)
                    );
                }
                Ok(Some(CultNetRudpServerEvent::Frame { session, frame })) => {
                    if feedback_processed >= REPAIR_MAX_FEEDBACK_PER_POLL {
                        return Ok(());
                    }
                    feedback_processed += 1;
                    let repairs = record_receiver_feedback(
                        &frame,
                        &mut self.feedback,
                        &self.repair_cache,
                        now,
                    )?;
                    let requested = repairs.len();
                    let room = REPAIR_CACHE_CHUNKS.saturating_sub(self.repairs.len());
                    let allowed = self.repair_budget.take(
                        requested.min(REPAIR_MAX_CHUNKS_PER_POLL).min(room),
                        now,
                        self.stats.groups_lost(),
                    );
                    self.feedback.repair_chunks_per_second = self.repair_budget.chunks_per_second();
                    self.feedback.deferred_repair_chunks = self
                        .feedback
                        .deferred_repair_chunks
                        .saturating_add(requested.saturating_sub(allowed) as u64);
                    // Repairs are queued for the receiver that asked, not for
                    // everyone; the send step sends each one inside the
                    // deadline of its own frame, after any waiting audio.
                    self.repairs.extend(
                        repairs
                            .into_iter()
                            .take(allowed)
                            .map(|(payload, deadline)| PendingRepair {
                                receiver: session.clone(),
                                payload,
                                deadline,
                            }),
                    );
                }
                Ok(Some(CultNetRudpServerEvent::Pong { .. })) => {}
                Ok(None) => return Ok(()),
                Err(error) if is_would_block_error(&error) => return Ok(()),
                Err(error) => return Err(error).context("polling Muninn RUDP media feedback"),
            }
        }
    }

    fn log_loss_milestone(&self) {
        let lost = self.stats.groups_lost();
        // Loss can arrive in the hundreds per second; the first and every
        // 300th is enough to see it without flooding the log.
        if lost == 1 || lost % 300 == 0 {
            eprintln!("{}", self.progress_detail());
        }
    }

    pub fn progress_detail(&self) -> String {
        format!(
            "Muninn RUDP media progress: receivers={} groups_sent={} payloads_sent={} groups_dropped_audio={} groups_dropped_video={} groups_expired={} groups_cut_short={} reliable_expired={} receiver_feedback={} receiver_keyframes={} receiver_late_frames={} receiver_missing_chunks={} receiver_repaired_chunks={} receiver_deferred_repairs={} repair_rate={} receiver_highest_decodable={}; pending_audio={} pending_video={}",
            self.hub.sessions().len(),
            self.stats.groups_sent,
            self.stats.payloads_sent,
            self.stats.groups_dropped_audio,
            self.stats.groups_dropped_video,
            self.stats.groups_expired,
            self.stats.groups_cut_short,
            self.hub.stats().reliable_packets_expired,
            self.feedback.feedback_records,
            self.feedback.requested_keyframes,
            self.feedback.late_frames,
            self.feedback.missing_video_chunks,
            self.feedback.repaired_video_chunks,
            self.feedback.deferred_repair_chunks,
            self.feedback.repair_chunks_per_second,
            self.feedback
                .highest_decodable_frame_id
                .map(|frame_id| frame_id.to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.queues.audio.len(),
            self.queues.video.len(),
        )
    }

    #[cfg(test)]
    pub(crate) fn queued_groups(&self, kind: MediaKind) -> usize {
        match kind {
            MediaKind::Audio => self.queues.audio.len(),
            MediaKind::Video => self.queues.video.len(),
        }
    }

    #[cfg(test)]
    pub(crate) fn queued_payload_bytes(&self, kind: MediaKind) -> usize {
        match kind {
            MediaKind::Audio => &self.queues.audio,
            MediaKind::Video => &self.queues.video,
        }
        .iter()
        .flat_map(|group| &group.payloads)
        .map(|payload| payload.payload.len())
        .sum()
    }

    #[cfg(test)]
    pub(crate) fn oldest_deadline(&self, kind: MediaKind) -> Option<Instant> {
        match kind {
            MediaKind::Audio => self.queues.audio.front(),
            MediaKind::Video => self.queues.video.front(),
        }
        .map(|group| group.deadline)
    }
}

fn record_receiver_feedback(
    frame: &CultNetTransportFrame,
    stats: &mut MuninnRudpReceiverFeedbackStats,
    repair_cache: &RecentVideoChunkRepairCache,
    now: Instant,
) -> Result<Vec<(MuninnMediaSendPayload, Instant)>> {
    if frame.channel_id != MUNINN_MEDIA_RUDP_CHANNEL {
        return Ok(Vec::new());
    }

    let GameCultMediaWireRecord::Feedback(feedback) = decode_media_wire_record(&frame.payload)?
    else {
        return Ok(Vec::new());
    };

    let repairs = repair_cache.repair_payloads_for_feedback(&feedback, now);
    stats.feedback_records = stats.feedback_records.saturating_add(1);
    if feedback.requested_keyframe {
        stats.requested_keyframes = stats.requested_keyframes.saturating_add(1);
    }
    stats.late_frames = stats
        .late_frames
        .saturating_add(feedback.late_frame_ids.len() as u64);
    stats.missing_video_chunks = stats
        .missing_video_chunks
        .saturating_add(feedback.missing_video_chunk_keys.len() as u64);
    if let Some(frame_id) = feedback.highest_decodable_frame_id {
        stats.highest_decodable_frame_id = Some(
            stats
                .highest_decodable_frame_id
                .map(|current| current.max(frame_id))
                .unwrap_or(frame_id),
        );
    }
    Ok(repairs)
}

/// Reads Annex B from `reader`, packetizes it, and hands one group per access
/// unit to the send loop, stamped with the request's deadline.
pub fn spawn_video_group_reader<R>(
    tx: MediaIntakeSender,
    mut reader: R,
    config: VideoAnnexBStreamSendConfig,
    policy: MediaSendPolicy,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        if let Err(error) = read_video_groups(&tx, &mut reader, config, &policy) {
            let _ = tx.send(Err(error));
        }
    })
}

pub fn spawn_audio_group_reader<R>(
    tx: MediaIntakeSender,
    mut reader: R,
    config: AudioPcmStreamSendConfig,
    policy: MediaSendPolicy,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        if let Err(error) = read_audio_groups(&tx, &mut reader, config, &policy) {
            let _ = tx.send(Err(error));
        }
    })
}

fn send_intake(tx: &MediaIntakeSender, item: MediaIntake) -> Result<()> {
    tx.send(Ok(item)).context("queueing typed Muninn media")
}

fn read_video_groups<R: Read>(
    tx: &MediaIntakeSender,
    reader: &mut R,
    config: VideoAnnexBStreamSendConfig,
    policy: &MediaSendPolicy,
) -> Result<()> {
    let mut sender = VideoAnnexBStreamSendState::new(config)?;
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut dropped_reported = 0_u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .context("reading encoded Annex B video from ffmpeg stdout")?;
        let stored_at = crate::timestamp()?;
        let (groups, done) = if read == 0 {
            (sender.finish(&stored_at)?, true)
        } else {
            (sender.push(&stored_at, &buffer[..read])?, false)
        };
        for payloads in groups {
            send_intake(
                tx,
                MediaIntake::Group(QueuedMediaGroup::new(
                    MediaKind::Video,
                    Instant::now(),
                    policy,
                    payloads,
                )),
            )?;
        }
        let dropped = sender.dropped_access_units();
        if dropped > dropped_reported {
            send_intake(tx, MediaIntake::VideoDropped(dropped - dropped_reported))?;
            dropped_reported = dropped;
        }
        if done {
            return Ok(());
        }
    }
}

fn read_audio_groups<R: Read>(
    tx: &MediaIntakeSender,
    reader: &mut R,
    config: AudioPcmStreamSendConfig,
    policy: &MediaSendPolicy,
) -> Result<()> {
    let mut sender = AudioPcmStreamSendState::new(config)?;
    let mut buffer = vec![0_u8; 16 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .context("reading PCM audio from ffmpeg stdout")?;
        let stored_at = crate::timestamp()?;
        let (payloads, done) = if read == 0 {
            (sender.finish(&stored_at)?, true)
        } else {
            (sender.push(&stored_at, &buffer[..read])?, false)
        };
        for payload in payloads {
            send_intake(
                tx,
                MediaIntake::Group(QueuedMediaGroup::new(
                    MediaKind::Audio,
                    Instant::now(),
                    policy,
                    vec![payload],
                )),
            )?;
        }
        if done {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests;
