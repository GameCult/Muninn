# Muninn realtime media robustness: cut map (v2)

Imagination pass 2, 2026-09-30. **Draft, uncommitted.** Self places it over
`Muninn/docs/realtime-robustness-cut.md` on Muninn `main` (pass 1 is `e70db69`,
and the rulings are `eb332b0`). Name that branch in every brief.

Campaign (operator, 2026-09-30): FEC, parity, receiver-requested IDR, bitrate
adaptation, and a loss-tolerant HID edge for remote play.

## Status header

| | |
|---|---|
| Map state | Imagination pass 3. Q1-Q10 are ruled (below). Pass 2 mapped Cut 6 in full (Q6 = a); pass 3 re-maps the HID cuts H0-H5 to Q9 (Sleipnir is a crate in Muninn's workspace) and Q10 (Muninn advertises an input-stream capability; the consumer requests it and dials). Three new forks are open: Q11 (who owns the input-stream records), Q12 (Move evidence rides the HID listener), Q13 (whether Raven's Sleipnir is live, which gates the H2/H4 merge). |
| In Hands | Cuts 0-1 on `hands/rr-cut0-1` (`2c120c9`, `f2a5db9`, `a189363`). This pass does not touch those sections. |
| Ready for Hands | Cut 2 (the latency default is settled by Q7); Cut 3 (Q2-Q4, Q8 settled); H1 (after H0, and after Q11); H3 (any time; behaviour-neutral); Cut 6a (after Cut 2) |
| Blocked | Cut 4 (on Cut 3's tag); Cut 5 (on the pin gate); Cut 6b (on Cut 2 and Cut 5); Cut 6c (on Idunn B3 and `route/b3-declare` merging, and on FFmpeg provisioning on Raven); H2 (on Q12); the H2+H4 merge to `main` (on Q13 and H5 readiness, because `raven-muninn` deploys `main`'s head); H5 (on Idunn B3) |
| Heads read | Muninn `origin/main` `eb332b0` (code identical to `cee7b9c`), `route/b3-declare` `2260853`; CultLib `069ecc3`; Ratatoskr `488b5b1`; Odin `main` `379b826`; donor Odin `80adfd3`; Odin tag `attic/claude-cultlib-pin-c2a9a6e` `3e96c6c`; Idunn `771138a`; gamecult-ops working tree (binding `idunn/yggdrasil/bindings/raven-muninn.toml.in`, `scripts/idunn/idunn-deployment-targets.ps1`) |

The CultLib media plane is byte-identical between Muninn's pin `c2a9a6e` and
`main` `069ecc3`: `git diff c2a9a6e main` over `media_stream_contracts.rs`,
`media_stream_wire.rs`, `rudp.rs` and `transport.rs` is empty. Every CultLib
anchor below holds at both SHAs. `M` anchors are at `eb332b0` and equal
`cee7b9c` for code. `S` = Sleipnir at Odin `main` `379b826`
(`crates/sleipnir-daemon/src/main.rs`), `DS` = the donor's Sleipnir at `80adfd3`,
and `I` = Idunn `771138a`.

---

## Rulings (operator, 2026-09-30)

- **Q1: remote play.** A viewer's controller drives a game on another host over LAN or mesh, so quick
  taps must survive loss. The HID edge record is born in Muninn after named cut 1, and delivery uses
  CultLib's profile-owned channels. Imagination maps the HID cuts next; nothing is specified yet.
- **Q2 (a): a pinned side-commit for now.** A single-purpose Odin commit bumps `odin-core` to Cut 3's
  CultLib, and a `pins/` tag keeps it alive rather than `attic/`. Severing Muninn from `odin-core`
  continues as Muninn's own cuts. Every CultLib bump repays this debt.
- **Q3: depend on `reed-solomon-erasure` 6, without simd.** A known-answer test pins the wire bytes, so
  the library can be replaced without changing the wire.
- **Q4: delete the reliable `audio` channel.** Audio goes lossy with parity on `media`. This reverses the
  2026-09-10 decision.
- **Q5: audio parity comes after Opus.** Video parity ships in Cut 5. Audio stays on the reliable channel
  until the Opus cut, then moves.
- **Q6 (a), against the recommendation: port the native encoder.** This is the libavdevice/NVENC child,
  with an Idunn recipe step on Raven. Cut 6 gives IDR on request and bitrate adaptation, and it is the
  campaign's one new native target.
- **Q7: 250 ms default, owned by the request.** It goes in the advertisement, and both ends derive from
  the request.
- **Q8: fixed parity first.** Video uses `k ≤ 16`, `m = max(2, ⌈0.25k⌉)`; audio uses 4+2. The rate is
  carried per record. Adaptive parity is a later cut.


**Pass-2 consequences of the rulings.** Every section below is rewritten to
the ruled design. The earlier options survive only in git (`e70db69`).
- Q2 names the `pins/` mechanism. The first `pins/` commit is **not** needed
  for HID: `3e96c6c` already pins CultLib `c2a9a6e`, which is Muninn's current
  pin. Retag it `pins/odin-core-cultlib-c2a9a6e` now, keep the `attic/` tag
  until nothing references it, and cut the next `pins/` commit only at Cut 3's
  CultLib tag.
- Q6 = (a) makes Cut 6 a real cut (6a-6c below).
**HID rulings, 2026-09-30.**
- **Q9: Sleipnir becomes a crate in Muninn's workspace.** It does not get its own repo, and it does not
  stay in Odin. Odin's copy is deleted after the StreamPixels ship.
- **Q10: the consumer asks Muninn, and Muninn advertises the capability.** This is the opposite of the map's
  viewer-dials recommendation. Operator: "Ratatoskr is asking Muninn to open a video stream, Sleipnir asks
  Muninn to open an input stream. Both should be Muninn capabilities which it chooses to broadcast via
  CultMesh." So Muninn publishes an input-stream capability through CultMesh beside its media capability,
  and Sleipnir requests it the way Ratatoskr requests video. The HID cuts H0-H5 need re-mapping to this
  shape before Hands.

**Rulings, 2026-09-30 (operator), pass 3 forks:**
- **Q11 (a): the input-stream records live in `muninn-contracts` as `muninn.input_stream_*`.** The input track
  needs no CultLib change. Promote them to CultLib if a second producer appears.
- **Q12 (a): Mimir's Move proof is in use.** Move evidence becomes a channel on the input-stream session, and a
  small Mimir cut switches over. H2 waits for that Mimir cut.
- **Q13 is open.** Run H5's preflight first, to find out whether Raven's legacy Sleipnir is live.

## Target: ends and invariants

**End.** A Muninn stream that loses packets degrades by what each signal class
permits, and recovers inside one latency budget without a human. Loss is
repaired by parity first, then by selective retransmit, then by a keyframe. The
sender never holds more than it can send in time, and every figure it reports
comes from observation.

Invariants. Each one must be falsifiable by a test or a probe.

1. **One latency budget, owned by the request.** `latency_budget_ms` on
   `gamecult.media_stream_request.v1` is the only deadline source. The sender's
   queue age, repair eligibility and parity grouping, and the receiver's
   assembly age, all derive from it. Nothing else mints a deadline. Two
   constants disagree today (finding B6).
2. **No unbounded buffers on the media path.** Every queue between the encoder
   and the socket has a count bound and a time bound. Crossing the bound drops
   **whole frame groups** (chunks together with their parity), oldest first. It
   never returns an error that restarts the stream, and it never sends a partial
   frame whose parity is then useless.
3. **Deterministic parity.** Parity bytes are a pure function of the data
   shards, `(k, m)` and the scheme id. They involve no clock, no randomness and
   no per-process state. A known-answer vector pins the scheme, so a library
   upgrade that changes its matrix fails a test.
4. **MDS recovery guarantee.** Any `≤ m` erasures in a `k+m` block recover the
   data exactly. Any `> m` erasures report failure and never produce wrong
   bytes. This is proved exhaustively for every shipped `(k, m)`.
5. **One codec, both ends.** The encoder and decoder of the erasure code live in
   one place, CultLib, with the record that carries it. A producer and a
   consumer that disagree on the math cannot both compile against the same pin.
   This is the split that stranded audio recovery in Mimir's C++ (teardown-map
   L241-247).
6. **Each media payload fits one datagram, parity included.** Encoded wire size
   is `≤ MUNINN_RUDP_MEDIA_MAX_FRAGMENT_BYTES` (1431; `main.rs:82-86`) for
   data and parity shards alike. The fragment amplification `1-(1-p)^n`
   (teardown-map L417-419) must not return through parity.
7. **Health is observed, never configured** (teardown-map L210-211). Parity
   recovered, parity failed, repairs sent, IDRs requested and bitrate changes
   are counted where they happen, per leg.
8. **A keyframe request is an edge, not a level.** One decode-chain
   invalidation produces at most one IDR per cooldown. A recoverable missing
   chunk never forces an IDR. That is already true on the receiver side:
   Ratatoskr `feedback.rs:92` and CultLib `4160ab5`.
9. **The transport stays semantics-free.** CultNet RUDP gains no media-, HID-
   or producer-named channel behaviour beyond what it has. The donor's
   `"hid.edge"` / `"hid.edge.assist"` arms in a vendored `rudp.rs` (`9af2edb`,
   `80adfd3`) are the pattern this forbids.


10. **One commit primitive per virtual pad.** Snapshot axes, ordered edges,
    epoch baselines and staleness neutralisation all reach ViGEm through one
    Sleipnir function. Within an epoch, button visibility is owned **only** by
    the edge stream. The snapshot's `buttons` field is a baseline at epoch start
    and nothing else. The donor overwrote snapshot buttons with the previous
    record's (`DS main.rs:573-578`); that split is forbidden.
11. **A tap survives loss without transport retransmission.** A press and
    release captured between two transport ticks reach the pad, in order, under
    the Cut 3b profiles up to `loss-5pct` and `burst-8`. The mechanism is
    redundancy: every unacknowledged edge rides every frame until it is
    acknowledged. No reliable channel carries input, so there is no
    retransmit debt and no head-of-line blocking.
12. **Input memory is bounded on both ends.**
    - The sender keeps at most 256 unacknowledged edges **per (subscriber
      session, device) window**, and rotates that window's epoch on overflow
      (donor rule, `D main.rs:5846-5854`). One lossy subscriber never forces
      another to rebaseline.
    - A frame carries at most `MAX_EDGES_PER_FRAME` (32, or less if H1's
      worst-case encode test says 32 does not fit one datagram).
    - The receiver buffers nothing beyond the window it was sent.
    - Stale epochs are fenced by monotonic comparison, not by a growing set
      (the donor's `retired_epochs: HashSet`, `DS main.rs:210`, grows forever).
13. **Input latency is bounded, and the bound is owned by the request.** A
    queued edge is held visible for 24 ms (`DS main.rs:32-35`, one 60 Hz game
    sample plus margin) **unless** it is older than `input_budget_ms`. Then it
    is actuated without the hold. A tap is compressed, never dropped. The
    budget is Q7's shape applied to input: the advertisement carries the
    default (100 ms), the request carries the value, and both ends derive from
    the request.
14. **No JSON on a live wire.** Muninn↔Sleipnir today speaks JSON both ways
    (B14), and the donor did too (`D main.rs:6117,6141`, `DS main.rs:693-699`).
    Every HID and encoder-control message becomes a typed CultCache record in
    a CultNet message.
15. **Encoder commands are typed and fire-and-forget; their effect is
    observed.** Muninn writes `muninn.video_encoder_command.v1` records to the
    encoder's stdin as CultNet frames. Whether an IDR happened is read from the
    Annex B stream (keyframe access units). Whether a bitrate took is read from
    measured bytes per second. Neither is read from what was asked (invariant 7).
16. **One video encoder path.** When Cut 6b lands, the ffmpeg CLI video child is
    deleted. A missing encoder binary fails the request with a typed `detail`.
    There is no fallback, because the donor's
    `video_encoder_path.is_some()` switch (`D main.rs:1942`) kept two paths
    alive.

Explicitly **not** in scope:
- Mimir's `Mimir.CultMeshMedia`: a separate MPEG-TS-over-documents relay,
  `mimir.cultmesh_media_frame`, `Program.cs:11`. It is not a
  `gamecult.media_*` consumer.
- The retired C++ OBS receiver. It is "deliberately no longer understood"
  (teardown-map L672-674).
- Opus itself, which is Muninn's own named cut. It is sequenced here, not
  mapped here.

---

## Probed mechanism claims

These were run, not inferred from names.

| Claim | How it was established | Result |
|---|---|---|
| What Muninn's sender emits | Read `media_packetizer.rs` @`cee7b9c`; Yggdrasil `cargo test -p muninn-daemon --bin muninn -- media parity repair rudp` | 78 pass. Video: data chunks plus XOR stripe parity (`gamecult.media_video_parity_shard.v2`) on `media`, unreliable. Audio: `pcm-f32le-interleaved`, 3,840-byte packets, on `audio`, **reliable**. **No audio parity.** |
| Receiver recovery present | Read Ratatoskr @`488b5b1` | Video: XOR stripe recovery, one chunk per stripe (`video.rs:224-269`). Audio: **none**; it passes straight through (`receiver.rs:320-323`). Keyframe request: yes, edge-triggered with a cooldown (`feedback.rs:92,130`). `cargo test -p ratatoskr-core` on Yggdrasil: 31 + 2 pass, including `parity_gives_back_one_lost_chunk_per_stripe` and `two_losses_in_one_stripe_are_beyond_repair`. |
| Donor GF(256) tested | Yggdrasil `cargo test -p muninn-daemon --bin muninn -- fec parity gf audio_fec video_wire` @`80adfd3` | 9 pass. Audio 4+2: exhaustive over every 1- and 2-erasure pattern (`donor media_packetizer.rs:4919`). **Video 8+4 Cauchy: only one single-erasure recovery, done by hand inside the test (`:2957`). There is no Rust multi-erasure video decoder** (it lived in Mimir's C++). **There is no known-answer vector for `gf256_mul`/0x11d**: a consistent wrong polynomial passes every round-trip. |
| An established codec fits | Scratch crate `rs-probe` on Yggdrasil, `--release` | `reed-solomon-erasure` 6 (GF(2^8)): any shard length (849 OK), exhaustive `≤m` recovery true for 4+2 and 8+4, `m+1` erasures fail cleanly. It costs 13.5 µs to encode and 11 µs to decode an 8+4 block of 850 bytes, and 2.6 µs / 3.4 µs for 4+2 at 864 bytes. `reed-solomon-simd` 3: **refuses odd shard sizes** (`InvalidShardSize{849}`), decode about 230 µs per block. |
| Current XOR vs RS at equal overhead | iid model, `python` (arithmetic, not a harness) | For a 200-chunk keyframe at 3% loss, XOR16 fails the frame **63%** of the time at 8% overhead, and RS 8+4 fails 4e-4 at 50%. For frames of `n ≤ 16` chunks, XOR16 is **duplication**: each stripe holds one chunk, so the overhead is 100%. |

| Muninn↔Sleipnir HID wire today | Read `M main.rs` and `S main.rs` | **JSON both ways.** Subscription: `S :2303-2310` → `M :6216-6217`. State: `M :6298-6300` (ingress server) and `M :6835-6836` (outbound client) → `S :2485-2498`. Sleipnir coalesces "latest wins" per device (`S :2543-2590`), so a tap between two received frames never reaches ViGEm. |
| CultLib channel delivery | Read `C rudp.rs:2339-2410` | Only `media` takes its delivery from the profile, plus `audio` hardcoded Reliable. `schema`, `latest` and `_` are fixed by name. `latest` is Unreliable and **Sequenced per channel** (`:2384-2390`, `:185-191`), so a late older packet is dropped. The window-per-frame design fits this exactly, because each frame supersedes the last. **No CultLib change is needed for HID.** |
| Idunn host-native runner for the encoder | Read `I src/host.rs:934-960`, `I src/deployment.rs:55-110`, the binding | The Raven runner's `allowed_programs = ["cargo"]`. **Host-native materialisation refuses `external_inputs`** (`I host.rs:953-956`), so a sha-pinned FFmpeg download cannot be a recipe input on Raven. Runner environment comes from the binding (`[runners.rust-host.environment]`, precedent `CARGO_NET_GIT_FETCH_WITH_CLI`). `[service] required_adjacent_artifacts` exists (`I deployment.rs:113`). Recipe phases include `acceptance` (`:71-77`). |
| Donor encoder shape | `git show 80adfd3:native/muninn-video-encoder/*` | 265 lines of C++20, CMake with `FFMPEG_ROOT`. lavfi `ddagrab` → decoder → `h264_nvenc`. A `std::getline` stdin thread parses `IDR` / `BITRATE n` / `QUIT`. `fail()` calls `exit(1)`. Bitrate is reconfigured by mutating `AVCodecContext` before `send_frame`, and IDR by `pict_type = I` with `forced-idr=1`. The build script links the OBS deps snapshot at an `E:\Projects\Mimir` path that no longer exists. |
---

## Body findings

`M` = Muninn `cee7b9c`, `C` = CultLib `069ecc3`, `R` = Ratatoskr `488b5b1`,
`D` = donor Odin `80adfd3`, `O` = Odin `main` `379b826`.

**B1. Audio has no parity and rides a reliable channel.**
- `M media_packetizer.rs:1055-1063` sends audio on `MUNINN_AUDIO_RUDP_CHANNEL`.
- `C rudp.rs:2218-2228` profiles `"audio"` as `Reliable` whatever the options
  say. Only `media` is configurable (`C rudp.rs:2208-2211`).
- `C media_stream_contracts.rs:34-41` says outright that audio has no parity
  record.
- The `audio` channel exists only so that audio could be reliable
  (`C media_stream_wire.rs:41-44`). Once audio is lossy with parity, it has no
  reason to exist.

**B2. PCM packets fragment.** 480 ticks × 2 ch × 4 B = 3,840 B
(`M main.rs:2455-2470`) against a 1,431-byte fragment limit (`M main.rs:82-86`).
So every audio packet is 3 datagrams. Parity over PCM protects 3-fragment
shards (roughly 3p shard loss) and costs 1.5 Mbps at 4+2. Opus at about 183 kbps
(teardown-map L475) fits one datagram.

**B3. Video parity is XOR stripes, per frame.**
- `M media_packetizer.rs:625` sets `STRIPES = 16`.
- `:627-724` builds the stripes and `:726-754` writes data and then parity.
- The recovery guarantee is one loss per stripe. See the probe table for what
  that costs on keyframes and small frames.
- The v2 record carries only stripe geometry (`C media_stream_contracts.rs:466-517`).
  It cannot describe an RS block.

**B4. A keyframe request is counted and ignored.**
- `M main.rs:3036-3048` `record_receiver_keyframe_pressure` logs "continuing
  current low-latency encoder session until explicit encoder control exists".
- teardown-map L666-668 claims "the sender turns every new keyframe request into
  an IDR". **That is false.**
- Recovery today is the fixed all-IDR GOP: `framerate/4`
  (`M main.rs:10309-10311`) with `-g`, `-keyint_min` and `-forced-idr 1`
  (`:10405-10410`). A lost reference therefore freezes for up to 250 ms. Ratatoskr
  shows nothing until the next keyframe (`R video.rs` test
  `nothing_is_delivered_between_a_loss_and_the_next_keyframe`, `:720`).

**B5. No bitrate adaptation.**
- Bitrate is fixed per command (`M main.rs:1481-1487`, `:10399-10404`).
- What adapts is the **repair** rate (`MuninnRudpRepairBudget`,
  `M main.rs:2744-2827`), not the encode rate.
- The ffmpeg CLI child (`:10343-10421`) has no live control surface. A change
  of bitrate is a restart (`:1475`).

**B6. The latency budget has two owners.**
- Producer: an empty request budget becomes
  `MUNINN_RUDP_MEDIA_RECEIVER_ASSEMBLY_DEADLINE_MS = 2_000`
  (`M main.rs:95,1489-1495`). That default drives the sender queue deadline,
  audio expiry and the advertisement default (`:10286-10291`, `:1159`).
- Consumer: Ratatoskr sends 0, so it gets the 2 s default
  (`R catalog.rs:174,198`), but it ages frames at its own **250 ms**
  (`R video.rs:99-101`).
- So the sender will queue video for up to 2 s that the receiver has already
  given up on.

**B7. The sender's buffers are unbounded in count.**
- `M main.rs:2427` is `mpsc::channel` (unbounded), and
  `PendingMuninnMediaSendQueues` (`:2701-2731`) is two unbounded `VecDeque`s.
- Payloads expire **individually** by age (`:2505-2532`). Half a frame can be
  sent after its other half expired, which spends bandwidth on an undecodable
  frame and its useless parity.
- The Annex B reader errors the whole stream when `pending > max_pending_bytes`
  (`M media_packetizer.rs:296-306`) instead of dropping.
- Backpressure spins with `thread::sleep(1 ms)` (`M main.rs:2981`).

**B8. Repairs renew their own send deadline.**
`poll_rudp_media_receiver_feedback` passes `Instant::now()` as `queued_at` for
every repair (`M main.rs:3125-3132`). A repair's age budget therefore starts
when it is asked for, not at the frame's deadline. This breaks invariant 1
(donor doctrine: "a repair cache must not mint a new deadline",
`D docs/realtime-delivery-authority.md`).

**B9. Receiver residue is still in the producer.**
- `AudioPacketBuffer` (`M media_packetizer.rs:847-987`) is a receiver jitter
  buffer.
- The whole-stream helpers `packetize_video_annex_b_stream` (`:756-805`),
  `encode_video_annex_b_stream_wire_records` (`:1017-1029`),
  `video_annex_b_stream_send_payloads` (`:1031-1036`) and
  `encode_record_payload`/`decode_record_payload` (`:1080-1090`) are called
  only from tests. The tests are `media_packetizer.rs` `#[test]`s at
  1533/1570/1602/1820-1930/1992/2033/2078/2467, and `main.rs:12353,12395`.
- teardown-map L463-465 ("Muninn now holds only the producer half") is false.

**B10. The mux loop has no seam.** `run_rudp_mux_once` (`M main.rs:2292-~2680`)
fuses process spawning, hub opening, catalog publishing, queueing, feedback,
repair and progress logging. No test can drive the send path without
PowerShell, ffmpeg and WASAPI. That is why "not exercised live" only ever meant
exercised on Raven.

**B11. The CultLib pin is welded to an attic tag.**
- Muninn builds `odin-core` from Odin `3e96c6c` (`M Cargo.toml:25`). That
  commit is on **no Odin branch**. Only the tag
  `refs/tags/attic/claude-cultlib-pin-c2a9a6e` holds it
  (`git ls-remote origin`).
- Odin `main` itself pins CultLib `a8aedda` (`O crates/odin-core/Cargo.toml:14-17`),
  which is **older** than Muninn's `c2a9a6e`.
- odin-core's types are `DatabaseEntry`. Any CultLib bump in Muninn therefore
  reopens the two-`cultcache-rs` diamond (teardown-map L616-619) unless odin-core
  moves too.
- Muninn uses 21 odin-core symbols (`M main.rs:27-37`). Four are Odin's:
  `EveProviderAdvertisementRecord`, `EveSurfaceStateRecord`,
  `IdunnDaemonHealthRecord` and `OdinDocuments`. teardown-map L621-626 says 23
  and 7; `discover_provider_endpoints` and `OdinEndpointQuery` are no longer
  imported.

**B12. The donor's shape.** 29 commits, `2796884..80adfd3`, 2026-07-12..17. It
predates Muninn's extraction (teardown-map L628-629), the CultLib media
contract (`a250ec6` onward), and the consumer-dials-producer inversion.
- Its records are `muninn.*` in odin-core: video parity **v4**, audio parity
  v1 (`D odin-core documents.rs`).
- Its transport is a single dialled client (`CultNetRudpSocketTransportConnection`,
  `D main.rs:684`), not the hub.
- Its encoder control is a native C++ child,
  `native/muninn-video-encoder/main.cpp` (265 lines, libavdevice with
  `h264_nvenc`, stdin `IDR` / `BITRATE <kbps>`).
- Its doc and code disagree on bitrate recovery: the doc says 5% every 2 s,
  the code does `max/50` every 10 s (`D main.rs:2502-2507`).
- Its bitrate ceiling is **half** the configured rate, because "Block FEC can
  roughly double the encoder payload" (`D main.rs:2454-2458`).
- Its "dispersed video parity" (`e58e0a7`) is not cross-frame: it was
  bookending, then superseded by a within-frame round-robin over blocks
  (`4e2abee`; `D media_packetizer.rs:1295-1360`).

**B13. The donor's HID edge rewrites the transport.**
- `9af2edb` and `80adfd3` hardcode `"hid.edge"`, `"hid.edge.ack"`,
  `"hid.subscribe"` and `"hid.edge.assist"` into a vendored `rudp.rs`, plus an
  ACK-state change.
- The consumer, `sleipnir-daemon`, is in **Odin** (`O crates/sleipnir-daemon`,
  3,658 lines). The record is `MuninnHidControllerStateRecord` in odin-core
  (latest-state snapshot, `buttons: Vec<String>`; `3e96c6c documents.rs:1524-1547`).
- Today Muninn sends snapshots only (`M main.rs:6172-6200, 6758-6800`), so a tap
  shorter than one sample, or lost with its packet, never happens.

**B14. The HID wire is JSON, and a tap dies at two places.**
- The wire: `M main.rs:6137-6142, 6216-6217, 6298-6300, 6835-6836`;
  `S main.rs:2303-2310, 2497-2498`. The 2026-09 repo census flagged the same
  lines (`gamecult-ops/docs/repo-census-2026-09/repos/Odin.md:48`).
- **Muninn reads a source once per transport tick**
  (`read_hid_controller_state_for_stream`, `M :6286`, `:6370`). A press and
  release inside one tick never exist as data.
- **Sleipnir keeps only the latest record per device** (`S :2543-2590`), so a
  tap that survived capture dies again at the sink.

**B15. Muninn has two HID transport paths, and so does Sleipnir.**
`M` here is Muninn `main` `c3520d1`; HID lines moved +7 to +9 since `eb332b0`.
- Muninn listens: `--hid-controller-rudp-bind`, `M :6151-6340`. This is the one
  Raven runs (recipe `raven-muninn.toml:62-65`, advertised `10.77.0.4:17887`),
  and Starfire's legacy script can run it too
  (`restart-starfire-muninn.ps1:14-15, 74-79`).
- Muninn dials: `--hid-controller-rudp-target`, `M :6724-6863`, parsed at
  `:10899-10904`.
- Sleipnir mirrors both: it dials a discovered endpoint, or listens on
  `--rudp-bind` (`S :2191-2281`, server branch `:2244-2281`).
- **Q10 settles it:** the consumer dials the producer's advertised endpoint,
  exactly as Ratatoskr dials Muninn for video. The surviving direction is
  "Muninn listens, Sleipnir dials". Muninn's outbound client and Sleipnir's
  listening mode are the losing paths; H2 and H4 delete them.

**B16. Sleipnir is welded into Odin's pin.**
- `S` is a member of Odin's workspace. It takes `odin-core` by path
  (`Odin crates/sleipnir-daemon/Cargo.toml`) and CultLib at `a8aedda`, which is
  Odin `main`'s pin, the one StreamPixels ships on.
- It imports 10 odin-core symbols (`S :10-14`). One is Muninn's HID record,
  and one is its own `SleipnirInputMappingRecord`.
- Any Sleipnir change that consumes a Muninn-owned record at Muninn's CultLib
  (`c2a9a6e`) reopens the two-`cultcache-rs` diamond **inside Odin's
  workspace**, and that can only be closed by moving Odin `main`'s pin.
- Its deployment target is the legacy script-driven `raven-sleipnir`
  (`gamecult-ops/scripts/idunn/idunn-deployment-targets.ps1:126-138`,
  `Repo = "Odin"`, scheduled task plus SSH). It has no Idunn recipe. **Whether
  it runs on Raven today is unverified.**

**B17. The sink already has the seam a harness needs.**
`trait VirtualPadBackend { fn update(&mut self, state: &VirtualPadState) }`
(`S :179-181`) has a `LoggingBackend` (`:183`) beside the ViGEm one
(`:200-240`, `cfg(windows)`). A recording backend gives a pad timeline on Linux.

**B18. Raven cannot fetch the encoder's FFmpeg through Idunn.**
Host-native materialisation refuses `external_inputs` (`I host.rs:953-956`), and
the runner may execute only `cargo`. So the FFmpeg development root must be:
- pre-provisioned on Raven (operator action, recorded in gamecult-ops with its
  sha256);
- named by the binding's runner environment;
- compiled against by `build.rs`, through the `cc` crate for the C++ and link
  directives for FFmpeg. No CMake step and no new allowed program.

`ffmpeg_path` today points at a WinGet static `ffmpeg.exe` (binding
`raven-muninn.toml.in`, `ffmpeg_path`), which has no headers or import
libraries.

**B19. The recipe is mid-edit by another campaign.** `route/b3-declare`
(`2260853`, unmerged) adds `[[dependencies]] odin.verse-rendezvous` to
`deployment/idunn/raven-muninn.toml`. Cut 6c edits the same file and must build
on it after it merges.

**B20. Move evidence rides the HID listener, and Mimir consumes it.**
- `serve` gets the Move-evidence sender back from
  `start_hid_controller_rudp_ingress` (`M :429-437`), and the ingress thread
  sends it on channel `move-evidence` over the HID connection id
  (`M :6271-6283`).
- It sends only while an HID subscription is selected (`M :6258-6269`), so a
  consumer that wants Move evidence must speak the JSON `hid.subscribe`.
- Mimir does exactly that: `MimirOdinMoveProofEvidenceRingProvider.cs:223`
  (Mimir `d88f2e6`) sends `{"streamId": ...}` on `hid.subscribe`. It discovers
  the endpoint from the JSON `inputStreams` in Muninn's Eve provider
  advertisement (`M :3713-3722`), channel `move-evidence`.
- The local ring is switched on by the absence of the HID bind
  (`local_ring_enabled: options.hid_controller_rudp_bind.is_none()`,
  `M :4238`).
- So deleting the JSON subscription (H2) silently stops Move evidence reaching
  Mimir. The pass-2 map did not know this second consumer existed. Q12.

**B21. HID discovery is JSON scraped out of Eve advertisements.**
- Muninn writes its HID endpoint and devices as JSON into three places in
  `publish_runtime_boundary_records`: the transport profile's `input_streams`
  (`M :3674-3699`, key at `:3856`), the provider advertisement's
  `inputStreams` (`M :3701-3727`, key at `:3885`) and its `endpoints`
  (`M :3729-3750`), in two casings.
- Sleipnir recovers it by pulling `muninn.hid_controller_state.v1` records to
  learn host ids (`S :978-1003`), then pulling those hosts' provider
  advertisements (`S :950-975`), then parsing JSON in either casing and in
  tuple shape (`S :1845-2131`, `:2370-2478`), with Odin's generic
  `discover_provider_endpoints` as a fallback (`S :2380-2398`).
- The video capability has none of this: one typed advertisement, one typed
  request (`R catalog.rs:64-145`, `M :971-1166`). That typed path is what Q10
  orders input to mirror.
- The Odin-facing `publish_hid_controller_state_to_odin` (`M :6865-6875`, a
  store `put` keyed by stream id) has **one reader**: Sleipnir's discovery.
  Nothing else in Odin, Mimir, Ratatoskr, StreamPixels, Eve, VoidBot, Heimdall
  or Idunn names the schema (`git grep`, 2026-09-30).

**B22. Two readers own the same HID devices.**
- The serve tick reads every source to build receipts
  (`publish_move_controller_states`, `M :4976-5134`).
- The ingress thread reads the same sources again at its own tick
  (`read_hid_controller_state_for_stream`, `M :6377-6528`).
- The collision is fenced, not removed: on Windows the tick skips PS Move
  sources whenever the HID bind is set (`M :5081-5086`), because the report
  reader thread would otherwise have two consumers.
- Neither reader samples fast enough for a tap. The ingress loop sleeps 8 ms
  (`M :6338`), and Sleipnir's default interval is 8 ms (`S :2791`). The
  pass-2 map's "2 ms at the current poll (`HID_CONTROLLER_RUDP_POLL_INTERVAL`)"
  is the **donor's** constant (`D main.rs:5879`). It does not exist at `c3520d1`.

**B23. A producer restart orphans every running request.**
- `sync_media_stream_requests` admits only requests in state `pending`
  (`M :1004`). The "already answered" memory is process-local
  (`answered_requests`, `M :463`).
- So after a Muninn restart, every request it had answered `running` stays
  `running` on Odin and is never admitted again. The consumer must notice and
  publish a fresh request.
- Whether Ratatoskr re-requests is **unverified** (the redial lives in
  `R receiver.rs`, and the request lives in the OBS plugin's C).
- H2 does not copy this. An input request is admitted as **demand**, by
  `action`, and its `state` is only the producer's answer. The same change for
  media belongs to Muninn's media owner and is recorded, not made, here.

---

## Donor ledger: port, rewrite, drop

| Donor piece | Verdict | Why |
|---|---|---|
| `gf256_mul`/`gf256_inv`, `recover_audio_fec_data`, `video_fec_coefficient` (`D media_packetizer.rs:1177-1180, 1525-1612`) | **Drop; replace with `reed-solomon-erasure`** (Q3) | No video decoder, no KAT, and it solves a matrix per byte for audio. The probed crate is MDS-exhaustive at the shipped sizes and fast enough by 3 orders of magnitude. Owned math is a liability we need not carry. |
| 4+2 audio block and 8+4 video block sizing | **Port as defaults**, parameterised as `(k, m)` in the record | Field-chosen numbers, but carried by the record so a decoder needs no config. |
| Within-frame round-robin send order over blocks (`D :1295-1360`) | **Port** (rewrite against CultLib types) | Turns a burst into one loss per block. It is policy, not math. |
| `pad_audio_codec_frame` 2-byte BE length prefix (`D media_packetizer.rs:674-686`) | **Port** into the Opus cut's framing (`opus-padded-v1`) | Constant shard size for variable codec frames. Matches the `aac-adts-padded-v1` precedent (teardown-map L477-479). |
| Datagram-fit tests (`D :3065`, `production_aac_audio_fec_shards_fit_one_rudp_datagram`) | **Port as behavioural tests** at the CultLib codec and in Muninn | They pin invariant 6. |
| Exhaustive erasure test (`D :4919`) | **Port**, generalised to every shipped `(k, m)` | Pins invariant 4. |
| `MuninnKeyframeRequestGate` (`D main.rs:2864-2884`, cooldown 500 ms at `:102`) | **Port** (Cut 6) | Small and correct: an edge with a cooldown. |
| `MuninnVideoBitrateController` (`D :2439-2512`) | **Rewrite** (Cut 6b) | Its ceiling halving assumes 50% FEC overhead, and its doc and code disagree. The rewrite keys the ceiling on the ruled parity rate and aggregates across hub receivers, which the donor predates. |
| `native/muninn-video-encoder/main.cpp` (265 lines) | **Port** the capture and encode loop (Cut 6a). **Rewrite** its shell. | Kept: the lavfi → decode → encode loop, NVENC options, `pict_type = I` and `AVCodecContext` bitrate mutation. Deleted: `main`, `parse_options`, the `std::getline` text command thread, `fail()`/`exit`, and `fwrite` to stdout. They become a C ABI (`muninn_encoder_run` with poll and write callbacks) under a Rust `main` that decodes typed command records (invariant 15). |
| `CMakeLists.txt`, `scripts/build-muninn-video-encoder.ps1` | **Drop** | `build.rs` with the `cc` crate compiles the C++. The Idunn runner may run only `cargo` (B18). The script's FFmpeg path no longer exists. |
| `capture_button_edges`, `ActiveHidControllerRudpSource` edge fields, overflow epoch rotation (`D main.rs:5732-5856`) | **Rewrite** as two pieces in `muninn-contracts::hid` (H1): `EdgeDetector` (diffs samples; runs on the capture thread, H2) and `EdgeWindow` (per subscriber session and device; owns epoch, pending bound and acks; runs on the input transport tick, H2). | The rotation and bound rules are right. The donor ran detection at transport-tick time, which loses sub-tick taps (B14); it kept one window per source, which cannot serve two subscribers; it cloned `epoch` needlessly; and it sent JSON. |
| Donor's `hid.edge` (reliable) + `hid.edge.assist` (unreliable resend) channels and the `hid.subscribe`/`hid.edge.ack` JSON | **Drop** | Two mechanisms for one job, plus four transport name arms (invariant 9). Replaced by one frame on the profile-declared `latest` channel that carries state and the unacknowledged window (invariant 11). |
| `HidSemanticCursor` (`DS main.rs:204-256`) | **Port** into `muninn-contracts::hid::EdgeCursor` (H1), with the `retired_epochs` set **replaced** by a monotonic epoch fence | The ordering, dedupe and gap bound (256) are right. The unbounded set is not (invariant 12). |
| `INPUT_EDGE_MINIMUM_VISIBILITY` 24 ms and the actuation queue (`DS main.rs:32-35, 655-700`) | **Port** into Sleipnir (H4), with the budget override added | Invariant 13. The donor queue has no latency ceiling. |
| Snapshot-buttons overwrite (`DS main.rs:573-578`) | **Drop** | Split button ownership (invariant 10). |
| Donor capture-thread split (`fb4f331`), 1 ms XInput sampling (`D main.rs:5881, 6880-6940`) | **Port** (H2) | This is where a tap first exists as data. |
| `crates/sleipnir-daemon/examples/xinput_edge_observer.rs` (`80adfd3`, 127 lines) | **Port** as Sleipnir's live probe (H5) | It reads the virtual pad through XInput, as the game does. That is the layer where a lost tap is visible (SKILL: instrument the layer the user sees). |
| Bounded `sync_channel` plus per-frame groups, capacities 256/512 (`D main.rs:85-87, 2023-2028, 2356-2369`) | **Port the shape**, rewrite against the hub loop (Cut 2) | Exactly invariant 2. |
| `cultnet-impair` (696 lines, seeded loss/burst/jitter/reorder/dup/stall, CSV) | **Port to CultLib** `packages/cultnet-rs/examples/` (Cut 3b) | It is a CultNet transport tool. `media_delivery_probe.rs` already sits there and expects it (its header, `C examples/media_delivery_probe.rs:5`). |
| Vendored `rudp.rs` ACK changes (`9af2edb` `direct_ack_state`, ACK reliable duplicates) | **Drop** | CultLib evolved separately (`75c1807` acknowledges retransmits across horizons; `47b9fc4` changes reader semantics). Anything still missing is a CultLib finding with its own proof, not a port. |
| `muninn.*` v4/v1 parity schemas | **Drop** | Superseded by `gamecult.*` in CultLib. No live reader. |

Recommend Self tags the donor `parked/muninn-bounded-realtime-send` at `80adfd3`
before anything cites it again. This is the parked-not-dead pattern: the branch
is untracked per the morning file item 11.

---

## Where FEC lives: owner decision

**Owner: CultLib `cultnet-rs`, a new `media_fec.rs` module beside
`media_stream_wire.rs`.** It holds pure functions and no I/O.

- **Not Muninn.** The receiver needs the decoder, so a codec in Muninn is either
  duplicated in Ratatoskr or missing there. The first reproduces the Mimir split
  (invariant 5). The second reproduces today's audio situation.
- **Not CultNet RUDP (transport FEC).**
  - Transport FEC would group datagrams by send time, not by frame and
    deadline. A block spanning two frames delays the first frame's recovery
    past its deadline.
  - It would also sit beside the frame-level NACK repair that Muninn and
    Ratatoskr already run through `gamecult.media_receiver_feedback`, giving
    two recovery authorities for one loss.
  - The media plane's rule "a media payload fits one datagram" (invariant 6)
    removes the one advantage transport FEC would have, which is protecting
    fragments.
  - The transport stays semantics-free (invariant 9).
- **Not CultMesh.** CultMesh carries state and discovery documents through Odin,
  the advertisement and the request. Media records are CultNet wire records on a
  live session and are never stored. FEC has no document to live in.
- **Not a new crate.** Authority separation does not buy a package. The media
  plane is two files in `cultnet-rs` today, and every consumer (Muninn,
  Ratatoskr, StreamPixels' vendored copy) already links it.

What the module owns:
- the scheme id;
- `protect(frame_records, policy) -> send-ordered wire records` for video;
- `protect_audio(block) -> parity records`;
- `recover(block shards) -> Result<data | beyond-repair>`.

What it does **not** own:
- the `(k, m)` choice, which is policy and belongs to the producer (Q8);
- when to give up, which is the receiver's deadline (invariant 1);
- sockets.

---

## Identity, lifecycle and authority (0b)

| Kind | What names it | What happens to it over time | Who decides |
|---|---|---|---|
| Video parity shard `gamecult.media_video_parity_shard.v3` | `(stream_id, session_id, frame_id, block_index, parity_index)`; the key extends today's (`C media_stream_wire.rs:92-110`) | Emitted with its frame's group, and dropped with the group at the deadline. Never repaired, never cached for repair. v2 is **deleted**, not kept beside it. | Producer emits. CultLib codec defines the bytes. Receiver consumes until the frame deadline. |
| Audio parity shard `gamecult.media_audio_parity_shard.v1` | `(stream_id, session_id, base_packet_id, parity_index)` | Emitted after its k-th data packet. It expires with the block's latest deadline. Lands only after Opus (Q5). | Same. |
| FEC scheme id (`rs-gf256-v1`) | A string on each parity record | Changes only with a new schema version. A KAT pins it. | CultLib. |
| `(k, m)` policy | Carried per parity record (`block_data_count`, `parity_count`) | Fixed (Q8): video `k ≤ 16`, `m = max(2, ⌈k/4⌉)`; audio 4+2. | Producer (Muninn). |
| Latency budget | `gamecult.media_stream_request.v1.latency_budget_ms` (key 11) | Fixed for a request. Default 250 ms in the advertisement (Q7). A new request restarts the child (teardown-map L533). | **Consumer** asks; producer honours it. |
| Keyframe request | `requested_keyframe` on feedback | Counted per receiver. It is an edge: at most one IDR per 500 ms cooldown per stream. | Receiver asserts. Muninn's gate decides (6b). |
| Encoder command `muninn.video_encoder_command.v1` | `(encoder process, command_sequence)`. The sequence is monotonic from 1 per spawned encoder process. | Lives only on the stdin pipe. Never stored. It dies with the process; a respawn restarts the sequence at 1. | Muninn's `EncoderControl` writes. The encoder applies at the next frame. **Effect is observed from the bitstream**, not acknowledged. |
| Live encode bitrate | Observed bytes per second over a 1 s window | Moves within `[floor, ceiling]` from the request's `video_bitrate_kbps` and the parity rate | Muninn's bitrate controller (6b) |
| Input-stream advertisement `muninn.input_stream_advertisement.v1` (Q11) | `stream_id` (`muninn.<host>.input`) | Republished on the same cadence and at the same site as the media advertisement. State `available` / `streaming` / `unavailable`. | Muninn (producer). |
| Input-stream request `muninn.input_stream_request.v1` (Q11) | `input_stream_request_key(stream_id, receiver_id)` | The consumer publishes `start` or `stop`. The producer admits it **as demand**, by `action`, not by `state` (B23), and writes its answer (`running` / `stopped` / `failed` with `detail`) onto the same key. A new request supersedes the old one under the key. | **Consumer** asks (Sleipnir); producer answers. |
| Input session | The hub session whose connect payload is the request's `receiver_id` (the Ratatoskr rule, `R receiver.rs:40-42`) | Lives while the consumer is attached. A session with no admitted request is served nothing. | Consumer dials; Muninn's input hub admits. |
| HID epoch | `(session, device_id, epoch: u64)`. The epoch is the window's monotonic creation time in ns, and `+1` on overflow rotation. | New per window (session attach, device change, process start) and per overflow. The receiver fences any epoch below its current one. | Muninn's `EdgeWindow`. Sleipnir only fences. |
| HID state sequence | `(session, device_id, epoch, state_sequence)` | Monotonic per epoch. Each frame supersedes the last (the `latest` channel is sequenced per session). | Muninn. |
| HID edge | `(session, device_id, epoch, edge_sequence)`, contiguous from 1 per epoch | Detected once, at sample time, on the capture thread. Pending per window until acknowledged. Resent in every frame while pending, at most `MAX_EDGES_PER_FRAME` per frame and 256 held. Overflow rotates that window's epoch and drops its history. | Muninn detects (capture) and delivers (window). Sleipnir applies in order and acknowledges cumulatively. |
| HID edge ack `muninn.hid_edge_ack.v1` | `(device_id, epoch, applied_through)` on the consumer's session | Cumulative and idempotent. Resent each Sleipnir tick while nonzero. | Sleipnir. |

---

## Cut 0: correct the Muninn maps against git (docs only)

**Status: in Hands on `hands/rr-cut0-1` (`2c120c9`, `f2a5db9`, `a189363`). Unchanged by pass 2.**

Repo Muninn, `main`. Hands, doc-only. Nothing is deleted from history. Superseded
claims are marked as history in place and not left as two live designs.

Corrections to `docs/teardown-map.md` @`cee7b9c`:

| Line | Says | Git says |
|---|---|---|
| 1-3 | "(pre-rebuild)", dated 09-09, body at `Odin\crates\muninn-daemon` | Live repo `GameCult/Muninn`. `main.rs` is 15,134 lines and `media_packetizer.rs` 2,633. |
| 24 | CultLib pin `c13b6ba0` | `c2a9a6e` (`Cargo.toml:21-23`) |
| 50, 65 | "19 muninn.* schemas… in odin-core", "20 symbols" | The four media schemas moved to CultLib (`gamecult.*`). 21 odin-core symbols (`main.rs:27-37`). |
| 450 | Ratatoskr "Pinned to CultLib `d1a9eda`" | `c2a9a6e` (Ratatoskr `Cargo.toml:16-18`) |
| 463-465 | "Muninn now holds only the producer half" | `AudioPacketBuffer` and the whole-stream helpers remain (B9). Cut 1 makes the sentence true. |
| 611 | CultLib `main` `05e0925` | Contract unchanged since `c2a9a6e`. `main` is `069ecc3`. |
| 615-642 | "The one thing blocking a clean Muninn build… uncommitted `[patch]`… `Cargo.lock` untracked" | Resolved (L684-686). `Cargo.lock` is tracked (`60ed6b2`). Mark as history. **Add:** odin-core comes from `3e96c6c`, which is reachable only through the attic tag (B11). |
| 621-626 | 23 symbols, 7 Odin's | 21 symbols, 4 Odin's (B11) |
| 656-659 | "Not yet exercised live: Raven runs a pre-b679fc2 Muninn" | Contradicted by L536-603. Both legs were proven 2026-09-10/11 on `bc43e1b`+. |
| 666-668 | "the sender turns every new keyframe request into an IDR" | False. `main.rs:3036-3048` logs only (B4). |

Corrections to `docs/muninn-media-streaming.md` @`cee7b9c`:

| Line | Says | Code says |
|---|---|---|
| 240 | preset `p1` | `p5` (`main.rs:10279`) |
| 249 | chunks `480` bytes | `848` (`main.rs:81`) |
| 258-259 | resend `5` ms inside a `75` ms expiry | `30` ms (`main.rs:94`). Expiry is the latency budget, default 2,000 ms (`main.rs:95`). |

Verification: `git diff --stat` shows only the two docs. Soul checks each row
against `git show cee7b9c:<file>`. Ledger: ±0 code.

---

## Cut 1: subtract the receiver residue from the producer

**Status: in Hands on `hands/rr-cut0-1` (`2c120c9`, `f2a5db9`, `a189363`). Unchanged by pass 2.**

Repo Muninn, `main`. This is a subtraction cut, kept separate from behaviour
cuts. No pin change.

**Deletes first** (`media_packetizer.rs` @`cee7b9c`):
- `AudioPacketBuffer` plus its impl, `:847-987`, and its 5 tests
  (`audio_packet_buffer_*`, around `:1820-1935`);
- `VideoAnnexBStreamPacketizeOptions` `:162-173`,
  `VideoAnnexBStreamWireOptions` `:175-180`, `packetize_video_annex_b_stream`
  `:756-805`, `encode_video_annex_b_stream_wire_records` `:1017-1029` and
  `video_annex_b_stream_send_payloads` `:1031-1036`;
- `encode_record_payload` / `decode_record_payload` `:1080-1090`;
- empty-line runs left by earlier cuts (`:230-240`, `:403-406`, `:552-556`,
  `:1091-1099`).

**Keeps and moves.** These tests prove real packetizer behaviour and are
**re-homed onto `VideoAnnexBStreamSendState`**, not deleted:
- `packetizes_annex_b_stream_into_timed_video_records` (`:1533`);
- `packetizes_non_reference_h264_p_frames_without_dependency` (`:1570`);
- `rejects_negative_annex_b_stream_deadline_delay` (`:1602`);
- `media_wire_batches_annex_b_video_records` / `media_wire_encodes_annex_b_stream_for_sender` / `media_send_payloads_pin_video_to_media_channel` (`:1992, 2033, 2078`);
- `main.rs:12340-12420`, the two fragment-limit tests.

`feedback_payload_decodes_legacy_record_without_chunk_keys` (`:2467`) proves a
CultLib `default` key. Delete it here if `C tests/media_stream_contracts.rs`
already pins key 10's default. Otherwise move it there. Hands checks which
applies and reports.

**Adds:** none.

**Verification:**
- `cargo build --release -p muninn-daemon` and
  `cargo test -p muninn-daemon --bin muninn` on Yggdrasil via ygg-verify.
- A negative grep: `rg "AudioPacketBuffer|packetize_video_annex_b_stream\b|decode_record_payload" crates/`
  returns nothing.
- `cargo mutants --in-diff` scoped to `media_packetizer.rs`.

**Ledger:** about −330 lines, no new surface.

---

## Cut 2: bounded sender, one deadline owner, and a drivable seam

Repo Muninn, `main`. No pin change. Cut 5's parity rides on it.

Q7 ruled: the default is 250 ms, owned by the request. It goes in the advertisement, and Ratatoskr derives its `max_frame_age` from its request (Cut 4).

**Authority map.**
- **Owner:** a new `MediaSendCore` in Muninn. It holds the queues, the hub, the
  feedback intake, the repair cache and budget, and the progress stats.
- **Inputs:** `Read` sources for encoded video and audio, a `MediaSendPolicy`
  derived **only** from the request (`latency_budget_ms`, `video_bitrate_kbps`,
  `media_packet_bytes`), a `CultNetRudpServerHub`, and a clock.
- **Outputs:** payloads on the hub, repairs to the asking receiver, stats.
- **Derived state:**
  - the queue deadline, repair eligibility and audio expiry derive from
    `policy.latency_budget`;
  - `MUNINN_RUDP_MEDIA_RECEIVER_ASSEMBLY_DEADLINE_MS` is **no longer an owner**.
    It dies, and the advertisement's `default_latency_budget_ms` takes the
    250 ms constant (Q7).
- **Forbidden writers:**
  - `Instant::now()` as a repair's `queued_at` (`main.rs:3129`). A repair
    inherits its frame's deadline.
  - Per-payload expiry. Expiry is per group.
  - `VideoAnnexBStreamSendState` erroring on overflow
    (`media_packetizer.rs:296-306`). Overflow drops the oldest complete access
    unit and counts it.
- **Shared paths:** live sends, repairs and the harness all go through
  `MediaSendCore`. `run_rudp_mux_once` keeps process spawning and publishing,
  and nothing else.
- **Deletion line:** `mpsc::channel` at `main.rs:2427`, the unbounded
  `PendingMuninnMediaSendQueues` (`:2701-2731`), per-payload expiry (`:2505-2532`),
  and the 1 ms sleep spin (`:2981`), which becomes a bounded poll of resends
  that stops at the group deadline.

**Adds (types and rules only):**
- `QueuedMediaGroup { kind, deadline: Instant, payloads }`. One access unit with
  its parity, or one audio packet.
- `mpsc::sync_channel` with a small bound, as in the donor (`capacity 1` of
  groups).
- Per-kind group caps: audio 256, video 512 in the donor. Cap them in **groups**,
  and name the byte ceiling they imply.
- Drop-oldest-group on cap, counted as `groups_dropped_{audio,video}`.
- Audio before video, as today (`:2705-2716`).

**Harness seam (the reason B10 matters).** A `#[cfg(test)]` driver feeds
`MediaSendCore` from a committed Annex B fixture. It needs:
- a short H.264 clip, generated once and committed with its ffmpeg command
  recorded beside it (provenance);
- a PCM ramp.

It serves over a loopback hub, through a seeded **drop relay thread** (a UDP
relay between two sockets that drops by a seeded schedule), to an in-test
`CultNetRudpSocketTransportConnection` client. The client counts received and
expired groups. It needs no ffmpeg, PowerShell or GPU, so it runs on Yggdrasil.

**Verification (rules each test pins):**
- Push faster than the hub drains: resident queue memory stays `≤` the stated
  ceiling, and the stream is not restarted. This kills a mutant that removes
  the cap or turns it into `usize::MAX`.
- Expire mid-frame: either every payload of a frame group is sent or none is.
  Assert on the relay's captured datagrams, not on stats. The observation sits
  at the wire (invariant 2).
- A repair asked for after its frame's deadline is not sent. A repair asked
  for before it is sent. Probe at `deadline − ε` and `deadline + ε`, with ε
  under 5 ms, so that a mutant that re-derives the deadline from `now` is
  caught. **At least one mutant must be a function of the budget**, a clamp or
  an offset (SKILL: "weakest thing that would still pass").
- The request's `latency_budget_ms` reaches the queue deadline exactly. Use a
  dev-only seam that records the computed `Duration` and assert exact equality
  at two budget values. Both must differ from the default, and both must be
  unequal to each other.
- `cargo mutants --in-diff`. Survivors are triaged by name.

**Only live proves:** that dropping whole groups under Raven's CPU pressure (93%,
teardown-map L603) keeps the stream watchable. That needs a Raven run after
Idunn B3 (see Sequencing).

**Ledger:** roughly −150 in `run_rudp_mux_once` and +250 for `MediaSendCore`
plus the harness. About +100 net. It buys invariant 2 and the only test path
into the sender.

---

## Cut 3: the CultLib media FEC codec and records

Repo CultLib, a branch from `main`. Tag and publish per CultLib's release
rules. A release is a cut: Soul passes before the tag (SKILL step 5). Q3, Q4
and Q8 are ruled. Consumption waits on the pin gate (Q2, Sequencing step 11).

**3a. Deletes first:**
- `GameCultMediaVideoParityShardRecord` v2 (`C media_stream_contracts.rs:466-517`),
  its schema const (`:46`), `validate_video_parity_record`
  (`C media_stream_wire.rs:323-388`), the `VideoParity` wire arm's v2 decode
  (`:98-103`, `:183-186`), and the v2 tests in `tests/media_stream_wire.rs`
  and `tests/media_stream_contracts.rs`.
- Per Q4: `GAMECULT_MEDIA_AUDIO_CHANNEL` (`C media_stream_wire.rs:41-44`),
  the `"audio"` channel profile (`C rudp.rs:2218-2228`), `"audio"` in the
  `channel_send_options` arm (`:2394`), and the test rows
  (`tests/cultnet.rs:436, 3023-3030`).

**Adds:**
- `gamecult.media_video_parity_shard.v3`. It carries the frame framing fields as
  v2 does, plus `fec_scheme: String`, `block_index`, `block_count`,
  `block_data_start`, `block_data_count` (k), `parity_index`, `parity_count`
  (m), `shard_payload_bytes`, `last_chunk_payload_bytes` and `payload` (bin).
  The donor's v4 field list (`B12`) plus the scheme id is the right shape.
- `gamecult.media_audio_parity_shard.v1`. Fields: `fec_scheme`,
  `base_packet_id`, `base_pts_ticks`, `packet_duration_ticks`, `timebase_*`,
  `deadline_ticks` (the maximum in the block), `data_shard_count`,
  `parity_index`, `parity_shard_count`, `shard_payload_bytes` and `payload`.
  This is the donor's v1 shape plus the scheme id.
- `GameCultMediaWireRecord::AudioParity`, with its validation, record key
  (`B` identity table) and decode arm.
- `media_fec.rs`:
  - `MediaFecPolicy`, fixed per Q8. Video frames split into `⌈n/16⌉` near-equal
    blocks (`k ≤ 16`), with `m = max(2, ⌈k/4⌉)`. Audio is 4+2. The rate is
    carried per record.
  - `protect_video_frame(&[VideoAccessUnitRecord], &policy) -> Vec<GameCultMediaWireRecord>`
    in **send order**, round-robin over blocks (donor `4e2abee`);
  - `protect_audio_block(&[AudioPacketRecord; k]) -> Vec<AudioParity>`;
  - `recover_video_block` / `recover_audio_block(present) -> Result<Vec<payload>, BeyondRepair>`.
  - Backing: `reed-solomon-erasure = "6"` (Q3), default features, no
    `simd-accel`. The KAT pins the wire, so the library is replaceable.
- Update the module doc `C media_stream_contracts.rs:34-41`: audio now has
  parity.

**Rules that must die under their own mutation:**
- Every `≤ m` erasure pattern recovers exactly, for **every** shipped `(k, m)`:
  `k = 1..16` with its ruled `m`, and 4+2. This is exhaustive. The largest is
  `k = 16, m = 4`, about 6,200 patterns, which the probe shows is cheap.
- `m+1` erasures return `BeyondRepair`. They never return `Ok` with wrong bytes.
- KAT: fixed 4×37-byte input, then the exact parity bytes, committed as hex.
  This kills any change of matrix, polynomial or shard order.
- Every data and parity wire record at the policy's maximum shard size is
  `≤ 1431` bytes encoded (invariant 6). The bound is a named constant, not a
  literal copied from Muninn.
- Blocks never span frames. A `protect_video_frame` call with mixed `frame_id`
  errors.
- Send order: within any window of `block_count` consecutive records, no two
  come from one block. That is the burst property. Assert it on the output
  sequence, not on a helper.
- Parity carries its frame's `deadline_ticks` unchanged.

**3b. Port `cultnet-impair`** from the donor
(`D crates/cultnet-impair/src/main.rs`, 696 lines, and
`tests/realtime-impairment/*.toml`) to
`packages/cultnet-rs/examples/cultnet_impair.rs` plus profiles under
`packages/cultnet-rs/tests/impairment/`. This is a separate commit.
- Keep the donor's six deterministic policy tests.
- Drop anything that imports the donor's vendored transport.
- Add a **seeded loss matrix** as a codec test, with no sockets: data plus
  parity through a seeded iid/burst erasure schedule, then `recover`, then a
  recovered-frame ratio per cell. `clean`, `loss-1/3/5pct`, `burst-4/8` are the
  donor's profiles. Commit the table the test prints as the evidence record.
  Assert only the guarantee (for example, `burst-4` against round-robin
  `k=8, m=4`: every block with ≤ m losses recovers), not a tuned ratio.

**Runtime parity.** The media plane is Rust-only today. `grep` finds no
`media_video_parity_shard` outside `packages/cultnet-rs`. Record that the scheme
id names the construction, so a future C# or TypeScript decoder has a KAT to
meet. `reed-solomon-erasure` states it is compatible with Backblaze JavaReedSolomon
and klauspost/reedsolomon. **This claim is not probed**; the KAT is the contract,
not the claim.

**Verification:**
- `cd packages/cultnet-rs && cargo test`
- `cargo mutants --in-diff` on Yggdrasil.
- Negative grep: `rg "media_video_parity_shard.v2|GAMECULT_MEDIA_AUDIO_CHANNEL" packages/`
  is empty.
- A **StreamPixels check**: its vendored CultLib carries `media_stream_*`
  (`StreamPixels/vendor/CultLib/...`). Confirm that StreamPixels compiles
  nothing that names the deleted symbols before the tag. StreamPixels is on the
  ship path, so this is read-only; do not bump it.

**Ledger:**
- deletes: about −180 (v2 record, validator, tests, audio channel);
- adds: +140 (records and validation), +200 (`media_fec.rs` with tests),
  +700 (impair port, an example rather than library surface);
- one new dependency, `reed-solomon-erasure`, which pulls `lru 0.7` and
  `parking_lot 0.11` (seen in the probe build).

Net library surface is about +160. The impair tool is tooling.

---

## Cut 4: Ratatoskr decodes with the CultLib codec

Repo Ratatoskr, `main`. It lands **before** Cut 5, so that the receiver
understands v3 before the producer emits it.

**Deletes first:**
- the XOR stripe recovery: `ParityShard` (`R video.rs:180-186`), `repair()`
  `:224-269`, the stripe doc `:1-8`, and the stripe tests `:638-683`;
- the `insert_parity` v2 path `:346-374`.

**Keeps:** the assembler's lifecycle: admit, expire, evict, repair cadence, and
the keyframe wait.

**Adds:**
- `insert_parity` for v3. It calls `recover_video_block` when a block has
  `≥ k` shards.
- An audio hole recoverer:
  - hold a missing packet only until its block can recover **or** its deadline
    passes, whichever comes first, bounded by the request's budget
    (invariant 1);
  - then recover, or report a hole;
  - "Stamp live audio with the receive clock" (`d11f533`) stays the playout
    owner.
- Stats counted from observation: `parity_recovered_chunks`,
  `parity_beyond_repair_blocks`, `audio_recovered_packets` and `audio_holes`.

**Rules:**
- A v3 block missing `≤ m` shards completes without a repair request. Assert
  that no feedback record names its chunks.
- A block missing `m+1` shards falls through to the repair path and names only
  the unrecovered chunks.
- An audio packet recovered by parity is delivered once, with its original
  `packet_id` and `pts`.
- An audio hole past its deadline is reported and never delivered late.
- `max_frame_age` derives from the request's `latency_budget_ms`, not
  `Duration::from_millis(250)` (`R video.rs:101`). The same two-value exact
  check as Cut 2.

**Verification:**
- Bump the CultLib pin to Cut 3's tag.
- `cargo test -p ratatoskr-core` and the C checks per Ratatoskr's README on
  Yggdrasil.
- `cargo mutants --in-diff`.

**Only live proves:** that OBS renders through recovered frames, and that audio
recovery does not add audible delay. That needs the plugin on Starfire, which
is operator-installed.

**Ledger:** −90 XOR, +150 (v3 intake and audio recoverer). About +60.

---

## Cut 5: Muninn emits CultLib parity, and audio goes lossy

Repo Muninn, `main`. It is gated by the pin gate (Q2, Sequencing step 11).
Per Q5, **video parity ships here, and the audio half waits for Opus**: audio
stays on the reliable `audio` channel until then.

**Deletes first:**
- `MUNINN_VIDEO_PARITY_STRIPES`, `build_video_parity_shards` and
  `video_wire_records_with_parity` (`M media_packetizer.rs:625-754`), and
  their tests;
- `audio_packet_send_payload`'s audio-channel pin (`:1055-1063`) and
  `MUNINN_AUDIO_RUDP_CHANNEL` (`:15-16`), per Q4, in the audio half after Opus;
- the `media_send_payload_pins_audio_to_media_channel` test's assertion
  (`:2252-2280`), which is rewritten to its new truth.

**Adds:**
- The video send state calls `cultnet_rs::protect_video_frame(records, &policy)`.
  Each output becomes one `QueuedMediaGroup` (Cut 2).
- The audio send state accumulates `k` packets and emits parity after the
  k-th. This is the donor's `fec_block` pattern (`D media_packetizer.rs:618-632`, `:812-826`),
  against CultLib types.
- A partial block at stream end or restart is emitted without parity and
  counted. It is not padded with invented packets.
- Audio on the `media` channel, lossy.
- The advertisement names the FEC scheme it emits, only if Ratatoskr needs to
  know before the first record arrives. The default is not to add a field:
  parity is self-describing.
- `MUNINN_RUDP_MEDIA_PROFILE_ID` (`main.rs:76`) bumps to `…lan.v2`, because the
  profile's wire behaviour changed.

**Rules:**
- End to end through Cut 2's drop relay, with the `loss-3pct` seed:
  - every frame whose blocks each lost `≤ m` shards is decodable at the client
    with **zero** repair requests;
  - audio packets lost `≤ m` per block are delivered.
  - The in-test client decodes with CultLib's `recover_*`, so this does not need
    Ratatoskr as a dependency.
- The wire contains no `v2` parity and no `audio`-channel frames. Assert on the
  relay's captured datagrams.

**Harness run on Yggdrasil (the loss-injection acceptance):**
- Cut 2's test driver with the Cut 3b profiles matrix:
  `clean`, `loss-1pct`, `loss-3pct`, `loss-5pct`, `burst-4`, `burst-8`,
  `reorder`, `duplicate`.
- Report per cell:
  - frames complete without repair;
  - frames complete via repair;
  - frames expired;
  - audio recovered and holes;
  - wire overhead, measured as bytes on the relay divided by encoded payload
    bytes.
- Run it once at Muninn `cee7b9c`-equivalent behaviour (XOR, reliable audio)
  and once after, and commit both tables. **The before-and-after table is the
  evidence that the campaign bought something.**

**Only live proves:**
- that NVENC's real frame-size distribution, including IDR spikes of hundreds of
  chunks, fits the chosen `(k, m)` without the overhead pushing past the link;
- that the Raven → Starfire LAN and the mesh-hairpin path (28 Mbps,
  teardown-map L292) both hold.

Protocol: the operator runs the stream for N minutes on each path with the
receive probe's stats captured. Parity overhead against the 28 Mbps hairpin is
the number that decides whether the later adaptive-parity cut (Q8) is needed.

**Ledger:** −130 XOR, +60 of calls into CultLib. **Net about −70.**

---

## Cut 6: native encoder, IDR on request and bitrate adaptation (Q6 = a)

The ruling was against Imagination's recommendation, which was to decide on
harness evidence. This map carries out the ruling. The one cost to name: this is
the campaign's only new native target, and its Windows build is proven nowhere
but Raven.

Three parts, each with its own verification:
- **6a** builds the encoder and needs no GPU;
- **6b** wires Muninn to it;
- **6c** deploys it and accepts it on Raven.

Build hosts and target platforms are named separately throughout, per the
global Build budget rule.

### 6a: the `muninn-video-encoder` package (Muninn repo)

**Why a package.**
- **Owner:** Muninn.
- **Live consumer:** Muninn's activation child, which spawns it.
- **Protected invariants:** invariant 15 (typed commands, observed effect) and
  invariant 16 (one encoder path).
- **Why the existing owner cannot serve:** FFmpeg's C API must be linked. Linking
  libav into `muninn-daemon` would make every Muninn build, on Nightwing,
  Starfire and in CI, need an FFmpeg development root. A separate package
  confines that to one target.
- **What it replaces:** the ffmpeg CLI video child (6b deletes it).
- **Build fan-out:** add `muninn-video-encoder` to the workspace `members`, and
  **not** to `default-members`, so that a plain `cargo build` or `cargo test`
  never needs FFmpeg.

**Layout.**
- `crates/muninn-video-encoder/Cargo.toml`: `[[bin]] muninn-video-encoder`.
  Dependencies: `muninn-contracts` (the command record, H0/H1),
  `cultnet-rs` (framing and message envelope), `anyhow`. Build-dependencies:
  `cc`, plus `pkg-config` for the Linux path only.
- `native/encoder_core.cpp`: the donor loop, **ported**, with these changes:
  - `main`, `parse_options`, the `command_reader` thread and `fail()` are
    deleted. Errors return a code and a message into a caller buffer.
  - Its only entry point is:
    ```c
    int muninn_encoder_run(const muninn_encoder_options*,
                           int (*poll)(void* ctx, muninn_encoder_control* out),
                           int (*write)(void* ctx, const uint8_t* data, size_t len),
                           void* ctx, char* error, size_t error_len);
    ```
    `poll` is called once per captured frame. It returns
    `{force_idr, bitrate_kbps (0 = unchanged), stop}`.
  - The encoder name is an option. `h264_nvenc` (with the donor's option
    dictionary) is for production. `libx264` (`preset=ultrafast`,
    `tune=zerolatency`) is only for the Linux verification build. The capture
    input is an option too: `ddagrab=...` in production, `testsrc2=...` in
    verification.
- `build.rs`:
  - On Windows, read `MUNINN_FFMPEG_ROOT` (required), compile
    `encoder_core.cpp` with `cc` (MSVC, `/std:c++20`), emit
    `cargo:rustc-link-search` for `lib/`, and link `avcodec avdevice avfilter
    avformat avutil`.
  - Refuse to build unless `include/libavcodec/version_major.h` has the pinned
    `LIBAVCODEC_VERSION_MAJOR`. The pin is a named constant in `build.rs`; the
    operator's provisioned build sets it.
  - Emit `cargo:rerun-if-env-changed=MUNINN_FFMPEG_ROOT`.
  - On Linux, find the same libraries through `pkg-config`.
- `src/main.rs`:
  - parses argv: `--input`, `--encoder`, `--framerate`, `--bitrate-kbps`,
    `--gop-frames`, `--frames N` (verification bound);
  - runs a stdin reader thread: `cultnet_rs::LengthPrefixedMessageFramer`,
    then `decode_cultnet_message_from_slice`, then `MuninnVideoEncoderCommandRecord`,
    then validation, then a **bounded** `sync_channel(16)`;
  - implements `poll` by draining that channel into one control value:
    - any IDR sets `force_idr`;
    - the **last** bitrate wins, so coalescing matches the donor's atomic
      exchange;
    - `stop` ends the run;
  - implements `write` to stdout;
  - guards the framer with a maximum frame length of 4 KiB. The framer buffers
    whatever arrives, so a garbage length prefix must not grow memory
    (invariant 2's spirit).

**The command record** `muninn.video_encoder_command.v1`, in `muninn-contracts`.
It is Muninn-owned and process-local, and published to no store.

| key | field | rule |
|---|---|---|
| 0 | `command_sequence: u64` | starts at 1 per encoder process and increases by exactly 1. A gap or repeat is refused and counted, which exposes a writer bug. |
| 1 | `kind: String` | `idr`, `bitrate` or `stop`. Anything else is refused and counted. |
| 2 | `bitrate_kbps: u32` | nonzero only for `bitrate`, within `[250, 100_000]` (the donor's bounds, from its command thread) |
| 3 | `cause: String` | for example `keyframe-request` or `receiver-pressure`. Telemetry only. |
| 4 | `issued_at: String` | telemetry only |

**Verification on Yggdrasil.** Build host Linux x86_64. Target platform Linux,
**not** the Windows shipping target.
- Image: the rust verify image plus
  `apt-get install -y libavdevice-dev libavfilter-dev libavformat-dev libavcodec-dev libx264-dev pkg-config`.
  This runs inside the job's command. A dedicated image is a later optimisation
  if the apt time hurts.
- Unit tests, `cargo test -p muninn-video-encoder`:
  - the record round-trips;
  - a stream of framed commands split at every byte boundary decodes
    identically;
  - `bitrate_kbps` of 249 and 100_001 is refused, and 250 and 100_000 are
    accepted. At least one tested value must sit where a clamp would change
    it; per SKILL, a clamp is the mutant to beat;
  - a sequence gap or repeat is refused;
  - many bitrate commands between two polls yield the last one;
  - an IDR and a bitrate in one poll yield both.
- Integration test, spawning the built binary with `--input testsrc2=size=320x240:rate=30`,
  `--encoder libx264`, `--gop-frames 1000` and `--frames 120`:
  - Parse stdout with Muninn's `h264_annex_b_access_units`. The test
    lives in `muninn-daemon`'s test module so it can reach the parser.
  - After the 20th access unit, write an `idr` command. **Exactly one** keyframe
    access unit appears in AUs 21-23. There is none elsewhere except AU 0,
    because the GOP is 1000.
  - After the 60th AU, write `bitrate 150` (down from 1500). The mean non-key
    AU size over 90-120 is at most half the mean over 30-60.
  - If libx264's in-session reconfigure does not honour this, the test must
    say so, and fall back to asserting that the core received the value
    (a dev-only counter on the C ABI). That fallback is recorded as **not yet
    reached** for Linux, never as unreachable.
- `cargo mutants` on the Rust shell's diff. The C++ core has no mutation tool,
  so it is defended behaviourally by the integration scenario (SKILL).

### 6b: Muninn drives the encoder (Muninn repo)

Depends on:
- Cut 2's `MediaSendCore` seam;
- Cut 5's parity policy, for the ceiling.

**Deletes first:**
- `record_receiver_keyframe_pressure` (`M main.rs:3036-3048`) and its two call
  sites in `run_rudp_mux_once`.
- The ffmpeg CLI video child:
  - `rudp_video_ffmpeg_args` (`M :10343-10421`);
  - `muninn_rudp_video_bitrate_arg`, `muninn_rudp_video_vbv_buffer_arg` and
    `muninn_rudp_video_gop_frames` (`:10296-10311`), which move into encoder
    options;
  - the video `Command::new(&options.ffmpeg_path)` spawn in `run_rudp_mux_once`;
  - `ffmpeg_args`;
  - the CLI-argument tests (around `:12180-12335`), re-homed as encoder-options
    tests where they pin a rule: GOP equals a quarter of the frame rate, and
    the explicit bitrate.
- The `MuninnRudpMediaProfile` fields that fed only the CLI (`video_preset`,
  `video_tune`, `video_b_frames`, `video_rc_lookahead`, `:10279-10285`). The
  runtime-boundary JSON at `:3830-3847` loses them, and its `"recovery"` string
  changes to what is now true.
- `ffmpeg_path` stays for the **audio** child until the Opus cut.

**Adds:**
- `trait EncoderControl { fn request_idr(&mut self, cause: &str); fn set_bitrate(&mut self, kbps: u32, cause: &str); }`
  as a port in `MediaSendCore`.
  - The production implementation owns the encoder's `ChildStdin` on a
    **writer thread** behind `sync_channel(8)`.
  - **Rule: a control write never blocks the send loop.** When the channel is
    full, a pending bitrate command is replaced by the newer one, and an IDR is
    kept.
  - The test implementation records the commands.
- `KeyframeRequestGate`, ported from `D main.rs:2864-2884` with the 500 ms
  cooldown (`D :102`). It is fed the aggregate `requested_keyframes` counter
  across hub receivers. Any receiver's new edge counts.
- `BitrateController`, rewritten from `D :2439-2512`:
  - ceiling `= request.video_bitrate_kbps / (1 + r)`, where `r` is the video
    parity overhead from Cut 5's policy, measured as parity bytes over data
    bytes in the last second, not assumed;
  - floor `= max(ceiling / 4, 1_000)`;
  - start at the ceiling;
  - each 500 ms sample, **back off** to ×0.85 on damage. Damage is new late
    frames, a new keyframe request, new deferred repairs, new
    `groups_dropped_video` (Cut 2), or jitter plus decode queue at or above 75%
    of the request's budget;
  - otherwise, **step up** by `ceiling / 50` after 10 s stable. These are the
    donor code's values (`D :2502-2507`); its doc disagreed, and the Cut 5
    harness may retune the named constants;
  - known limit, recorded: with many viewers, the worst receiver sets the
    rate.
- Spawn:
  - `--video-encoder <path>` resolves relative to the release directory, like
    the loopback script;
  - `--video-encoder-runtime-dir <dir>` is prepended to the child's `PATH`, so
    that the FFmpeg DLLs resolve;
  - the child runs at the existing video priority class
    (`media_child_priority`).
  - A missing or failing encoder fails the request with a typed `detail`
    (invariant 16).
- Observation (invariant 15):
  - `idr_commands`, and `idr_observed` (a keyframe access unit from
    `VideoAnnexBStreamSendState` within 3 frames of a command);
  - `bitrate_commanded_kbps`, and `bitrate_observed_kbps` (encoded bytes per
    second, 1 s window);
  - all four in the progress line and the telemetry surface.

**Rules (each pinned by a test on `MediaSendCore` with the recording
`EncoderControl`):**
- One keyframe-request edge gives exactly one `idr`. A second edge inside
  500 ms gives none. One edge at `cooldown + ε` gives one. The cooldown is
  probed on both sides.
- A recoverable missing chunk (a feedback record with
  `missing_video_chunk_keys` and `requested_keyframe = false`) gives no `idr`
  (invariant 8).
- Damage in one sample gives exactly one ×0.85. It never goes below the floor.
- Stable for 10 s gives exactly one step. It never goes above the ceiling.
- The ceiling is **derived from the request**: test two request bitrates and
  two parity ratios, with exact equality through a dev-only seam that records
  the computed ceiling. Kill a clamp mutant.
- A stalled encoder stdin (the test writer never drains) does not stall
  `MediaSendCore`. Assert the send loop's tick count keeps advancing.
- The Cut 2 harness, driven by a fake encoder that honours `idr` (a
  file-backed Annex B source that emits a keyframe on command): under
  `loss-3pct`, the receiver's frozen interval after a lost reference is
  ≤ RTT + 2 frames, not the 250 ms GOP.

**Verification:** Yggdrasil, `cargo test -p muninn-daemon` and
`cargo mutants --in-diff`. No GPU is needed.

**Ledger:** about −220 (CLI child, args, tests and the log-only keyframe
handler) and +260 (port, gate, controller, spawn and observation). 6a adds
roughly +400: 230 of ported C++ and 170 of Rust shell and tests.

### 6c: recipe, binding and Raven acceptance

Build host **Raven** (Windows 11, MSVC, the NVIDIA driver). Target platform
**Raven**. Builder: Idunn's host actuator, `rust-host` runner, `cargo` only.

**Prerequisites (not Hands):**
- Idunn B3 and `route/b3-declare` merged to Muninn `main` (B19).
- **Operator provisioning on Raven**, a one-time step like the firewall rule:
  - unpack a pinned FFmpeg **shared** build with NVENC enabled (headers, import
    libraries, DLLs) to `C:\GameCult\deps\ffmpeg-<version>-shared\`;
  - record its URL and sha256 in `gamecult-ops/inventory.md` under Raven.
  - Nothing in the deploy path can fetch it (B18). This gap is recorded for
    Idunn (Substrate missing).

**Recipe** (`Muninn/deployment/idunn/raven-muninn.toml`, on top of `2260853`):

    [[steps]]
    id = "build-muninn-video-encoder"
    phase = "build"
    runner = "rust-host"
    required_environment = ["MUNINN_FFMPEG_ROOT"]
    argv = ["cargo", "build", "--locked", "--release",
            "-p", "muninn-video-encoder", "--bin", "muninn-video-encoder"]

    [[steps]]
    id = "accept-nvenc-control"
    phase = "acceptance"
    runner = "rust-host"
    required_environment = ["MUNINN_FFMPEG_ROOT"]
    argv = ["cargo", "test", "--locked", "--release",
            "-p", "muninn-video-encoder", "--features", "nvenc-acceptance",
            "--", "--exact", "nvenc_honours_idr_and_bitrate_commands"]

    [[artifacts]]
    id = "muninn-video-encoder"
    source_kind = "runner-output"
    runner = "rust-host"
    source = "target/release/muninn-video-encoder.exe"
    destination = "muninn-video-encoder.exe"
    executable = true

In `[service]`:
- `required_adjacent_artifacts = ["muninn-video-encoder", "wasapi-loopback-capture"]`;
- the arguments gain `--video-encoder` → literal `muninn-video-encoder.exe`, and
  `--video-encoder-runtime-dir` → binding `ffmpeg_runtime_dir`.

The acceptance test sits behind a feature so that CI and Yggdrasil never select
it. It is the 6a integration scenario with `--encoder h264_nvenc` and
`--input testsrc2`, hardware encode from system-memory frames (no desktop
needed). So **the deploy itself refuses a Raven where NVENC cannot honour a
typed IDR or bitrate command**. Hands confirms that host-native `acceptance`
steps run before promotion (`I src/deployment.rs:71-77` declares the phase).
If they do not, this becomes a build-phase step, and that is recorded.

**Binding** (`gamecult-ops/idunn/yggdrasil/bindings/raven-muninn.toml.in`,
gamecult-ops owns it, an operator-approved ops change):

    [runners.rust-host.environment]
    MUNINN_FFMPEG_ROOT = 'C:\GameCult\deps\ffmpeg-<version>-shared'
    [workload.argument_bindings]
    ffmpeg_runtime_dir = 'C:\GameCult\deps\ffmpeg-<version>-shared\bin'

Both are open maps. The binding vocabulary does not grow, so the actuator needs
no rebuild (`Idunn docs/host-actuator.md:112`). Hands verifies this against the
actuator's binding parser before sealing.

**Deploy:** `sudo idunn up raven-muninn` from yggdrasil, per
`gamecult-ops/runbooks/idunn-host-raven.md`. It is the operator's action; Raven
is a human-used workstation (teardown-map L268-271).

**What only Raven proves, live, with the operator present:**
- `ddagrab` with `h264_nvenc`: a receiver-requested IDR is **observed at
  Ratatoskr** as a keyframe within one RTT plus one frame, while a game is
  running (Raven at 93% CPU is the known condition).
- NVENC in-session reconfigure under a game. The donor recorded 12 → 6 Mbps at
  frame 35 on 2026-07-17, on a pre-extraction body.
- `bitrate_observed_kbps` follows commands on the mesh-hairpin path (28 Mbps)
  and backs off under it.
- DLL resolution from the release directory under the actuator's launch.
- **The payoff step.** Lengthening the scheduled GOP beyond a quarter second is
  what IDR-on-request buys. Do it only after this acceptance, as a one-constant
  change measured against the Cut 5 table and the live run. Until then the
  250 ms GOP stays as the fallback ceiling.

**Ledger 6c:** recipe +25 lines, binding +4.

---

## HID edge for remote play: cuts H0-H5 (Q1, Q9, Q10 ruled)

**What it is for** (Q1, 2026-09-30): a viewer's controller drives a game on
another host, over LAN or mesh, so quick taps must survive loss.
- The **producer** is Muninn on the viewer's host, capturing the pad.
- The **consumer** is Sleipnir on the game host, which drives ViGEm.
- Today the consumer is `raven-sleipnir`, Raven being the game host (B16).

**Anchors in this section.**
- `M` = Muninn `main` `c3520d1`.
- `S` = Sleipnir at Odin `main` `379b826`, `crates/sleipnir-daemon/src/main.rs`.
  H3 moves that file verbatim, so after H3 the same line numbers hold at
  `Muninn/crates/sleipnir-daemon/src/main.rs` until H4 edits it.
- `R` = Ratatoskr `488b5b1`.
- `C` = CultLib `c2a9a6e`, Muninn's pin.
- `Mi` = Mimir `d88f2e6`.
- `Ops` = gamecult-ops `8554e60`.

### The capability mechanism, verified: video today, input mirrored

Q10 orders input to be requested the way Ratatoskr requests video. This is
that request path, read at the SHAs above.

| Step | Video today | Input after H2/H4 |
|---|---|---|
| Producer advertises through CultMesh | Muninn builds `gamecult.media_stream_advertisement.v1` (`M :1122-1166`). It stores it and publishes it to Odin's catalog under its `stream_id`, with role `media-stream-producer` (`M :4146-4166`). | Muninn builds `muninn.input_stream_advertisement.v1` in `input_stream.rs`, and publishes it at the same site with role `input-stream-producer`. |
| Advertised endpoint | `media_endpoint` is explicit (`--media-rudp-advertise`), or derived, **never guessed** (`advertised_media_endpoint`, `M :1173-1196`). `media_connection_id` is `MUNINN_MEDIA_RUDP_CONNECTION_ID`. | `input_endpoint` comes from the same function, parameterised (`advertised_endpoint(explicit, bind)`). `input_connection_id` is `0x6d75_0005`, today's HID id (`M :74`, `S :23`). |
| Recipe declares the capability | `[[provides]] capability = "muninn.media-producer"` (`raven-muninn.toml:93-96`) | `[[provides]] capability = "muninn.input-producer"`, schema `muninn.input_stream_advertisement.v1` |
| Consumer discovers | Ratatoskr pulls the advertisement schema from Odin (`R catalog.rs:64-91`) | Sleipnir pulls the input advertisement schema and its own mapping schema: one pull, typed, with no JSON scraping (B21). |
| Consumer asks | Ratatoskr publishes a request under `media_stream_request_key(stream, receiver)`, role `media-stream-consumer` (`R catalog.rs:95-122`). The request names the sources, and the latency budget defaults from the advertisement (`R :151-179`). | Sleipnir publishes `muninn.input_stream_request.v1` under `input_stream_request_key(stream, receiver)`, role `input-stream-consumer`. It names `device_id`, and `input_budget_ms` defaults from the advertisement. |
| Producer admits and answers | Muninn pulls request documents from Odin each tick (`M :971-1034`), turns them into `MuninnCaptureStreamCommandRecord`s, and writes the answer onto the request key (`M :1038-1117`). | The **same pull** takes both request schemas in one snapshot. Input requests go to the input organ as demand (B23), with **no** detour through a command record, and are answered on the same key. |
| Who dials | The consumer dials the advertised endpoint (`R receiver.rs:4-7`). Its connect payload is its `receiver_id` (`R receiver.rs:40-42`). Muninn listens on a `CultNetRudpServerHub` (`M :3192-3213`). | The consumer (Sleipnir, on the game host) dials the viewer's Muninn. Muninn listens on a `CultNetRudpServerHub` and serves a session only the device its request names. |

**Consequence the operator accepted in Q10:** the producer is on the viewer, so
**the viewer's host admits inbound UDP** for input. That is the cost the
2026-09-10 media inversion refused for video viewers (teardown-map
L486-495). For input it is ruled acceptable. H5 records the firewall rule per
viewer host.

**Two deliberate differences from the video path**, each protecting a named
invariant:
- **Admission as demand, not as `state == pending`** (B23). A restarted
  producer resumes serving every live consumer without a re-request. The video
  path should adopt this too, under its own owner.
- **No command-record detour.** `MuninnCaptureStreamCommandRecord` exists for
  the media child's lifecycle and for legacy readers (teardown-map, item 2 of
  "Next unblocked work"). Input has neither, so it does not grow a second
  command vocabulary.

### Ownership

| Organ | Owns | Does not own |
|---|---|---|
| Muninn capture thread, one per source (H2) | The **only** device reader (B22); sampling (XInput 1 ms, Windows HID every report, joystick events as they arrive); `EdgeDetector`; the latest-state slot; the per-source edge queue | Sessions, windows, epochs, frames |
| Muninn input organ, `crates/muninn-daemon/src/input_stream.rs` (H2) | The advertisement; admission of input requests as demand, and their answers; the input hub and its sessions; one `EdgeWindow` per (session, device); frame emission on `latest`; applying acks | Device reads; mapping; deciding that an edge was applied |
| `muninn-contracts` (H0, H1) | The input records and their key and validation; the HID wire records and their CultNet envelope; `EdgeDetector`, `EdgeWindow` and `EdgeCursor` as pure code | Sockets, threads, devices, Odin |
| Sleipnir (`crates/sleipnir-daemon`, Muninn workspace, H3/H4) | Choosing the device from typed advertisements plus its mapping record; publishing the request; dialling; the `EdgeCursor`; **the single `commit_pad` primitive** (invariant 10); the hold and the request-owned budget (invariant 13); acks; neutralising on staleness | Edge creation, epochs, capture, discovery JSON |
| CultLib | The `latest` channel: Unreliable and Sequenced, fixed by name (`C rudp.rs:2189-2192, 2384-2390`); `CultNetRudpServerHub` with per-session send (`C rudp.rs:1384, 1464, 1475`) | Anything HID-named (invariant 9). **No CultLib change** (under Q11's recommendation). |
| Odin | Catalog storage for the advertisement and request documents. It is schema-agnostic: `odin-daemon` names no media schema either (`git grep`, Odin `379b826`). | Anything input-specific. **No Odin change.** |

**Why one frame on `latest`, and not the donor's reliable `hid.edge` channel.**
- A frame carries the current state **and** every unacknowledged edge of that
  session's window, up to `MAX_EDGES_PER_FRAME`.
- Each frame therefore supersedes the last, which is exactly what a sequenced
  channel delivers: an older frame arriving late is dropped, and nothing is
  lost by that.
- Loss costs one transport tick. H2 sets that tick to 2 ms. Today it is 8 ms
  (B22).
- There is no retransmit timer, no reliable window to stall, and no second
  "assist" path.
- The ruling's "profile-owned channels" is met by using a channel the profile
  already declares, with the delivery it declares.

**How Odin `main`'s pin stays untouched while StreamPixels ships.**
- Nothing in H0-H5 commits to Odin `main`.
- Sleipnir moves into Muninn's workspace (Q9) at Muninn's pins: CultLib
  `c2a9a6e`, and odin-core from Odin `3e96c6c`. That is the commit Odin made
  for exactly this ("CultLib pin a8aedda -> c2a9a6e so a consumer can build
  odin-core without a path override"). It already bumps `sleipnir-daemon`'s
  own manifest to `c2a9a6e`, and its odin-core API equals Odin `main`'s:
  `git diff 3e96c6c 379b826 -- crates/odin-core` touches only `Cargo.toml` and
  two examples.
- From H3 on, Odin `main`'s `crates/sleipnir-daemon` is a **forbidden writer**.
- After StreamPixels ships, **one** Odin commit deletes the crate from Odin's
  workspace. It is a deletion, with no pin change and no other edit.
- Until then two copies of the source exist, and only Muninn's is built and
  deployed from H5 on. Recorded as debt, with its deletion line.

### H0: shape of Muninn's named cut 1 (a constraint, not a new cut)

Named cut 1 moves Muninn's 16 records out of odin-core. For H, it must place
them in a **library** crate in Muninn's workspace, `crates/muninn-contracts`
(`crate-type = rlib`), not in `muninn-daemon`'s binary.

- **Owner:** Muninn.
- **Live consumers:**
  - `muninn-daemon`;
  - `sleipnir-daemon`, a workspace member from H3. It imports
    `MuninnHidControllerStateRecord` today (`S :12`), until H4.
  - `muninn-video-encoder` (6a).
- **Protected invariant:** each Muninn wire record has one definition, and
  every crate that links it resolves one `cultcache-rs`. With Sleipnir in the
  workspace that is structural: one `Cargo.lock`, a path dependency, and no
  git pin to drift. The pass-2 guard test for a cross-repo pin is **not**
  written.
- **Why the existing owner cannot serve:** a binary crate cannot be a
  dependency, and two binaries (`muninn`, `sleipnir`) share these records.
- **What it replaces:** Muninn records defined in Odin's odin-core.

Its dependencies are `cultcache-rs`, `cultnet-rs`, `serde` and `anyhow`, taken
from `[workspace.dependencies]` (H3 hoists the pins there).

**Ordering against H3.** Neither waits for the other. If H3 lands first,
Sleipnir keeps importing the HID record from odin-core. Named cut 1 then
switches both importers in one commit, which is the practical gain of Q9.

### H1: input-stream records, HID wire records and protocol state machines (`muninn-contracts`)

Location per Q11 (recommended): `crates/muninn-contracts/src/input_stream.rs`
(the capability records) and `crates/muninn-contracts/src/hid.rs` (the wire
records and state machines). If Q11 rules CultLib, the same types go to
`CultLib packages/cultnet-rs/src/input_stream_contracts.rs` under `gamecult.*`
ids, and H1 waits for Cut 3's CultLib tag and the pin gate.

**Adds, the capability pair** (types and rules; bodies are Hands'). It is
modelled field-for-field on `C media_stream_contracts.rs:71-183`.
- `muninn.input_stream_advertisement.v1`:
  - `stream_id`, `producer_id`, `label`;
  - `state`, one of `available`, `streaming`, `unavailable`;
  - `device_ids`, `device_labels`, `device_kinds`: parallel, equal length;
  - `default_input_budget_ms`: the default, 100;
  - `input_endpoint`, `input_connection_id`;
  - `updated_at`.
- `muninn.input_stream_request.v1`:
  - `request_id`, `stream_id`, `producer_id`, `receiver_id`;
  - `action`, one of `start` or `stop`;
  - `state`, one of `pending`, `running`, `stopped`, `failed`;
  - `device_id`, `input_budget_ms`, `detail`, `updated_at`.
- `input_stream_request_key(stream_id, receiver_id)`, with the same shape as
  `media_stream_request_key` (`C media_stream_contracts.rs:180`).
- `validate_input_stream_advertisement` and `validate_input_stream_request`.
  They mirror `C :184-290`. Validation refuses:
  - unequal device arrays;
  - `start` without a `device_id`;
  - an `input_budget_ms` of 0 on a `start`, because the consumer fills in the
    advertised default, as Ratatoskr does (`R catalog.rs:174`);
  - an unknown action or state.

**Adds, the wire:**
- `muninn.hid_input_frame.v1`:
  - `device_id`, `stream_id`;
  - `epoch: u64`, never 0;
  - `state_sequence: u64`;
  - `source_timestamp_ns: i64`, telemetry only;
  - `axes: Vec<f32>`;
  - `buttons: Vec<String>`, the held set **after** the last edge in this
    frame's window;
  - `edge_first_sequence: u64`;
  - `edge_buttons: Vec<String>`, `edge_pressed: Vec<bool>`,
    `edge_captured_ns: Vec<i64>`: parallel arrays, equal length, at most
    `MAX_EDGES_PER_FRAME`, contiguous from `edge_first_sequence`.
  - Hands may use a nested record list instead of parallel arrays if the
    `cultcache-rs` derive supports it at the pin. The rule, not the spelling, is
    the contract.
- `muninn.hid_edge_ack.v1`: `device_id`, `epoch`, `applied_through: u64`.
- `MuninnHidWireRecord { Frame, Ack }` with `encode`/`decode` through the
  CultNet message envelope. This is the pattern of CultLib's
  `encode_media_wire_record` (`C media_stream_wire.rs:134-190`), with
  validation on both encode and decode.
- **Deleted from the design:** pass 2's `muninn.hid_subscription.v1`. Device
  selection is the request's `device_id`, and the subscriber's identity is the
  session's connect payload. There is no in-session subscription message.

**Adds, the state machines:**
- `EdgeDetector` (runs on the capture thread):
  - `sample(buttons_now, captured_ns) -> impl Iterator<Edge>` diffs against the
    previous sample: releases first, then presses (the donor order);
  - edges carry no sequence and no epoch. Those belong to a window.
- `EdgeWindow` (one per session and device, on the input tick):
  - `new(epoch_ns)`;
  - `push(edge)` appends, with a sequence contiguous from 1;
  - `frame_edges() -> ≤ MAX_EDGES_PER_FRAME oldest pending`;
  - `ack(epoch, applied_through)`;
  - `rebaseline()`. Past 256 pending it sets the epoch to `+1`,
    `state_sequence` to 0, clears pending, and reports the rotation to the
    caller, which counts it.
- `EdgeCursor` (receiver):
  - `accept_frame(epoch, state_sequence, first_seq, edges) -> Admission`;
  - an epoch below the current one: fenced (a monotonic comparison, with no
    set);
  - a new, higher epoch: take `buttons` as the baseline, set
    `applied_through = first_seq + len - 1`, and return no edges. Taps inside
    the first frame of an epoch are carried by the baseline, not replayed.
    **This is recorded as a deliberate loss at epoch start only.**
  - the same epoch: return the edges with sequence above `applied_through`, in
    order. Duplicates are dropped by sequence, and a gap past 256 is refused.

**Rules that must die under their own mutation.** A property-style test drives
`EdgeDetector` → two `EdgeWindow`s (two subscribers, independent drop seeds) →
frames → a seeded drop, reorder or duplicate → one `EdgeCursor` per
subscriber.
- Every edge captured is delivered **exactly once, in capture order, to each
  subscriber**, under every Cut 3b profile. Use the **same seed tables** as the
  media harness, so one impairment vocabulary serves both.
- One subscriber at `burst-8` never rotates the other subscriber's epoch.
- A press and release captured in one sampling interval arrive as two edges,
  press then release.
- The window in a frame never exceeds `MAX_EDGES_PER_FRAME`. Pending is never
  above 256. At 257 the epoch rotates, and the next frame's epoch is exactly
  one more.
- An ack for a stale epoch changes nothing. An ack for `n` trims exactly
  through `n`.
- The cursor rejects a frame from any epoch below its current one, including
  `current − 1`. Kill the off-by-one comparison mutant.
- **Fit.** A frame with a full window, the longest button names, 16 axes and
  a full held set encodes to ≤ 1,200 bytes (`HID_CONTROLLER_RUDP_MAX_FRAGMENT_BYTES`,
  `M :6140`), so it never fragments (invariant 6's rule, applied to input).
  - A rough count puts 32 edges with 16-character names at about 1,270 bytes.
    **The test sets the constant.** Start at 32, and lower it until the worst
    case fits.
  - Never raise the fragment limit to make it fit.
- The frame round-trips. A frame with unequal parallel arrays, or a
  non-contiguous window, is refused on decode.
- The request and advertisement round-trip. Each refusal listed above has a
  test.

**Verification:** `cargo test -p muninn-contracts` and `cargo mutants --in-diff`
on Yggdrasil, one job.

**Build budget:** one library crate that already exists from named cut 1.
It gains no dependencies and no new target.

**Ledger:** about +550 (the capability pair ~100, the wire and three state
machines ~250, tests ~200). It retires the donor's ~350 HID lines, which never
land.

### H2: Muninn serves input streams (Muninn repo)

Depends on H1 and Q12. It is developed on the same Hands branch as H4 and
merged with it (see Sequencing).

**Authority map.**
- **Owner:**
  - edges: `EdgeDetector` on each source's capture thread;
  - delivery: the input organ in `input_stream.rs`, one `EdgeWindow` per
    (session, device).
- **Inputs:**
  - device samples, capture only;
  - admitted input requests, from the serve tick's single Odin pull;
  - hub sessions and their connect payloads;
  - `hid_edge_ack` frames.
- **Outputs:**
  - `hid_input_frame` per session on `latest`;
  - the advertisement;
  - answers on request keys;
  - counters (frames, rotations, acks, sessions).
- **Derived state:**
  - `muninn.hid_controller_state.v1` receipts become **observation-only**,
    projected from the capture slot. They are no longer read from the device,
    and they are no longer a discovery source.
  - The Eve provider advertisement lists no input streams.
- **Forbidden writers:**
  - the transport tick deciding edges;
  - the serve tick reading a device that a capture thread owns;
  - wall-clock time deciding freshness (telemetry only; donor doctrine,
    `D docs/realtime-delivery-authority.md`);
  - JSON anywhere on this path;
  - an in-session subscription message.
- **Shared paths:** live capture, reconnect, restart (demand re-admission) and
  tests all go through `EdgeDetector` → `EdgeWindow`. A reattaching session
  gets a new window, a new epoch, and a baseline.

**Deletes first** (`M`):
1. **The losing path per Q10: Muninn's outbound client.**
   - `ActiveHidControllerStream` (`:6077-6086`);
   - `create_hid_controller_stream` (`:6724-6763`);
   - `publish_hid_controller_state_to_stream` (`:6765-6778`) and `_inner`
     (`:6780-6863`);
   - in `serve`: `let mut hid_controller_stream` (`:418`) and its argument
     (`:506`);
   - the `hid_controller_stream` parameter of `publish_move_controller_states`
     (`:4982`), and its three calls (`:5026-5029`, `:5073-5076`,
     `:5112-5115`);
   - the option field (`:229`), its default (`:10628`) and its parse arm
     `--hid-controller-rudp-target` | `--hid-controller-udp-target`
     (`:10899-10904`).
2. **The JSON listener body.** Its direction survives; its body does not.
   - `HidControllerRudpSubscription` (`:6143-6149`);
   - `start_hid_controller_rudp_ingress` (`:6151-6177`);
   - `run_hid_controller_rudp_ingress` (`:6179-6340`);
   - `hid_controller_source_matches_subscription` (`:6342-6360`);
   - `hid_controller_record_matches_subscription` (`:6362-6375`);
   - `sanitize_hid_controller_record` (`:6710-6722`);
   - the `serve` wiring at `:429-437`, per Q12.
3. **The per-tick reader as the place where HID state is made.**
   - `read_hid_controller_state_for_stream` (`:6377-6528`),
     `emit_hid_controller_record_if_due` (`:6530-6554`),
     `hid_controller_stream_heartbeat_due` (`:6556-6560`) and
     `hid_controller_axes_changed` (`:6562-6570`).
   - The device-read bodies inside them move to the capture thread; they are
     not rewritten. The emit-if-due throttling dies, because the window
     decides what to send.
   - `ActiveHidControllerRudpSource`, `active_hid_controller_rudp_source` and
     `sync_hid_controller_rudp_sources` (`:6088-6129`) become the capture-set
     reconciler. The hot-plug test `rudp_sources_reconcile_hotplugged_physical_identity`
     (`:14620-14636`) is kept and retargeted.
4. **The double-read fence** (`:5081-5086`), because the capture thread is
   now the only reader (B22).
5. **The JSON discovery surface** (B21).
   - `hid_controller_endpoint` (`:3626-3631`);
   - the device JSON (`:3653-3672`);
   - `transport_input_streams` and `provider_input_streams` (`:3673-3727`);
   - the provider `endpoints` pushes (`:3729-3750`);
   - the `"input_streams"` key (`:3856`) and the `"inputStreams"` key
     (`:3885`).
   - The test `runtime_boundary_advertises_hid_and_move_evidence_streams`
     (`:14072-14172`) is rewritten against the typed advertisement.
6. **The discovery write** `publish_hid_controller_state_to_odin`
   (`:6865-6875`) and its three calls (`:5030`, `:5077`, `:5116`). Its only
   reader is Sleipnir's discovery, which H4 deletes (B21). Receipts through
   `put_hid_controller_state_receipt` (`:5970`) stay.
7. **Rename, not keep beside.** The options `hid_controller_rudp_bind` and
   `_advertise` (`:230-231`, parse `:10906-10916`) become
   `--input-rudp-bind` and `--input-rudp-advertise`. The old spellings are
   refused with a message naming the new ones (the house pattern,
   `M :10890-10894`). `local_ring_enabled` (`:4238`) follows Q12.

**Adds:**
- `crates/muninn-daemon/src/input_stream.rs`, so `main.rs` shrinks. It holds:
  - `input_stream_advertisement(options, live_sources)`, published at `:4146`
    beside the media advertisement, with role `input-stream-producer` and tag
    `muninn.input-stream-advertisement`;
  - `InputDemand`: the admitted requests, keyed by `receiver_id`. It is
    written by the serve tick and read by the input thread (a latest-wins
    `Arc<Mutex<…>>`);
  - `run_input_hub`: a `CultNetRudpServerHub` on `--input-rudp-bind`, open for
    the life of `serve` when configured (`muninn_input_rudp_hub_options`, on
    the model of `M :3232-3241`). On a 2 ms tick it:
    - drains each source's edge queue into the windows of every session that
      requested that device;
    - sends one frame per (session, device) on `latest`, with a 50 ms
      heartbeat when a window holds nothing new
      (`HID_CONTROLLER_RUDP_HEARTBEAT_AFTER`, `:6138`);
    - applies acks;
    - drops sessions after timeout, as the media hub does (`M :3090-3096`).
- **Request intake: one owner for both capabilities.**
  - `sync_media_stream_requests` (`M :971-1034`) becomes
    `sync_stream_requests`. It makes **one** `pull_rudp_catalog_snapshot` with
    `schema_ids = [media request, input request]`.
  - Media requests continue exactly as today.
  - Input requests addressed to `options.host_id` are admitted by `action`
    (B23). An unknown `device_id` is answered `failed` with a typed `detail`.
    A `stop`, or a newer request under the same key, withdraws the demand.
  - `answer_media_stream_request` (`M :1069-1117`) gets an input twin with
    the same keying and dedupe (`answered`). A generic helper is not
    introduced for two call sites.
- `advertised_media_endpoint` (`M :1173-1196`) is parameterised as
  `advertised_endpoint(explicit, bind, command_advertise)`. Media and input
  call the one function, so input inherits "explicit or derived, never
  guessed".
- `MuninnDocuments` (`M :10503-10542`) registers the two input records.
- **A capture thread per source**, ported from `fb4f331`:
  - XInput sampled every 1 ms (`D main.rs:5881`);
  - Windows HID through the existing report reader
    (`start_windows_hid_report_reader_if_supported`, `M :8813`), every report;
  - joystick events as they arrive.
  - Each sample goes through `EdgeDetector` **on the capture thread**. The
    edges go to a bounded per-source queue (256; overflow rebaselines every
    window on that device), and the latest state goes to a slot.
  - The serve tick's HID receipts (`M :5016-5025`, `:5064-5072`,
    `:5102-5111`) read the slot, not the device.
- `raven-muninn.toml`:
  - the argument pair `:62-65` becomes `--input-rudp-bind 0.0.0.0:17887` and
    `--input-rudp-advertise` (binding `input_rudp_advertise`);
  - a second `[[provides]]` block: `muninn.input-producer`,
    `muninn.input_stream_advertisement.v1`, `v1`.
  - The binding rename lands in gamecult-ops with H5.
  - This edits the file `route/b3-declare` also edits, in a different block.
    Rebase onto it if it has merged.

**Rules:**
- A tap shorter than one transport tick, from a scripted fake XInput source,
  becomes two edges in the next frame.
- Two sessions requesting one device each receive every edge once. Dropping
  all frames to one session for 600 ms rotates only that session's epoch.
- A session whose `receiver_id` has no admitted request receives no frame.
  So does a session whose request names another device.
- After a simulated restart, with fresh `InputDemand` and an Odin fixture
  still holding the request at `running`, the session is served again with no
  new request (B23).
- The frame stream from a fake source through a seeded drop relay matches
  H1's property on the wire. Assert on captured datagrams.
- The serve tick never opens a device that a capture thread owns: a dev-only
  per-device open counter reads 1.
- `rg serde_json crates/muninn-daemon/src/input_stream.rs` is empty, and no HID
  path in `main.rs` calls `serde_json`.
- The typed advertisement round-trips through a local CultMesh node fixture,
  as `R tests/catalog.rs:162-180` does for media.

**Verification:** Yggdrasil, `cargo test -p muninn-daemon`, one job. The
XInput and Windows HID readers are `cfg(windows)`, so the scenario uses a
scripted source behind the same capture trait.
- **Starfire:** one Windows compile check of `muninn-daemon`. It is batched
  with H4's, one job and no burners (load budget).
- **Only live proves:** real XInput sampling jitter on the viewer's host (H5).

**Build budget:** `muninn-daemon` only. One new module, no new target and no
new dependency.

**Ledger:** about −750 (the outbound client ~150, the JSON listener and match
functions ~235, the per-tick reader and emit ~195 (its read bodies relocate),
the JSON advertisement ~105, the discovery write and fence ~20, the rewritten
test ~60 net) and about +450 (the organ ~300, capture threads ~100,
intake/answer twin ~50). **Net about −300.**

### H3: Sleipnir moves into Muninn's workspace (Q9); behaviour-neutral

**Source:** Odin `379b826` `crates/sleipnir-daemon/{Cargo.toml, src/main.rs}`,
3,658 lines. That is 8 commits by path, the oldest through `4ad8bdb`, and
`--follow` finds no earlier rename.

**How history moves.** This is the Muninn extraction precedent: Muninn's own
history came out of Odin whole (316 commits, root `2d88408`).
- In a scratch clone of Odin, run
  `git filter-repo --path crates/sleipnir-daemon`.
- In Muninn, `git merge --allow-unrelated-histories` that result. It lands at
  the same path, `crates/sleipnir-daemon/`.
- `git log -- crates/sleipnir-daemon` then works in Muninn. Odin keeps its own
  copy of the history after its deletion commit.

**Per-file changes, all in Muninn:**
- `Cargo.toml` (workspace):
  - add `crates/sleipnir-daemon` to `members`;
  - add a `[workspace.dependencies]` table holding the one CultLib rev
    (`c2a9a6e526390808ef195759056fda0b9a55d8a3`) for `cultcache-rs`,
    `cultmesh-rs` and `cultnet-rs`, plus `odin-core` at Odin `3e96c6c`.
- `crates/muninn-daemon/Cargo.toml:18-20, 24`: the four git pins become
  `{ workspace = true }`. From here, "pins move in lockstep" is one line, not
  a discipline.
- `crates/sleipnir-daemon/Cargo.toml`:
  - the CultLib pins become `{ workspace = true }`. They were `a8aedda` in
    Odin: the pin moves from Odin `main`'s to Muninn's;
  - `odin-core = { path = "../odin-core" }` becomes `{ workspace = true }`;
  - `edition`, `license` and `publish` already read `.workspace`, and
    Muninn's workspace defines all three.
- `crates/sleipnir-daemon/src/main.rs`: **no edit**. The pin move from
  `a8aedda` to `c2a9a6e` is the only behavioural exposure. Odin made the
  same move for this crate in `3e96c6c`.
- `Cargo.lock`: regenerated. It gains `vigem-client` for Windows only, and
  Muninn's recipe keeps building `-p muninn-daemon --bin muninn` (`:14-22`),
  so Raven's Muninn build is unchanged.

**Deletes:** none in Muninn. Odin `main`'s copy becomes a forbidden writer,
and its removal is the post-ship Odin commit.

**Verification:**
- `cargo test -p sleipnir-daemon` on Yggdrasil (Linux, `LoggingBackend`). The
  existing 22 tests (`S :2959-3660`) pass unchanged at the new pin.
- `cargo test -p muninn-daemon` passes unchanged. The workspace-dependency
  hoist is proved by an unchanged `cargo tree -d -p muninn-daemon` for
  `cultcache-rs`: exactly one.
- Starfire: one Windows `cargo check -p sleipnir-daemon`, because
  `vigem-client` is `cfg(windows)`.
- `SleipnirInputMappingRecord` keeps coming from odin-core (`S :13`). Moving
  it into Sleipnir is a later subtraction, not this cut.

**Build budget:** one binary target, `sleipnir`, joins the workspace. It
leaves Odin's workspace at the post-ship commit, so the estate's target count
is unchanged.

**Ledger:** 0 (a move) plus about −10 manifest lines (the pins hoisted).

### H4: Sleipnir requests input and applies edges (Muninn repo)

Developed with H2 on one Hands branch, and merged with it.

**Authority map.**
- **Owner:** `commit_pad(state)` is the only caller of
  `VirtualPadBackend::update`. Today there are five direct calls: `S :387`,
  `:400`, `:472`, `:525`, and the `LoggingBackend` path.
- **Inputs:**
  - typed input advertisements;
  - the mapping record;
  - the answered request;
  - frames on its own session.
- **Outputs:** the request, acks, the pad timeline, and the Eve surface.
- **Derived state:**
  - "available devices" is derived from advertisements;
  - the selected endpoint is derived from the chosen advertisement;
  - neither is discovered from JSON.
- **Forbidden writers:**
  - any `backend.update` outside `commit_pad`;
  - a snapshot writing `buttons` inside an epoch;
  - JSON;
  - `discover_provider_endpoints`.
- **Shared paths:** frames, epoch baselines, staleness neutralisation, mapping
  change and reconnect all reach the pad through `commit_pad`.

**Deletes first** (`S`, identical lines after H3):
1. **The losing path per Q10: Sleipnir's listening mode.**
   - `Options.rudp_bind` (`:42`);
   - the `--rudp-bind` | `--udp-bind` arm (`:2823-2829`);
   - the server branch of `create_rudp_stream` (`:2244-2281`);
   - `[--rudp-bind ADDR]` in `usage` (`:2906`).
2. **The JSON subscription.**
   - `HidControllerRudpSubscription` (`:68-72`);
   - `ActiveRudpStream.last_subscription*` (`:64-65`);
   - `send_hid_subscription_if_due` (`:2283-2321`) and its call (`:426-431`).
3. **The JSON decode and the button coalescing.**
   - `receive_rudp_records` (`:2480-2541`) is rewritten to decode
     `MuninnHidWireRecord`;
   - `coalesce_latest_timed_hid_records_by_device` (`:2543-2565`) and
     `coalesce_latest_hid_records_by_device` (`:2567-2588`) are deleted,
     with their tests (`:2959-2996`). Axes stay latest-wins, because the
     frame carries only current axes.
4. **JSON and Odin-generic discovery** (B21).
   - The second, provider-key pull in `pull_odin_catalog_snapshot`
     (`:950-975`). The first pull's schema list becomes
     `[muninn.input_stream_advertisement.v1, sleipnir.input_mapping.v1]`,
     plus the request schema for the answer.
   - `muninn_provider_keys_for_discovered_hid_hosts` (`:978-1003`) and
     `hid_state_host_id_from_value` (`:1005-1019`).
   - `effective_muninn_hid_endpoint` (`:1826-1833`).
   - `discover_available_hid_devices` through `endpoint_looks_like_socket`
     (`:1845-2131`).
   - `resolve_socket_addr` stays. `discover_muninn_hid_endpoint` through
     `stream_host_hint` (`:2370-2478`) goes.
   - The eight discovery tests (`:3164-3513`) are replaced by three tests over
     typed advertisements.
   - Imports go: `EVE_PROVIDER_ADVERTISEMENT_SCHEMA` (used only at `:957`,
     `:1890` and `:2420`), `MUNINN_HID_CONTROLLER_STATE_SCHEMA`,
     `MuninnHidControllerStateRecord`, `OdinEndpointQuery` and
     `discover_provider_endpoints` (`:10-14`). Sleipnir's odin-core symbols
     fall from 10 to 4: the Eve advertisement and surface records,
     `OdinDocuments`, and `SleipnirInputMappingRecord`. Pass 1 of H5 takes
     `IdunnDaemonHealthRecord` too.
5. **Direct pad writes:** the five `backend.update` sites listed above.
6. **Removed-flag stubs that only reject old spellings:**
   `--command-rudp-bind`, `--command-rudp-advertise` and
   `--odin-cultmesh-rudp` (`:2830-2849`), with their tests (`:3605-3647`).
   They are older than a Muninn-workspace body, and nothing passes them now
   (`Ops idunn-deployment-targets.ps1:131`: `sleipnir --host raven`).

**Adds:**
- `SleipnirDocuments`: `OdinDocuments` plus the two input records, on the
  model of `MuninnDocuments` (`M :10503-10542`). It replaces `OdinDocuments`
  at `S :269, 281`.
- `select_input_device(advertisements, mapping) -> Option<(advertisement, device_id)>`.
  This is **one** function. It matches `mapping.stream_id` against the
  advertisement's `stream_id`, and `mapping.device_filter` against
  `device_ids` and `device_kinds`. The rules of `record_matches_filter`
  (`S :2602-2612`) are kept.
  - Persisted mapping records hold the old stream-id format
    (`host:move_id:hid-controller-state`). They fall through to
    `device_filter` and are rewritten on the next Eve mapping command. No
    migration shim.
- **The request lifecycle,** the Ratatoskr pattern (`R catalog.rs:95-179`):
  - publish `start` on selection, with `receiver_id = sleipnir.<host>` and
    `input_budget_ms` from the advertisement's default;
  - publish `stop` on deselection, on mapping disable, and on clean exit;
  - publish a **new** `start` (a fresh `request_id`) when the session is lost
    or silent for `INPUT_STREAM_RECONNECT_AFTER` (`S :29`). A restarted
    producer re-admits by demand anyway (B23); this covers the Odin document
    being lost.
- **Dial:** the client branch of `create_rudp_stream` (`S :2196-2243`), with
  `connect(receiver_id bytes)` in place of `connect(Vec::new())`
  (`S :2232`, `:2365`), because the hub binds the session to the request by
  its connect payload.
- `EdgeCursor` per selected device.
- An actuation queue drained by `commit_pad`. Frame axes, cursor edges, epoch
  baselines, staleness neutralisation (`INPUT_STALE_NEUTRAL_AFTER`, `S :28`)
  and a mapping change all go through it (invariant 10).
- The 24 ms visibility hold, overridden at the **request's**
  `input_budget_ms`: an edge older than the budget is committed without the
  hold (invariant 13).
- `hid_edge_ack` for `applied_through` after each commit, and every tick while
  nonzero, on `latest`.
- `map_record_to_virtual_pad` (`S :2677-2696`), `apply_pending_learn`
  (`S :1714-1754`) and `input_latency_snapshot` (`S :2614-2651`) take the
  cursor's pad input (axes plus held buttons plus the frame's timestamps)
  instead of `MuninnHidControllerStateRecord`. The mapping logic is unchanged.
- The loop's sleep uses a 2 ms default for `--interval-ms` (`S :2791`
  today: 8).

**Rules** (on a recording backend, asserting the pad timeline, which is the
layer the game samples):
- A quick tap under `loss-5pct` and `burst-8`: the timeline shows press, then
  release; the press is held ≥ 24 ms; nothing is doubled.
- Reordered and duplicated frames produce the same timeline as clean.
- A burst of 20 taps: every one appears. The last is committed no later than
  `input_budget_ms` plus one tick after capture. Kill a mutant that removes the
  budget override. Probe at `budget ± ε`, with the budget taken from a request
  of 60 ms and one of 100 ms. The hold follows the request, not a constant.
- A frame from epoch `current − 1` after a rotation is ignored, and the pad
  shows the new baseline.
- Silence for 1.5 s neutralises through `commit_pad`. Assert that the backend
  was called only from that function (a dev-only call-site counter).
- Selection:
  - a mapping naming a device kind picks the right advertisement among two
    hosts;
  - a mapping naming an absent device publishes no request;
  - a `failed` answer surfaces its `detail` on the Eve surface.
- **End to end on Yggdrasil**, now inside one workspace:
  - H2's `run_input_hub` with a scripted source, through a seeded drop relay
    (the Cut 3b tool's relay and profiles), to Sleipnir's session loop with a
    recording backend;
  - an in-process CultMesh node stands in for Odin, holding the advertisement
    and request.
  - The test lives in `crates/sleipnir-daemon/tests/`, with `muninn-daemon`'s
    organ reached as a dev-dependency only if Hands can make it a lib target
    cheaply. Otherwise the test drives the hub from `muninn-contracts`'s state
    machines plus a thin test driver, as in pass 2.

**Verification:** `cargo test -p sleipnir-daemon` and `cargo mutants --in-diff`
on Yggdrasil, one job batched with H2's. One Starfire Windows check covers both
crates.

**Build budget:** `sleipnir-daemon` only. It gains the `muninn-contracts` path
dependency and drops `serde_json` if nothing else in `main.rs` still needs it.
The Eve surface builds JSON (`S :1021-1455`), so it probably stays.

**Ledger:** about −1,050 (listening mode ~45, JSON subscription ~50,
JSON decode and coalesce ~110, discovery ~420, removed-flag stubs ~20, their
tests ~450) and about +350 (selection and request ~120, cursor, queue and
commit ~130, tests ~250 minus the retired). **Net about −600.**

### H5: deploy, and the viewer's side

**Preflight (Q13):** establish whether `raven-sleipnir` runs today (B16 says
unverified). On Raven, read:
- the scheduled task the legacy target keeps alive
  (`Ops idunn-deployment-targets.ps1:126-138`);
- Sleipnir's published health `sleipnir.cultnet-rudp-input-mirror-health` on
  Odin.

If it runs, Raven's local pad mirror is live. It dials Raven's Muninn at
`10.77.0.4:17887` with JSON, and it breaks the moment Idunn deploys a
`main` that contains H2 (`raven-muninn` admits `refs/heads/main` by
`ref-head`, binding `raven-muninn.toml.in:7-9`).

**Steps:**
- **Sleipnir recipe** `deployment/idunn/raven-sleipnir.toml` in **Muninn's**
  repo, beside `raven-muninn.toml`:
  - a `rust-host` build step, `cargo build --locked --release -p sleipnir-daemon
    --bin sleipnir`, which builds `vigem-client` on Windows;
  - `[[dependencies]] odin.verse-rendezvous`, per Idunn's B3 ruling ("the
    recipe declares readiness"), because Sleipnir publishes its Eve surface
    and its requests to Odin;
  - its presence contract.
- **Presence:** `M src/idunn_presence.rs` (229 lines; Muninn-specific only in
  one doc line and one error string, `:14`, `:146`) moves **verbatim** into
  `muninn-contracts` as `idunn_presence`. Both daemons call it.
  - Delete Sleipnir's legacy `publish_idunn_health_if_configured` and
    `publish_idunn_rudp_health` (`S :792-905`), their three flags
    (`S :2853-2867`), and the `IdunnDaemonHealthRecord` import. That leaves
    three odin-core symbols.
  - Presence's rightful owner is CultLib, because every Idunn-launched daemon
    needs it. Recorded under "Substrate missing"; not moved here.
- **Binding** `idunn/yggdrasil/bindings/raven-sleipnir.toml.in` in
  gamecult-ops, modelled on `raven-muninn.toml.in`, with the same repository
  (`GameCult/Muninn`) and a different `recipe_path`.
  - In the same change, `raven-muninn.toml.in`'s argument binding
    `hid_controller_rudp_advertise` (`:38`) becomes `input_rudp_advertise`.
- **Retire** the legacy `raven-sleipnir` script target
  (`Ops idunn-deployment-targets.ps1:126-138`, `Repo = "Odin"`) and
  `scripts/deploy-raven-sleipnir.ps1` / `restart-raven-sleipnir.ps1`, in the
  same gamecult-ops change. gamecult-ops owns them.
- **The viewer's Muninn.**
  - Starfire's is the legacy `starfire-muninn` script target
    (`Ops :74-86`). `restart-starfire-muninn.ps1:14-15, 74-79` already passes
    an optional HID bind and advertise; these become `--input-rudp-bind` and
    `--input-rudp-advertise`, with Starfire's mesh address advertised.
  - Rebinding `starfire-muninn` to an Idunn recipe is its own ops item, not
    this cut.
  - A viewer outside GameCult's hosts runs Muninn by hand. That limit is
    recorded, not solved here.
- **Firewall, per the Q10 consequence:** each **viewer** host admits inbound
  UDP on its input port (17887) from the game host. Starfire needs one
  Windows Firewall rule. It is scoped to the mesh interface where possible,
  and recorded in gamecult-ops inventory beside Raven's 5220/17887. Raven
  keeps 17887 open only while it also produces input for its own local
  mirror.
- **What only live proves.** Raven with a game, the operator present, and the
  ported `xinput_edge_observer` example (`80adfd3`, 127 lines, into
  `crates/sleipnir-daemon/examples/`) reading the ViGEm pad as the game does:
  - Sleipnir on Raven finds Starfire's advertisement, requests, dials, and the
    request is answered `running`;
  - taps from Starfire's pad reach the game over LAN and over the mesh
    hairpin;
  - the observer shows every tap at ≥ 24 ms visibility;
  - end-to-end input latency, from `edge_captured_ns` to the observer;
  - a `serve` restart on Starfire resumes input with no operator action (B23).

**Ledger (repo code only):** about −115 (legacy health) and +40 (presence
wiring). The recipe and binding are declarations.

---

## Sequencing

Two tracks, media and input. From H3 on they share one repository, Muninn,
and `muninn-daemon/src/main.rs`, so their Hands run **serially in Muninn**,
and in parallel only across repositories and worktrees.

**Now:**
1. **Cuts 0-1** (merged to `main` at `c3520d1`).
2. **Retag `3e96c6c` as `pins/odin-core-cultlib-c2a9a6e`** (Q2 note). This is a
   tag only; Odin `main` is untouched.
3. **Muninn named cut 3** (retire `--idunn-rudp-health`). It removes one of
   Muninn's four Odin symbols. It runs serially with the next item.
4. **Cut 2** (bounded sender and seam; Q7's 250 ms default). Muninn.
5. **Muninn named cut 1 in H0's shape** (`muninn-contracts`). Muninn, serially
   after Cut 2 or before it; never in parallel on `main.rs`.
6. **In parallel, other repos:** Cut 3 (CultLib worktree; Soul passes before
   the tag).

**Input track:**
7. **H3** (Sleipnir into Muninn's workspace). Behaviour-neutral and
   independent of named cut 1. It touches only manifests, the lock and the
   imported crate, so it may run beside Cut 2 without touching `main.rs`.
8. **H1** (`muninn-contracts`), after step 5 and Q11. If Q11 rules CultLib, H1
   joins Cut 3's CultLib work and the input track waits for the pin gate
   (step 12).
9. **H2 and H4** on one Hands branch, after Q12. Soul verifies each cut
   separately. They **merge to `main` together, and only when H5 is ready to
   deploy in the same window**, or with `raven-muninn`'s deployment brake
   engaged, because `raven-muninn` deploys `main`'s head and a new-wire
   Muninn breaks the legacy Sleipnir (Q13).
10. **H5**, after Idunn B3 (the recipe declares its Odin dependency), and the
    Q13 preflight. This is operator deployment.

**Media track:**
11. **Pin gate:** a single-purpose Odin commit bumps odin-core to Cut 3's
    CultLib tag, parented on `3e96c6c`, tagged
    `pins/odin-core-cultlib-<cut3>`. Muninn's `[workspace.dependencies]` (H3)
    moves once, which moves Muninn and Sleipnir together by construction.
    Ratatoskr moves in lockstep.
12. **Cut 4** (Ratatoskr), then **Cut 5** (Muninn video parity).
13. **Cut 6a** (after Cut 2; independent of Cut 5), then **6b** (after Cut 5,
    for the ceiling), then **6c** (after `route/b3-declare` merges, Idunn B3,
    and FFmpeg provisioning on Raven).
14. **Opus** (Muninn named cut 2), then **Cut 5's audio half** (Q5) and the
    `audio` channel deletion taking effect (Q4).
15. **GOP lengthening:** the payoff of 6c, a single-constant change after live
    acceptance.

**StreamPixels ship:**
- no Odin `main` commits;
- no Odin pin change;
- no Idunn edits;
- no StreamPixels CultLib bump.

After the ship: the Odin commit that deletes `crates/sleipnir-daemon`, and
optionally Odin's own CultLib bump, which is the Odin Stability session's call.

**Recipe contention.** 6c and H2 both edit `raven-muninn.toml`, in different
blocks (H2: the input arguments `:62-65` and a `[[provides]]`; 6c: the encoder),
and both sit on top of `2260853` after it merges. Whichever lands second
rebases. H5 creates `raven-sleipnir.toml` beside it.

**Yggdrasil load.** One job at a time for this campaign while StreamPixels'
verification runs. 6a's apt install makes its jobs longer, so batch its unit
and integration runs into one job. H2 and H4 run as one job.

---

## Operator questions

Q1-Q10 are ruled; see the Rulings block. Three forks are open. None blocks the
media track.

**Q11. Who owns the input-stream capability records?**
- Context:
  - The video capability's pair is CultLib's producer-agnostic
    `gamecult.media_stream_*` (`C media_stream_contracts.rs:71-183`). Muninn,
    Ratatoskr and StreamPixels' vendored copy share it across three repos.
  - Input has one producer (Muninn) and one consumer (Sleipnir), and after Q9
    they share one workspace.
  - Any CultLib change moves Muninn off `c2a9a6e`, and only the pin gate
    closes the odin-core diamond that reopens (B11).
- Options:
  - (a) **`muninn-contracts`, as `muninn.input_stream_*`.** H1 needs no CultLib
    change and runs before Cut 3's tag. When a second producer or consumer
    appears, the records move to CultLib under `gamecult.*`. They are live
    documents republished every tick, not stored truth, so that move is a
    schema rename with no migration.
  - (b) **CultLib, as `gamecult.input_stream_*`,** beside the media pair. This
    is the general-library shape a reasonable CultLib consumer would expect,
    and it is the exact mirror. H1 joins Cut 3's CultLib tag, and the whole
    input track waits for the pin gate (step 12), behind Cuts 3 and 4.
- Recommendation: **(a)**. The mechanism is mirrored exactly; only the
  schema's owner differs, and it differs because the capability has one
  implementation today.
- What depends on it: H1's location and ids; whether the input track waits
  for the pin gate; `MuninnDocuments` and `SleipnirDocuments` registrations.

**Q12. Move evidence rides the HID listener. What happens to it?** (B20)
- Context:
  - Muninn sends Move-evidence frames (MessagePack
    `mimir.muninn_move_evidence_stream_frame.v1`) on channel `move-evidence`
    of the HID connection, only to a session that sent the JSON
    `hid.subscribe`.
  - Mimir's runtime does that (`Mi MimirOdinMoveProofEvidenceRingProvider.cs:223`),
    after finding the endpoint in the JSON `inputStreams` H2 deletes.
  - Mimir's provider was last changed on 2026-07-14 (`345c46b`). Raven's
    recipe sets no `--move-evidence-stream`. Nightwing and Starfire configure
    it.
- Options:
  - (a) **Fold Move evidence into the input-stream capability.** Muninn
    advertises a second `muninn.input_stream_advertisement.v1` per host, with
    a `payload_schema` field distinguishing HID frames from Move-evidence
    frames. Mimir requests it by document and dials with its `receiver_id`.
    This needs a Mimir (C#) cut, "H2m", landing with H2.
  - (b) **Keep a Move-evidence-only JSON `hid.subscribe` arm** on the new hub
    until Mimir moves. This keeps JSON on a live wire (invariant 14) and a
    second admission path (Q10), so it needs a named deletion line.
  - (c) **Drop the remote Move-evidence transport**, if Mimir's move proof is
    dormant. Delete the RUDP hand-off (`M :4267-4275`), `rudp_sender`, and the
    bind coupling of `local_ring_enabled` (`:4238`). Mimir re-enters through
    (a) when it next needs the stream.
- Recommendation: **(a) if anyone runs Mimir's move proof today, else (c).**
  (b) only as a bounded bridge under (a).
- What depends on it: H2's deletion list items 2 and 7; whether a Mimir cut
  joins the input track.

**Q13. Is Raven's legacy Sleipnir live, and may its local mirror go dark
between the H2/H4 merge and H5?**
- Context:
  - B16 left "whether it runs on Raven today" unverified.
  - `raven-muninn` deploys `main`'s head. The moment `main` holds H2, Raven's
    Muninn speaks the new wire, and a legacy JSON Sleipnir on Raven stops
    receiving input.
- Options:
  - (a) **Hold the H2/H4 merge until H5 deploys in the same window.** No dark
    interval; the merge waits on Idunn B3.
  - (b) **Merge when Soul passes, with `raven-muninn`'s deployment brake
    engaged** until H5. Raven's Muninn stays on the previous release, and
    media Cuts behind it wait too.
  - (c) **Accept a dark interval** for Raven's local mirror, if the preflight
    shows it is not in use.
- Recommendation: run H5's preflight first. If the mirror is live, choose
  **(a)**. If not, choose **(c)**. (b) blocks the media track and is the
  worst trade.
- What depends on it: when H2 and H4 may merge; whether H5 waits on B3 before
  any of the input track lands on `main`.

---

## Substrate missing (record, do not wait on it)

- **No typed verify transaction.** Every probe went through the ygg-verify
  stopgap, and one run queued behind 5 busy slots. This is Idunn's verify
  campaign.
- **No machine-readable media-path map.** The ownership facts are spread across
  `teardown-map.md` (with 10+ stale rows), a memory file (`media-path-ownership.md`)
  and three repos. The staleness this pass found (B4, B9, L656) is the cost.
- **No synthetic media source anywhere on the path.** Every earlier
  measurement needed Raven, a GPU and a game. Cut 2's seam is the first harness
  that runs off Raven.
- **Idunn host-native targets cannot fetch pinned inputs.**
  `external_inputs` are refused on host-native materialisation (`I host.rs:953-956`).
  So Raven's FFmpeg development root is hand-provisioned and verified only by
  `build.rs`'s version pin and the inventory's sha256. Every future Windows-host
  native dependency hits this. The requirement belongs to Idunn: materialise a
  sha-pinned archive on a host-native runner.
- **No C++ CultNet runtime.** The encoder's typed command channel needs a Rust
  shell around the C++ core (6a). Every native child with a control channel
  will pay this, and the alternative (hand-decoding MessagePack in C++) is the
  Mimir receiver scar. The Rust-shell-over-C-ABI pattern (Ratatoskr's FFI,
  `muninn-move-tracker`'s staticlib) is the house answer until a need justifies
  a C++ runtime.

## Appendix A: probe logs

In this session's scratchpad:
- `donor-fec-tests.log` (9 pass);
- `muninn-media-tests.log` (78 pass);
- `rs-probe.log` and `rs-probe/` (the scratch crate, commit `656865b`);
- `ratatoskr-tests.log` (31 + 2 pass at `488b5b1`).

Pass 2 ran no new builds. Its claims are source reads at the named SHAs. The
6a, H1 and H4 harnesses are the first executions of those designs.
