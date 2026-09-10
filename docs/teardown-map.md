# Muninn Teardown Map (pre-rebuild)

Date: 2026-09-09. Working map for extracting Muninn from Odin into a dedicated repo.

## What Muninn is (operator, 2026-09-09)

A local capture broker. It resolves what data streams a host can offer, advertises
them, and serves them to CultMesh peers on request.

- Muninn running does NOT mean Muninn is broadcasting.
- Each accessible feed is available for another service to request.
- Raven screen/loopback -> the stream. Input -> Sleipnir. Move + Quest sensors -> Mimir.

Independently confirmed in gamecult-ops `scripts/idunn/idunn-deployment-targets.ps1`,
`raven-muninn.Reason`: keepalive is explicitly "without activating A/V capture".

## Current body

`F:\Projects\Odin\crates\muninn-daemon` -> bin `muninn`.
- `src/main.rs` 14,567 lines
- `src/media_packetizer.rs` 4,244 lines
Sibling crates: `muninn-move-tracker` (308), `muninn-psmoveapi-tracker` (359).

CultLib pin: `c13b6ba0` (2026-09-05), 3 commits behind CultLib HEAD `d1a9edad`.

## Authorities currently fused in main.rs

Eight bespoke verticals, not eight instances of one contract:
1. A/V capture + RUDP media transport
2. PS Move optical tracking (V4L2, YUYV->luma, marker candidates)
3. PS Move Bluetooth pairing + host claiming (Linux + Windows HID)
4. PS Move LED hue programs  <-- ACTUATOR, not capture. Only authority with no home.
5. HID / joystick / XInput controller state
6. Quest access probing (adb)
7. Odin provider lease / catalog / health
8. ~20-subcommand CLI

Each invents its own enumeration, lifecycle, health and publication. That is why
the file is 14.5k lines: every new sensor was a new vertical.

## Rebuild target

A source-resolver contract: enumerate, describe, grant, stream, report health.
Screen, loopback, camera, Move HID, Move optical, XInput, Quest are implementations.
Daemon core owns request -> grant -> lease -> transport, nothing else.
Adding a sensor touches one file and zero core code.

## Schema ownership (decided)

19 `muninn.*` schemas, 38 declarations, currently in `odin-core/src/documents.rs`.
Muninn's contracts owned by Odin = inversion. Move to CultLib, following
`a539d4f` (Idunn/Odin authority contracts returned to CultLib) and
`c13b6ba` (IdunnDaemonHealthTrustBindingRecord into cultnet-rs).
Odin and Muninn both consume; neither owns the other's wire shape.

Schemas: capture_stream, capture_stream_command, command_boundary,
hid_controller_state, media_audio_packet, media_receiver_feedback,
media_video_access_unit, media_video_parity_shard.v2, move_controller_state,
move_evidence_transport_health, move_hue_program, move_identity,
move_light_command, move_marker_candidate, move_tracker_health,
obs_stream_catalog, quest_access, telemetry_surface, transport_profile.

## Coupling

Muninn -> odin-core: ONE import block, 20 symbols (main.rs).
odin-core / odin-daemon / sleipnir-daemon -> Muninn: NONE.
Crate boundary cuts clean.

## Blast radius (extraction breaks these)

gamecult-ops `scripts/idunn/idunn-deployment-targets.ps1` — three runtime-enforced
targets, all `Repo = "Odin"`, `LocalPath = $projectsRoot\Odin`, restart scripts
resolved under Odin's `scripts\`:
- `starfire-muninn`  — Quest access + local telemetry
- `raven-muninn`     — remote telemetry posture, A/V NOT part of keepalive
- `nightwing-muninn` — Move HID daemon
All three `Deploy = $null` (continuity-only; nothing is Idunn-deployed).

Also: gamecult-ops `inventory.md`; 17 scripts in `Odin/scripts/`;
`GameCult-Muninn-Activate` scheduled task; `deploy-raven-muninn-binary.ps1`.

Mimir docs reference Muninn but are drifted (16 stale `E:\Projects` paths,
a `.ps1` that is actually `.cmd`, a wrapper script that does not exist).
Mimir's truth to fix, not part of this cut.

## Protocol ownership (operator ruling, 2026-09-09)

The media protocol is NOT Muninn's. It is a **CultMesh Media Stream**: a generic
typed contract that Muninn advertises as one producer, and that any consumer
resolves through Odin discovery without knowing Muninn exists.

Proof at record level — the media records carry zero producer-specific fields:
- `MuninnMediaAudioPacketRecord`: stream_id, session_id, packet_id, codec,
  pts_ticks, duration_ticks, timebase_num/den, deadline_ticks, payload
- `MuninnMediaVideoAccessUnitRecord`: + frame_id, keyframe,
  dependency_frame_id, chunk_index
The `Muninn` prefix was the entire coupling. odin-core accepted the deposit
because there was nowhere else to put it.

### Revised schema decomposition

-> CultLib, as the CultMesh Media Stream contract (producer-agnostic):
   media_audio_packet, media_video_access_unit, media_video_parity_shard.v2,
   media_receiver_feedback

-> Muninn, genuinely its own:
   capture_stream, capture_stream_command, command_boundary, telemetry_surface,
   move_controller_state, move_evidence_transport_health, move_hue_program,
   move_identity, move_light_command, move_marker_candidate, move_tracker_health,
   quest_access, hid_controller_state, + the input-binding profile record

-> DELETE, do not move:
   obs_stream_catalog — a producer publishing a catalog shaped around one
   consumer's lowering (urls, media_target_host, media_port, bitrate). Odin's
   generic discovery replaces it. Also removes obs-catalog-status,
   publish_obs_catalog, publish_obs_catalog_idle, pull_odin_obs_catalog_snapshot.

### Naming landmine

`MuninnTransportProfileCompatRecord` is NOT a media transport profile. Its fields
are device_filter, axis_map, button_map, pending_learn, presentation — it is a
GAMEPAD INPUT-BINDING profile. Two unrelated meanings of "transport profile" in
one namespace. Rename wherever it lands.

### The OBS bridge

Belongs to neither Muninn nor Mimir. It is a CultMesh media lowering — the same
organ shape Hermodr already is for browsers ("resolves providers through Odin's
catalog, reads those providers for their state, renders their surfaces to
browsers"). Swap browser -> OBS, Eve surfaces -> media streams. Mimir consumes a
built bridge; it does not maintain a transport stack.

Current state: `Mimir/native/obs_stem_source/` is 6,515 lines of C++ that
hand-rolls CultNet RUDP (muninn_rudp_ack.h, muninn_rudp_fragments.h,
muninn_udp_socket.h, muninn_rudp_url.h, muninn_media_wire.h, muninn_audio_fec.h,
muninn_video_fec.h) and links NO CultLib. Every CultNet fix from Aug-Sep 2026
reached cultnet-rs and cultnet-ts and could not reach it.

CORRECTION to the "audio decay" section below: being current on the CultLib pin
fixes the SENDER only. The receiver is a separate codebase that must be replaced,
not bumped. Precedent for the fix: muninn-move-tracker already builds
crate-type = ["rlib", "cdylib", "staticlib"] so C can call it. The bridge should
be a thin OBS-shaped shim over a Rust CultNet core.

Checked and NOT a bug: the C++ receiver correctly keeps separate ack_tracker_ and
audio_ack_tracker_ dispatched by connection_id (mimir_obs_muninn_source.cpp:1068).
The naive "shared ACK domain starves audio" theory is false. Decay mechanism
remains UNPROVEN pending soak.

### Genericity caveat

Muninn is the only media producer today. The plurality is on the CONSUMER axis
(OBS bridge, Mimir compositor, Mimir.VerseRecorder, possible Hermodr browser
preview). Shape the contract by consumers that exist. Do not design for imagined
future producers.

## Dead code found in build (36 warnings, none cfg-gated except Bluetooth)

Two abandoned machines, not scattered rot:
1. Entire AAC/ADTS audio path: AudioAdtsStreamSendState, AudioAdtsStreamSendConfig,
   AudioPacketBuffer (+5 methods), CompleteAdtsFrames, complete_adts_frames.
   Residue of 2026-06-23 PCM switch.
2. Entire second video path incl. a receiver: packetize_video_annex_b_stream,
   encode_video_annex_b_stream_wire_records, reassemble_video_access_unit,
   VideoFrameAssemblySet (+4 methods), VideoFrameKey, VideoFrameAssembly,
   build_receiver_feedback, build_feedback_for_expired_video_frames,
   ExpiredVideoFrame(+FeedbackOptions), ReceiverFeedbackOptions.
   A dead Rust receiver inside the sender, while the live receiver is C++ in Mimir.
   The dead Rust copy is the fossil of the right architecture.

Bluetooth dead-code warnings ARE platform-gated (unix cfg on a Windows build).
Ignore those.

## Build baseline

2026-09-09, Starfire, current pin: `cargo build --release -p muninn-daemon`
exit 0, 1m22s, muninn.exe 4,181,504 bytes, 37 warnings (36 dead-code + summary).

## Audio decay — findings

Last media commit: `b13fa7d` 2026-06-23 "Switch Muninn RUDP audio to PCM packets".
Nothing has touched media since; all work from 2026-07-13 on is Move-light.

CultNet landed the matching fixes AFTER that and BEFORE Muninn's current pin:
- `0736ae4` Separate RUDP reliable and lossy sequence domains (08-26)
- `f4dabfe` Bound RUDP reliable transmission windows (08-23)
- `a2e3d3f` Bind reliable windows to the RUDP ACK horizon (08-23)
- `fda51fa` Acknowledge reliable RUDP windows cumulatively (08-23)
- `75c1807` Acknowledge retransmits across RUDP horizons (08-23)
- `bac5be8` / `cbde940` Restore reliable expiry; profile-owned per-channel expiry (09-04)

A reliable window that never advances its horizon starves its own channel while a
lossy channel flows past it. Audio dies, video survives. Matches the report.

Muninn's source already consumes the fixed CultNet and has never been run.
SOAK TEST REQUIRED before designing around this bug.

## Why the decay went unnoticed — instrument defect

`rudp_media_activity_detail` (main.rs:1794) derives its published status string
from `options.capture_video` / `options.capture_audio` — CONFIG FLAGS. While audio
decayed to silence it kept publishing "typed video/audio access units are
publishing over CultNet RUDP media".

`reliable_packets_expired` (main.rs:2855) SUMS video + audio into one number, so
even the single real counter cannot say which channel died.

There is no per-channel audio counter published anywhere.

Rebuild invariant: media health MUST derive from observed transport, never from
requested configuration. Per-channel counters are mandatory, not optional.
Soak instrumentation cannot trust existing telemetry.

## PRIMARY AUDIO DECAY HYPOTHESIS (2026-09-09) — architectural, not transport

Measured facts:
- Audio: `f32le` uncompressed PCM, 48000 Hz x 2 ch x 4 B = **3.07 Mbps**
  (main.rs:10168-10169 defaults; rudp_audio_ffmpeg_args at main.rs:10058)
- The audio ffmpeg invocation is a **NO-OP**: `-f f32le -ar R -ac C -i pipe:0`
  -> `-f f32le -ar R -ac C pipe:1`. Identical in and out. A whole process
  spawned to copy bytes.
- Video: `MUNINN_RUDP_MEDIA_VIDEO_BITRATE_KBPS = 12_000` (main.rs:73), lossy + FEC
- Combined sustained uplink demand: ~15 Mbps
- Audio rides a SEPARATE RELIABLE connection
  (test `rudp_audio_transport_uses_separate_reliable_media_connection`, main.rs:11885)
- Video's media channel asserts `reliable_expire_after_ms == None` (main.rs:11881)
- Raven link: USB-tethered mobile, WireGuard `10.77.0.4`, tether is default route,
  only 10.77.0.0/24 through mesh. Measured RTT 2026-09-09: **77 ms**.
- `MUNINN_RUDP_MEDIA_PACKET_BYTES = 848` — conservative, safely under WireGuard's
  1420 MTU. MTU/fragmentation hypothesis FALSIFIED cheaply.

Mechanism:
~15 Mbps sustained on a mobile tether saturates the uplink. Under congestion the
two channels fail asymmetrically by design:
- video is lossy + FEC -> drops frames, degrades gracefully, never retransmits
- audio is RELIABLE -> retransmits on loss -> ADDS load under congestion ->
  congestion collapse on that channel specifically -> reliable window stalls ->
  progressive decay to silence
Video survives, audio dies. Matches the reported symptom exactly.

Corroborating split-brain defect:
The C++ receiver implements full Reed-Solomon audio FEC and RUNS it —
`muninn_audio_fec.h` (179 lines), `audio_fec_receiver_.insert(...)`,
`audio_fec_data_cache_`, block recovery (mimir_obs_muninn_source.cpp:1392-1425).
The Rust sender NEVER emits audio parity shards. There is no audio parity schema
(only `media_video_parity_shard.v2`). The receiver built recovery machinery for a
sender feature that does not exist, so recovery can never fire.

IMPLICATION: this is NOT a CultNet transport bug. No CultNet improvement fixes it.
The operator's opening hypothesis is answered: NO.

FALSIFIABLE PREDICTION for the soak:
decay tracks uplink saturation. On a wired LAN path it should NOT reproduce;
on the tether it SHOULD. That is the experiment.

Fix direction (cheap):
- Compress audio: Opus @ 128-256 kbps replaces 3.07 Mbps PCM (12-24x reduction).
  The no-op ffmpeg process becomes an actual encoder, earning its existence.
- Move audio OFF the reliable channel to lossy + the FEC the receiver already
  implements. Emit audio parity shards. Both legs then degrade gracefully.
- Removes the reliable channel from the media path entirely.

UNMEASURED: Raven's actual tether uplink capacity. 77 ms RTT is measured;
bandwidth starvation is inferred. The soak proves or kills it.

## Operational constraint

gamecult-ops inventory.md, recorded 2026-06-21: **Raven is a human-used
workstation. Do not reboot, shut down, or force-logoff during recovery work
without explicit live operator approval for that exact action.** It is also
Deru's machine for this stream.

## MEASURED 2026-09-09 — the constraint was never the tether. It is a VPS hairpin.

Raven moved off USB tethering onto a switch beside Starfire. The ops inventory
LAN address `192.168.1.84` is STALE — the whole subnet changed.

Current, measured:
- Starfire LAN: `192.168.178.146`
- Raven LAN:    `192.168.178.165`   (same /24, ARP resolves d8-43-ae-61-ee-ab
                                     directly = same L2 segment, one switch hop)
- Raven mesh:   `10.77.0.4`

`tracert 10.77.0.4` from Starfire:
    1   36 ms   10.77.0.1      <- Yggdrasil VPS (159.195.114.249)
    2   75 ms   10.77.0.4      <- Raven, on the same physical switch

Traffic between two machines on one switch is hairpinning through a VPS,
crossing the public internet twice.

Throughput, 64 MiB over SSH (Compression=no), single stream:
- MESH via VPS:  67,108,864 B in 18,535 ms = **28 Mbps**, 74 ms RTT
- LAN direct:    67,108,864 B in  1,904 ms = **281 Mbps**, sub-ms
=> **10x**. (SSH/TCP numbers; real UDP ceiling higher on both, ratio holds.)

Direct LAN ICMP is 100% loss — Windows Firewall blocking ping, NOT a routing
failure. TCP 22 succeeds on the LAN address. Do not read the failed ping as
an unreachable host; that misread is presumably why everything fell back to
the mesh and stayed there.

### Arithmetic on the decay

Muninn demand: 12 Mbps video + 3.07 Mbps PCM audio = ~15 Mbps.
On the mesh path that is **~54% of measured capacity**, shared with all other
mesh traffic (Vili on 10.77.0.4:8824, Idunn health, telemetry, VoidBot tunnel).
On the LAN path it is **~5%**.

54% sustained utilization across an internet hairpin with bufferbloat, carrying
a RELIABLE audio channel that retransmits under congestion, is precisely a
"mostly works, slowly falls apart, audio dies first" machine. That matches the
operator's report verbatim: "somewhat reliably held a stream, though it would
still decay, particularly with the audio which would eventually degrade to
nothing."

The hypothesis survives with its premise CORRECTED: bandwidth starvation is real
and now measured, but the cause is the VPS hairpin, not the (now removed) tether.

### Fixes, in order of cheapness

1. **Point the media at the LAN address.** 28 -> 281 Mbps, 74 ms -> sub-ms.
   Utilization 54% -> 5%. Costs one config change.
2. **Compress the audio + take it off the reliable channel.** Opus 128-256 kbps
   replaces 3.07 Mbps PCM; audio becomes lossy + the FEC the receiver already
   implements. Removes the congestion-collapse mode entirely.

(1) alone may hold the stream. (2) is what makes it hold when conditions degrade
on purpose — which is the actual soak test.

### Ops correction owed to gamecult-ops

`inventory.md` Raven section: "Historical LAN IP: 192.168.1.84" and the
2026-07-16 LAN reverification are both stale. Raven is now 192.168.178.165 on
192.168.178.0/24 beside Starfire. Not corrected here — gamecult-ops's truth to own.

## MEASURED 2026-09-10 — the video channel was the decay, and not for the reason predicted

Run on Raven through `cultnet-impair.exe`, seed 7, 848-byte packets at 12 Mbps
(Muninn's real media shape), 4000 packets per cell. Harness is
`examples/media_delivery_probe.rs` in CultLib's `claude/media-channel-delivery`.

    profile    mode          delivered    ack bytes back
    clean      unreliable      100.00%           43
    clean      reliable         69.10%      131,322
    loss 1%    unreliable       98.95%           43
    loss 1%    reliable         72.00%      135,837
    loss 3%    unreliable       96.73%           43
    loss 3%    reliable         73.30%      138,030

**A reliable media channel loses 31% of a CLEAN 12 Mbps offer.** Its delivery is
nearly flat as loss rises (69 -> 72 -> 73), so loss was never the binding
constraint: the reliable path cannot carry the rate at all. Unreliable holds
every packet clean and then degrades by about the loss injected, which is what a
media lane should do.

The reverse channel differs by three orders of magnitude: 43 bytes of
acknowledgement against ~135,000.

### Correction to the hypothesis recorded above

The "PRIMARY AUDIO DECAY HYPOTHESIS" section argues congestion collapse: a
reliable channel retransmitting under loss, adding load when the link has least
to give. The measurement does not support that mechanism. Reliable fails on a
clean link with no loss and no congestion. The conclusion — video was on the
wrong channel — survives; the reasoning did not.

The VPS hairpin (28 vs 281 Mbps) and the 1.45x byte-encoding tax are both real
and both made it worse. Neither caused it.

### Caveat on the numbers

Sender-side wire cost for the reliable rows is an undercount: the probe stops
draining after 5 s while retransmission is still in flight (the receiver ran
13.7 s). No amplification figure is quoted for that reason. Delivery percentages
are receiver-measured and sound.

### What is still unmeasured

- No cross-host run. Starfire's firewall drops inbound UDP on both the LAN and
  mesh interfaces, so the matrix ran on Raven's loopback with impairment as the
  controlled variable. Adding a firewall rule is an operator decision.
- Audio is still PCM on a reliable channel. That pairing is untouched and still
  wrong; it is the Opus change, not this one.
- `Mimir/docs/research/moonlight-reliability-acceptance-2026-07-16.md` names the
  next reliability cut as CultNet fragmentation sitting below application-level
  FEC, so one lost transport fragment discards a whole FEC shard. Measured
  below: the amplification is real and, worse, the receiver died. FEC shard
  granularity is still open; receiver death is closed.

### Correction to the June "tuned for LAN on a VPS path" claim

That claim was wrong. The July acceptance ledger records the field runs as
direct LAN, explicitly no WireGuard route. The hairpin arrived with the
relocation, after the tuning. See [[july-2026-interruption]].

## MEASURED 2026-09-10 — fragmentation was a fuse, not a tax

Same harness, Raven loopback through `cultnet-impair.exe` (seed 11,
`loss_basis_points = 300`, `burst_length = 1`, `--metrics` for ground-truth
drop counts), unreliable media channel at 12 Mbps, `max_fragment_bytes = 1200`,
equal bytes per cell. "old" is CultLib `main` at 05e0925; "new" is
`claude/rudp-fragment-eviction`, merged to main as a8aedda.

    cell                 datagrams  dropped   old receiver     new receiver
    848B   x1 fragment      12,003     356    97.03%           97.03%  evicted 0
    3392B  x3 fragments      9,003     277    DIED             91.00%  evicted 207
    6784B  x6 fragments      9,003     277    DIED             82.73%  evicted 196

`RUDP pending fragment-set limit reached`: a fragment set missing one fragment
was never reclaimed, only completed sets and oversize sets left the map, so the
64-set bound was a fuse that blew after 64 lost fragments at any loss rate. The
receiver returned `Err` from `receive` and the loop terminated. Fixed in
`d3bc6e6` (CultLib main a8aedda): evict the set untouched longest, count it as
`fragment_sets_evicted`. The counts fit the model — ~270 payloads lost a
fragment, 64 could pend, the rest were evicted.

The amplification the July ledger predicted is exactly 1 - (1 - p)^n: 3.08%
datagram loss became 9.0% payload loss at three fragments and 17.3% at six. A
media payload must fit one datagram; Muninn packetizes to 848 bytes for this
reason and Ratatoskr must keep it that way. Any consumer sending large typed
documents over a lossy link pays this; Ghostlight to Heimdall fragments sealed
envelopes at 2,048 bytes.

Also closed in the same commit: `allocate_fragment_id` used `saturating_add`,
so the u16 id parked on 65,535 and every later set shared it — the July
eight-minute failure, recorded then as fixed in an Odin-vendored snapshot and
never upstream. cultnet-ts and cultcache-py had no bound at all (unbounded
growth rather than death); bounded the same way on
`claude/ts-fragment-eviction`.

Harness scar: two earlier runs today reported fragmentation numbers from a
probe whose `max_fragment_bytes` argument had been lost with a working tree,
so it silently sent whole datagrams. The impair proxy's `received` count
equalling the payload count is what exposed it. A harness must prove it is
exercising the path it claims to measure; `--metrics` stays in the script.

## Ratatoskr exists (2026-09-10)

https://github.com/GameCult/Ratatoskr — the receiving end, extracted from the
6,515-line C++ plugin in `Mimir/native/obs_stem_source/`.

Holds one rule: **it implements no transport, it consumes CultNet.** The plugin
it replaces hand-rolled acknowledgement, fragment reassembly, sockets,
`cultmesh://` parsing, wire framing and two erasure codes in 973 lines of
headers linking no CultLib, which is why every Aug/Sep CultNet fix reached
cultnet-rs and cultnet-ts but not the receiver.

Working: subscription over `cultnet-rs`, drained and handed across a C ABI.
9 Rust tests, 14 C checks (the C side loads the cdylib dynamically and verifies
the header's claims, rather than assuming them). Pinned to CultLib `d1a9eda`.

### The next blocking piece

`MuninnMediaWireRecord` and `encode_media_wire_record` are still in
`crates/muninn-daemon/src/media_packetizer.rs`. Both ends need them, so they
belong in CultLib beside the `gamecult.media_*` records. **Until that moves,
Ratatoskr cannot decode a real Muninn stream.**

The receiver-side reassembly code that was kept in `media_packetizer.rs` as the
seed for Ratatoskr moved there on 2026-09-10 (`Ratatoskr` 2618184,
`crates/ratatoskr-core/src/video.rs`) and was cut here (705 lines: frame
assembly, the assembly set, reassembly, expired-frame feedback, their tests).
Muninn now holds only the producer half: packetize, XOR stripe parity, and the
feedback *builder* its sender tests exercise. The contract's erasure code is
XOR stripes, one recovered chunk per stripe; the C++ receiver's GF(256) block
solver reads fields the v2 record does not carry and stayed in Mimir.

### Opus, unblocked but unstarted

The receiver's audio decode is an ffmpeg child process with a hardcoded
`-f aac` (`mimir_obs_muninn_source.cpp:2640`, `FfmpegAudioDecoder`). Opus is
that becoming codec-driven, not a missing decoder — an earlier claim in this
session that no decoder existed was wrong.

Measured on Raven: Opus 160k target = 183 kbps actual against PCM's 3,072.
**16.8x.** `libopus` is present on Raven. Framing has in-house precedent: the
codec string `aac-adts-padded-v1` is a 2-byte big-endian length prefix, which is
what an `opus-padded-v1` needs, and explains why June switched to PCM — PCM is
self-framing by size.

Separately worth chasing: the plugin decodes `-f aac` while Muninn has sent PCM
over RUDP since `2a4f08d` (2026-06-23). Those do not match. The July ledger
records a clean run, so it worked then; it is an independent candidate for
current RUDP audio failure.

## State at end of the 2026-09-09/10 pass

Three repos where there was one crate, and a contract neither of them owns.

| Repo | Role | State |
|---|---|---|
| CultLib `main` `05e0925` | the contract | media records, wire envelope, validation, channel delivery |
| Muninn | producer | speaks the shared contract; blocked on one pin (below) |
| Ratatoskr | consumer | decodes the envelope; builds clean, no conflicts |

### The one thing blocking a clean Muninn build

`odin-core` pins CultLib `c13b6ba`; Muninn needs `05e0925`. Cargo resolves two
`cultcache-rs` copies and `DatabaseEntry` from one is not `DatabaseEntry` from
the other.

Muninn cannot resolve this alone: it uses 23 odin-core symbols, and while 16 are
its own records that belong here (capture stream, command boundary, HID
controller state, five Move records, OBS catalog, Quest access, telemetry
surface, transport profile), 7 are genuinely Odin's — `discover_provider_endpoints`,
`OdinDocuments`, `OdinEndpointQuery`, Eve advertisement and surface records,
`IdunnDaemonHealthRecord`.

**Resolution: Odin bumps its CultLib pin to main.** Owned by the Swarm Migration
work, deliberately deferred — bumping it changes what the running Odin daemon
links, and Odin is the target the estate observes through.

Until then `Cargo.toml` carries an uncommitted `[patch]` unifying both consumers
on the local CultLib checkout. `Cargo.lock` is also left untracked on purpose:
committing it now would bake those local paths into the lockfile. Commit a clean
one once the patch is gone.

### Next unblocked work, in rough order

1. **Move the 16 Muninn-owned records out of `odin-core` into Muninn.** Does not
   fix the diamond, but it is the last of the ownership inversion and needs no
   coordination.
2. ~~Port the OBS plugin into Ratatoskr~~ — done 2026-09-10 as a new source
   type, "CultMesh Media Stream", over a producer-agnostic contract: CultLib
   `gamecult.media_stream_advertisement` / `gamecult.media_stream_request`
   (780e1a4). Muninn advertises and takes requests (b679fc2); the
   Muninn-specific `obs_stream_catalog` / `capture_stream_command` stay
   published beside them until nothing reads them. The old plugin's CLI
   shell-out and `.cc` snapshot are not ported. **Not yet exercised live:**
   Raven runs a pre-b679fc2 Muninn, and Starfire's inbound UDP is closed.
3. **Opus.** Unblocked and measured at 16.8x. `-f aac` in the receiver becomes
   codec-driven; framing follows the `aac-adts-padded-v1` precedent (2-byte
   big-endian length prefix).
4. ~~Receiver feedback from Ratatoskr~~ — done 2026-09-10 (Ratatoskr
   `feedback.rs`). The producer's repair cache and keyframe counter now have a
   live counterpart. `build_receiver_feedback` moved to CultLib (34c8ea7) and
   stopped implying a keyframe request from missing chunks (4160ab5): a repair
   request is the alternative to one, and the sender turns every new keyframe
   request into an IDR.

### Deliberate breaks, both recorded as passing tests

- `the_previous_generation_bridge_is_deliberately_no_longer_understood` — the
  deployed C++ OBS bridge speaks `muninn.*` media schemas and is no longer
  understood. If it starts passing, someone reintroduced the old ids.
- Muninn's video channel is now `Unreliable`. Measured: reliable delivered 69%
  of a clean 12 Mbps offer.
