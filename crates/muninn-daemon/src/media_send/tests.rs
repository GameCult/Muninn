use super::*;
use crate::media_packetizer::{MediaWireProvenance, encode_media_wire_record};
use cultnet_rs::{
    CultNetRudpPacketType, CultNetRudpSocketTransportConnection, CultNetRudpSocketTransportOptions,
    CultNetTransportDelivery, ReceiverFeedbackOptions, build_receiver_feedback, decode_rudp_packet,
};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const CLIP: &[u8] = include_bytes!("../../testdata/clip.h264");
const STREAM_ID: &str = "muninn.harness.rudp";
const VIDEO_SESSION_ID: &str = "harness:video";
const AUDIO_SESSION_ID: &str = "harness:audio";
const WAIT: Duration = Duration::from_secs(5);

fn wait_until(timeout: Duration, mut done: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if done() {
            return true;
        }
        thread::sleep(Duration::from_millis(1));
    }
    done()
}

fn video_config() -> VideoAnnexBStreamSendConfig {
    VideoAnnexBStreamSendConfig {
        stream_id: STREAM_ID.to_string(),
        session_id: VIDEO_SESSION_ID.to_string(),
        codec: "h264".to_string(),
        first_frame_id: 0,
        first_pts_ticks: 0,
        frame_duration_ticks: 3_000,
        timebase_num: 1,
        timebase_den: 90_000,
        deadline_delay_ticks: 22_500,
        max_payload_bytes: 848,
        max_pending_bytes: 848 * 4096,
        source_runtime_id: "harness".to_string(),
        source_role: "muninn.rudp.video".to_string(),
    }
}

fn audio_config() -> AudioPcmStreamSendConfig {
    AudioPcmStreamSendConfig {
        stream_id: STREAM_ID.to_string(),
        session_id: AUDIO_SESSION_ID.to_string(),
        codec: "pcm-f32le-interleaved".to_string(),
        first_packet_id: 0,
        first_pts_ticks: 0,
        packet_duration_ticks: 480,
        timebase_num: 1,
        timebase_den: 48_000,
        deadline_delay_ticks: 24_000,
        channels: 2,
        bytes_per_sample: 4,
        max_pending_bytes: 480 * 2 * 4 * 128,
        source_runtime_id: "harness".to_string(),
        source_role: "muninn.rudp.audio".to_string(),
    }
}

/// Ten audio packets of an ascending f32 ramp: every byte position is
/// distinct, so a reordered or dropped packet cannot pass for the ramp.
fn pcm_ramp() -> Vec<u8> {
    let samples = 480 * 2 * 10;
    (0..samples)
        .flat_map(|index| (index as f32 / samples as f32).to_le_bytes())
        .collect()
}

/// The clip's access units as the packetizer sends them: one group each.
fn clip_groups() -> Vec<MuninnMediaPayloadGroup> {
    let mut sender = VideoAnnexBStreamSendState::new(video_config()).unwrap();
    let mut groups = sender.push("unix-0", CLIP).unwrap();
    groups.extend(sender.finish("unix-0").unwrap());
    groups
}

fn group_at(
    kind: MediaKind,
    produced_at: Instant,
    policy: &MediaSendPolicy,
    payloads: MuninnMediaPayloadGroup,
) -> QueuedMediaGroup {
    QueuedMediaGroup::new(kind, produced_at, policy, payloads)
}

fn marker_payload(marker: u8) -> MuninnMediaSendPayload {
    MuninnMediaSendPayload {
        channel_id: MUNINN_MEDIA_RUDP_CHANNEL,
        payload: vec![marker],
    }
}

/// `(frame_id, is_parity, chunk or parity index)` of a video wire record.
fn frame_of(payload: &[u8]) -> Option<(u64, bool, u16)> {
    match decode_media_wire_record(payload).ok()? {
        GameCultMediaWireRecord::Video(record) => Some((record.frame_id, false, record.chunk_index)),
        GameCultMediaWireRecord::VideoParity(record) => {
            Some((record.frame_id, true, record.parity_index))
        }
        _ => None,
    }
}

fn test_hub(policy: &MediaSendPolicy) -> CultNetRudpServerHub {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.set_nonblocking(true).unwrap();
    CultNetRudpServerHub::new(crate::muninn_media_rudp_hub_options(
        socket,
        &crate::muninn_rudp_media_profile(),
        policy,
    ))
    .unwrap()
}

/// The request's budget and no pacing sleeps: the harness is about what the
/// core decides, not about how fast a Windows sender is allowed to go.
fn quiet_policy(latency_budget_ms: u32) -> MediaSendPolicy {
    let mut policy = MediaSendPolicy::from_request(latency_budget_ms);
    policy.video_pace_sleep = Duration::ZERO;
    policy
}

/// One datagram the hub sent toward the receiver, as the relay saw it.
#[derive(Clone, Debug)]
struct Sent {
    packet_type: CultNetRudpPacketType,
    channel: String,
    payload: Vec<u8>,
    dropped: bool,
}

/// A UDP relay between the receiver and the hub. The hub's data packets are
/// dropped by a seeded schedule. Everything is logged before the drop, so
/// tests assert on what the sender put on the wire.
struct DropRelay {
    addr: SocketAddr,
    log: Arc<Mutex<Vec<Sent>>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl DropRelay {
    fn start(hub_addr: SocketAddr, drop_percent: u64, seed: u64) -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(1)))
            .unwrap();
        let addr = socket.local_addr().unwrap();
        let log = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let log = Arc::clone(&log);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                let mut client: Option<SocketAddr> = None;
                let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
                let mut buffer = vec![0_u8; 65_535];
                while !stop.load(Ordering::Relaxed) {
                    let Ok((received, from)) = socket.recv_from(&mut buffer) else {
                        continue;
                    };
                    if from != hub_addr {
                        client = Some(from);
                        let _ = socket.send_to(&buffer[..received], hub_addr);
                        continue;
                    }
                    let Some(client) = client else { continue };
                    let Ok(packet) = decode_rudp_packet(&buffer[..received]) else {
                        continue;
                    };
                    let is_data = packet.packet_type == CultNetRudpPacketType::Data;
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    let dropped = is_data && state % 100 < drop_percent;
                    log.lock().unwrap().push(Sent {
                        packet_type: packet.packet_type,
                        channel: packet.channel_id,
                        payload: packet.payload,
                        dropped,
                    });
                    if !dropped {
                        let _ = socket.send_to(&buffer[..received], client);
                    }
                }
            })
        };
        Self {
            addr,
            log,
            stop,
            thread: Some(thread),
        }
    }

    fn data(&self) -> Vec<Sent> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .filter(|sent| sent.packet_type == CultNetRudpPacketType::Data)
            .cloned()
            .collect()
    }

    fn video_frames_on_wire(&self) -> Vec<(u64, bool, u16)> {
        self.data()
            .iter()
            .filter(|sent| sent.channel == MUNINN_MEDIA_RUDP_CHANNEL)
            .filter_map(|sent| frame_of(&sent.payload))
            .collect()
    }
}

impl Drop for DropRelay {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Rig {
    core: MediaSendCore,
    client: CultNetRudpSocketTransportConnection,
    relay: DropRelay,
    policy: MediaSendPolicy,
}

fn rig(latency_budget_ms: u32, drop_percent: u64, seed: u64) -> Rig {
    let policy = quiet_policy(latency_budget_ms);
    let hub = test_hub(&policy);
    let relay = DropRelay::start(hub.local_addr().unwrap(), drop_percent, seed);
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.set_nonblocking(true).unwrap();
    let mut options = CultNetRudpSocketTransportOptions::client(
        "harness-receiver",
        socket,
        relay.addr,
        crate::MUNINN_MEDIA_RUDP_CONNECTION_ID,
    );
    options.media_delivery = Some(CultNetTransportDelivery::Unreliable);
    options.max_fragment_bytes = Some(crate::MUNINN_RUDP_MEDIA_MAX_FRAGMENT_BYTES as u32);
    let mut client = CultNetRudpSocketTransportConnection::new(options).unwrap();
    let mut core = MediaSendCore::new(hub, &policy);
    client.connect(b"harness-receiver".to_vec()).unwrap();
    assert!(
        wait_until(WAIT, || {
            core.service_control(Instant::now()).unwrap();
            let _ = client.receive_once().unwrap();
            client.connected() && core.hub().sessions().len() == 1
        }),
        "receiver never attached through the relay"
    );
    Rig {
        core,
        client,
        relay,
        policy,
    }
}

// ---- policy, groups and queues -----------------------------------------------------

#[test]
fn a_group_is_expired_only_after_its_deadline_and_never_at_it() {
    let policy = quiet_policy(250);
    let produced_at = Instant::now();
    let group = group_at(MediaKind::Video, produced_at, &policy, Vec::new());

    assert!(!group.expired(group.deadline));
    assert!(!group.expired(group.deadline - Duration::from_millis(1)));
    assert!(group.expired(group.deadline + Duration::from_millis(1)));
}

#[test]
fn a_zero_request_budget_still_leaves_the_stream_a_millisecond() {
    assert_eq!(
        MediaSendPolicy::from_request(0).latency_budget,
        Duration::from_millis(1)
    );
}

#[test]
fn the_queue_caps_groups_per_kind_and_drops_the_oldest_first() {
    let policy = quiet_policy(250);
    let mut core = MediaSendCore::new(test_hub(&policy), &policy);
    let t0 = Instant::now();
    let over = 37;
    for index in 0..(MEDIA_SEND_VIDEO_GROUP_CAP + over) {
        core.enqueue(group_at(
            MediaKind::Video,
            t0 + Duration::from_millis(index as u64),
            &policy,
            vec![marker_payload(1)],
        ));
    }
    for index in 0..(MEDIA_SEND_AUDIO_GROUP_CAP + 5) {
        core.enqueue(group_at(
            MediaKind::Audio,
            t0 + Duration::from_millis(index as u64),
            &policy,
            vec![marker_payload(2)],
        ));
    }

    assert_eq!(
        core.queued_groups(MediaKind::Video),
        MEDIA_SEND_VIDEO_GROUP_CAP
    );
    assert_eq!(
        core.queued_groups(MediaKind::Audio),
        MEDIA_SEND_AUDIO_GROUP_CAP
    );
    // One byte per group: resident bytes are bounded by the cap times the group size.
    assert_eq!(
        core.queued_payload_bytes(MediaKind::Video),
        MEDIA_SEND_VIDEO_GROUP_CAP
    );
    assert_eq!(
        core.queued_payload_bytes(MediaKind::Audio),
        MEDIA_SEND_AUDIO_GROUP_CAP
    );
    assert_eq!(core.stats().groups_dropped_video, over as u64);
    assert_eq!(core.stats().groups_dropped_audio, 5);
    // The survivors are the newest: the oldest deadline left is the 38th group's.
    assert_eq!(
        core.oldest_deadline(MediaKind::Video),
        Some(t0 + Duration::from_millis(over as u64) + policy.latency_budget)
    );
    assert_eq!(
        core.oldest_deadline(MediaKind::Audio),
        Some(t0 + Duration::from_millis(5) + policy.latency_budget)
    );
}

#[test]
fn the_intake_channel_holds_the_reader_at_its_bound_instead_of_growing() {
    let policy = quiet_policy(5_000);
    let (tx, rx) = media_intake_channel();
    let frames = clip_groups().len();
    assert!(frames > MEDIA_GROUP_CHANNEL_BOUND + 1);
    let reader = spawn_video_group_reader(tx, Cursor::new(CLIP.to_vec()), video_config(), policy);

    // Nothing drains, so the reader parks with the channel full.
    thread::sleep(Duration::from_millis(300));
    assert!(
        !reader.is_finished(),
        "an unbounded channel let the reader run to the end"
    );

    let mut received = 0;
    wait_until(WAIT, || {
        while rx.try_recv().is_ok() {
            received += 1;
        }
        received == frames
    });
    assert_eq!(received, frames);
    assert!(wait_until(WAIT, || reader.is_finished()));
}

#[test]
fn a_blocked_send_gives_up_at_the_group_deadline_and_not_before() {
    let start = Instant::now();
    let deadline = start + Duration::from_millis(3);
    let ticks = std::cell::Cell::new(0_u64);
    let clock = || {
        ticks.set(ticks.get() + 1);
        start + Duration::from_millis(ticks.get())
    };
    let mut polls = 0_u32;

    // Blocked forever: the deadline ends it, after polling resends in between.
    let sent = send_until(
        &mut polls,
        deadline,
        clock,
        |_| Err(std::io::Error::from(ErrorKind::WouldBlock).into()),
        |polls| {
            *polls += 1;
            Ok(())
        },
    )
    .unwrap();
    assert!(!sent);
    assert_eq!(polls, 3, "one poll per blocked attempt inside the deadline");

    // Blocked twice, then through: sent, without waiting for the deadline.
    ticks.set(0);
    let mut attempts = 0_u32;
    let sent = send_until(
        &mut attempts,
        start + Duration::from_millis(100),
        clock,
        |attempts| {
            *attempts += 1;
            if *attempts < 3 {
                Err(std::io::Error::from(ErrorKind::WouldBlock).into())
            } else {
                Ok(())
            }
        },
        |_| Ok(()),
    )
    .unwrap();
    assert!(sent);
    assert_eq!(attempts, 3);
}

#[test]
fn a_send_error_that_is_not_backpressure_is_an_error_not_a_dropped_group() {
    let error = send_until(
        &mut (),
        Instant::now() + Duration::from_secs(1),
        Instant::now,
        |_| Err(anyhow::anyhow!("socket closed")),
        |_| Ok(()),
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("socket closed"));
}

// ---- moved from the mux: repair budget, feedback intake, repair cache ---------------

#[test]
fn rudp_repair_budget_backs_off_on_media_drops_and_recovers_when_stable() {
    let start = Instant::now();
    let mut budget = MuninnRudpRepairBudget {
        chunks_per_second: 64,
        min_chunks_per_second: 8,
        max_chunks_per_second: 128,
        add_chunks_per_second: 8,
        recovery_interval: Duration::from_secs(2),
        max_available_chunks: 4,
        available_chunks: 4,
        last_refill_at: start,
        last_rate_adjust_at: start,
        last_queue_dropped: 0,
    };

    assert_eq!(budget.take(4, start, 0), 4);
    assert_eq!(budget.chunks_per_second(), 64);
    assert_eq!(budget.take(3, start + Duration::from_millis(10), 1), 0);
    assert_eq!(budget.chunks_per_second(), 32);
    assert_eq!(budget.take(3, start + Duration::from_secs(1), 1), 3);
    assert_eq!(budget.chunks_per_second(), 32);
    assert_eq!(budget.take(3, start + Duration::from_secs(3), 1), 3);
    assert_eq!(budget.chunks_per_second(), 40);
}

#[test]
fn default_rudp_repair_budget_has_lan_stream_headroom() {
    let mut budget =
        MuninnRudpRepairBudget::new(REPAIR_INITIAL_CHUNKS_PER_SECOND, REPAIR_BURST_CHUNKS);
    let start = budget.last_refill_at;

    assert_eq!(budget.chunks_per_second(), 4_096);
    assert_eq!(budget.take(2_048, start, 0), 2_048);
    assert_eq!(budget.take(8_192, start + Duration::from_secs(1), 0), 2_048);
    assert_eq!(budget.take(8_192, start + Duration::from_secs(3), 0), 2_048);
    assert_eq!(budget.chunks_per_second(), 6_144);
    assert_eq!(budget.take(512, start + Duration::from_secs(4), 1), 512);
    assert_eq!(budget.chunks_per_second(), 3_072);
}

fn feedback_for(
    missing_chunk_keys: Vec<String>,
    requested_keyframe: bool,
) -> GameCultMediaReceiverFeedbackRecord {
    build_receiver_feedback(ReceiverFeedbackOptions {
        stream_id: STREAM_ID,
        session_id: VIDEO_SESSION_ID,
        receiver_id: "harness-receiver",
        highest_decodable_frame_id: Some(41),
        missing_frame_ids: Vec::new(),
        missing_video_chunk_keys: missing_chunk_keys,
        late_frame_ids: vec![42, 43],
        requested_keyframe,
        jitter_us: 500,
        decode_queue_us: 2_000,
        observed_at: "unix:1000",
    })
    .unwrap()
}

fn feedback_wire(feedback: GameCultMediaReceiverFeedbackRecord) -> Vec<u8> {
    encode_media_wire_record(
        &GameCultMediaWireRecord::Feedback(feedback),
        MediaWireProvenance {
            stored_at: "unix:1000",
            runtime_id: "harness",
            role: "harness.receiver",
            producer: "harness",
        },
    )
    .unwrap()
}

#[test]
fn rudp_media_receiver_feedback_updates_sender_pressure_stats() {
    let frame = CultNetTransportFrame {
        channel_id: MUNINN_MEDIA_RUDP_CHANNEL.to_string(),
        payload: feedback_wire(feedback_for(vec!["42:1".into(), "42:3".into()], true)),
    };
    let mut stats = MuninnRudpReceiverFeedbackStats::default();
    let repair_cache = RecentVideoChunkRepairCache::new(16);

    let repairs =
        record_receiver_feedback(&frame, &mut stats, &repair_cache, Instant::now()).unwrap();

    assert_eq!(stats.feedback_records, 1);
    assert_eq!(stats.requested_keyframes, 1);
    assert_eq!(stats.late_frames, 2);
    assert_eq!(stats.missing_video_chunks, 2);
    assert_eq!(stats.repaired_video_chunks, 0);
    assert_eq!(stats.highest_decodable_frame_id, Some(41));
    assert!(repairs.is_empty());
}

#[test]
fn repair_cache_returns_recent_missing_video_chunks_with_their_frames_deadline() {
    let groups = clip_groups();
    let payload = groups[0][0].clone();
    let (frame_id, _, chunk_index) = frame_of(&payload.payload).unwrap();
    let feedback = feedback_for(
        vec![
            video_chunk_feedback_key(frame_id, chunk_index),
            "999:0".to_string(),
        ],
        false,
    );
    let deadline = Instant::now() + Duration::from_millis(250);

    let mut cache = RecentVideoChunkRepairCache::new(16);
    cache.remember(&payload, deadline).unwrap();
    let repairs = cache.repair_payloads_for_feedback(&feedback, deadline);

    assert_eq!(repairs, vec![(payload, deadline)]);
}

// ---- the wire: whole groups, repairs inside their frame's deadline ------------------

#[test]
fn a_group_past_its_deadline_puts_nothing_on_the_wire_and_a_live_one_puts_all_of_it() {
    let mut rig = rig(250, 0, 1);
    let groups = clip_groups();
    let stale = groups[0].clone();
    let live = groups
        .iter()
        .skip(1)
        .find(|group| group.len() > 2)
        .expect("the clip has a later multi-chunk frame")
        .clone();
    let live_frame = frame_of(&live[0].payload).unwrap().0;
    let t0 = Instant::now();
    let budget = rig.policy.latency_budget;
    rig.core
        .enqueue(group_at(MediaKind::Video, t0, &rig.policy, stale));
    rig.core.enqueue(group_at(
        MediaKind::Video,
        t0 + 5 * budget,
        &rig.policy,
        live.clone(),
    ));

    // Two budgets after the stale group was made, inside the live group's.
    assert!(rig.core.send_next(t0 + 2 * budget).unwrap());

    assert_eq!(rig.core.stats().groups_expired, 1);
    assert_eq!(rig.core.stats().groups_sent, 1);
    assert!(wait_until(WAIT, || {
        rig.relay.video_frames_on_wire().len() == live.len()
    }));
    thread::sleep(Duration::from_millis(100));
    let on_wire = rig.relay.video_frames_on_wire();
    assert_eq!(
        on_wire.len(),
        live.len(),
        "every payload of the live group, and no other"
    );
    assert!(on_wire.iter().all(|(frame, _, _)| *frame == live_frame));
    assert!(rig.core.progress_detail().contains("groups_sent=1"));
    assert!(rig.core.progress_detail().contains("groups_expired=1"));
}

#[test]
fn audio_leaves_before_video_on_the_wire() {
    let mut rig = rig(250, 0, 1);
    let t0 = Instant::now();
    let video = clip_groups().remove(1);
    let mut audio_sender = AudioPcmStreamSendState::new(audio_config()).unwrap();
    let audio = audio_sender.push("unix-0", &pcm_ramp()).unwrap().remove(0);
    rig.core
        .enqueue(group_at(MediaKind::Video, t0, &rig.policy, video));
    rig.core
        .enqueue(group_at(MediaKind::Audio, t0, &rig.policy, vec![audio]));

    assert!(rig.core.send_next(t0).unwrap());
    assert!(rig.core.send_next(t0).unwrap());

    assert!(wait_until(WAIT, || !rig.relay.data().is_empty()));
    assert_eq!(rig.relay.data()[0].channel, "audio");
}

/// Sends the clip's first multi-chunk frame at `t0` and returns its deadline,
/// its frame id and how many payloads it put on the wire.
fn send_repairable_frame(rig: &mut Rig, t0: Instant) -> (Instant, u64, usize) {
    let group = clip_groups()
        .into_iter()
        .find(|group| group.len() > 2)
        .unwrap();
    let (frame_id, _, _) = frame_of(&group[0].payload).unwrap();
    let queued = group_at(MediaKind::Video, t0, &rig.policy, group.clone());
    let deadline = queued.deadline;
    rig.core.enqueue(queued);
    assert!(rig.core.send_next(t0).unwrap());
    assert!(wait_until(WAIT, || rig.relay.video_frames_on_wire().len()
        == group.len()));
    (deadline, frame_id, group.len())
}

fn ask_for_chunk(rig: &mut Rig, frame_id: u64, at: Instant, records_seen: u64) {
    rig.client
        .send(
            MUNINN_MEDIA_RUDP_CHANNEL,
            feedback_wire(feedback_for(
                vec![video_chunk_feedback_key(frame_id, 0)],
                false,
            )),
        )
        .unwrap();
    assert!(
        wait_until(WAIT, || {
            rig.core.service_control(at).unwrap();
            rig.core.feedback().feedback_records == records_seen
        }),
        "the hub never received the feedback"
    );
}

#[test]
fn a_repair_is_sent_inside_its_frames_deadline_and_not_after_it() {
    // Taken before the handshake, so real time is well past it by the time
    // anything is sent: a repair that took its deadline from the clock
    // instead of its frame would still be alive at the probe below.
    let t0 = Instant::now();
    let mut rig = rig(250, 0, 1);
    let (deadline, frame_id, frame_payloads) = send_repairable_frame(&mut rig, t0);
    let epsilon = Duration::from_millis(2);

    ask_for_chunk(&mut rig, frame_id, deadline - epsilon, 1);
    assert!(
        wait_until(WAIT, || rig.relay.video_frames_on_wire().len()
            == frame_payloads + 1),
        "a repair asked for just before the deadline is sent"
    );
    assert_eq!(rig.core.feedback().repaired_video_chunks, 1);

    ask_for_chunk(&mut rig, frame_id, deadline + epsilon, 2);
    thread::sleep(Duration::from_millis(150));
    assert_eq!(
        rig.relay.video_frames_on_wire().len(),
        frame_payloads + 1,
        "a repair asked for just after the deadline is not sent"
    );
    assert_eq!(rig.core.feedback().repaired_video_chunks, 1);
    assert_eq!(rig.core.feedback().missing_video_chunks, 2);
}

// ---- the harness end to end: the fixture clip and the PCM ramp ---------------------

#[derive(Default)]
struct Received {
    chunks: BTreeMap<u64, BTreeSet<u16>>,
    chunk_counts: BTreeMap<u64, u16>,
    audio: BTreeMap<u64, Vec<u8>>,
}

impl Received {
    fn take(&mut self, rig: &mut Rig) {
        while let Some(frame) = rig.client.receive_once().unwrap() {
            match decode_media_wire_record(&frame.payload).unwrap() {
                GameCultMediaWireRecord::Video(record) => {
                    self.chunks
                        .entry(record.frame_id)
                        .or_default()
                        .insert(record.chunk_index);
                    self.chunk_counts
                        .insert(record.frame_id, record.chunk_count);
                }
                GameCultMediaWireRecord::Audio(record) => {
                    self.audio.insert(record.packet_id, record.payload);
                }
                _ => {}
            }
        }
    }

    fn complete_frames(&self) -> BTreeSet<u64> {
        self.chunks
            .iter()
            .filter(|(frame, chunks)| chunks.len() == self.chunk_counts[frame] as usize)
            .map(|(frame, _)| *frame)
            .collect()
    }
}

/// Feeds the fixture through the real readers and the send loop, over the
/// relay, and returns what the receiver assembled.
fn run_clip(rig: &mut Rig, with_audio: bool) -> Received {
    let (tx, rx) = media_intake_channel();
    let video = spawn_video_group_reader(
        tx.clone(),
        Cursor::new(CLIP.to_vec()),
        video_config(),
        rig.policy.clone(),
    );
    let audio = with_audio.then(|| {
        spawn_audio_group_reader(
            tx.clone(),
            Cursor::new(pcm_ramp()),
            audio_config(),
            rig.policy.clone(),
        )
    });
    drop(tx);
    let mut received = Received::default();
    let start = Instant::now();
    while !rig.core.finished() && start.elapsed() < WAIT {
        rig.core.intake(&rx, Duration::from_millis(2)).unwrap();
        rig.core.send_next(Instant::now()).unwrap();
        rig.core.service_control(Instant::now()).unwrap();
        received.take(rig);
    }
    assert!(rig.core.finished(), "the send loop never finished the clip");
    let settle = Instant::now();
    while settle.elapsed() < Duration::from_millis(300) {
        rig.core.service_control(Instant::now()).unwrap();
        received.take(rig);
        thread::sleep(Duration::from_millis(1));
    }
    video.join().unwrap();
    if let Some(audio) = audio {
        audio.join().unwrap();
    }
    received
}

#[test]
fn the_clip_and_the_pcm_ramp_arrive_whole_through_a_lossless_relay() {
    // A budget far above the run time: nothing here may expire.
    let mut rig = rig(5_000, 0, 1);
    let frames = clip_groups().len() as u64;

    let received = run_clip(&mut rig, true);

    assert_eq!(
        received.complete_frames(),
        (0..frames).collect::<BTreeSet<_>>()
    );
    assert_eq!(rig.core.stats().groups_lost(), 0);
    let heard: Vec<u8> = received.audio.values().flatten().copied().collect();
    assert_eq!(received.audio.len(), 10);
    assert_eq!(heard, pcm_ramp(), "the ramp arrived complete and in order");
}

#[test]
fn the_seeded_relay_drops_the_same_datagrams_every_run_and_the_sender_carries_on() {
    let run = |seed| {
        let mut rig = rig(5_000, 30, seed);
        let received = run_clip(&mut rig, false);
        let dropped: Vec<bool> = rig.relay.data().iter().map(|sent| sent.dropped).collect();
        (received.complete_frames(), dropped, rig.core.stats().clone())
    };

    let (complete_a, dropped_a, stats_a) = run(7);
    let (complete_b, dropped_b, _) = run(7);
    let (_, dropped_other, _) = run(8);
    let frames = clip_groups().len();

    assert_eq!(dropped_a, dropped_b, "same seed, same drop schedule");
    assert_ne!(
        dropped_a, dropped_other,
        "a different seed drops different datagrams"
    );
    assert_eq!(complete_a, complete_b);
    assert!(dropped_a.iter().any(|dropped| *dropped));
    assert!(
        complete_a.len() < frames,
        "30% loss without parity or repair leaves incomplete frames"
    );
    assert_eq!(stats_a.groups_sent, frames as u64);
    assert_eq!(
        stats_a.groups_lost(),
        0,
        "loss on the wire is not a sender-side drop"
    );
}

#[test]
fn a_group_is_counted_by_what_the_deadline_let_reach_the_wire() {
    let mut stats = MediaSendStats::default();

    stats.record_group(&GroupSend::Complete, 6);
    assert_eq!((stats.groups_sent, stats.payloads_sent), (1, 6));

    // Deadline passed before the first payload: nothing left, so nothing was cut.
    stats.record_group(&GroupSend::DeadlinePassed { payloads_out: 0 }, 6);
    assert_eq!((stats.groups_expired, stats.groups_cut_short), (1, 0));
    assert_eq!(stats.payloads_sent, 6);

    // Deadline passed with part of the group already out: a partial frame.
    stats.record_group(&GroupSend::DeadlinePassed { payloads_out: 2 }, 6);
    assert_eq!((stats.groups_expired, stats.groups_cut_short), (1, 1));
    assert_eq!(stats.payloads_sent, 8);
    assert_eq!(stats.groups_lost(), 2);
}

#[test]
fn an_oversize_access_unit_reaches_the_sender_as_a_dropped_video_group_not_an_error() {
    let policy = quiet_policy(250);
    let mut core = MediaSendCore::new(test_hub(&policy), &policy);
    let (tx, rx) = media_intake_channel();
    let mut config = video_config();
    config.max_pending_bytes = 8;
    let mut oversize = vec![0, 0, 0, 1, 0x65, 0x80];
    oversize.extend_from_slice(&[7; 20]);
    let reader = spawn_video_group_reader(tx, Cursor::new(oversize), config, policy);

    assert!(wait_until(WAIT, || reader.is_finished()));
    core.intake(&rx, Duration::from_millis(50)).unwrap();
    core.intake(&rx, Duration::from_millis(50)).unwrap();

    assert_eq!(core.stats().groups_dropped_video, 1);
    assert!(core.finished(), "the stream ended cleanly, it was not restarted by an error");
}
