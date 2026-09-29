# Muninn realtime media robustness: cut map

Imagination pass, 2026-09-30. **Draft, uncommitted.** Self places it. It
belongs beside `Muninn/docs/teardown-map.md` on Muninn `main`; name that
branch in every brief.

Campaign (operator, 2026-09-30): FEC, parity, receiver-requested IDR, bitrate
adaptation, and a loss-tolerant HID edge.

## Status header

| | |
|---|---|
| Map state | Imagination pass 1. No cut dispatched. Seven operator forks are open (end of file). |
| Cuts ready for Hands without a ruling | Cut 0 (doc corrections), Cut 1 (dead-code subtraction), Cut 2 (bounded sender + seam), minus the latency default, which waits on Q7 |
| Cuts blocked on a ruling | Cut 3 (Q2, Q3, Q4, Q8), Cut 4 (Cut 3), Cut 5 (Q2, Q5), Cut 6 (Q6), HID (Q1) |
| Heads read | Muninn `cee7b9c`; CultLib `069ecc3` (`main`); Ratatoskr `488b5b1`; Odin `main` `379b826`; donor Odin `origin/codex/muninn-bounded-realtime-send` `80adfd3`; Odin tag `attic/claude-cultlib-pin-c2a9a6e` `3e96c6c`; Mimir `d88f2e6`; Idunn `docs/route-continuity-cut.md` at its current HEAD |

The CultLib media plane is byte-identical between Muninn's pin `c2a9a6e` and
`main` `069ecc3`: `git diff c2a9a6e main` over `media_stream_contracts.rs`,
`media_stream_wire.rs`, `rudp.rs` and `transport.rs` is empty. Every CultLib
anchor below holds at both SHAs.

---

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
| `MuninnVideoBitrateController` (`D :2439-2512`) | **Rewrite** (Cut 6, only if Q6 = a) | Its ceiling halving assumes 50% FEC overhead, and its doc and code disagree. It must key on the chosen parity rate, and aggregate across hub receivers, which the donor predates. |
| `native/muninn-video-encoder` (C++, 265 lines) plus build script | **Held at the donor tag, pending Q6** | A new native target with FFmpeg dev libraries on Raven, built by an Idunn `rust-host` runner whose vocabulary is `cargo`. That is a large liability, bought only if IDR and bitrate adaptation earn it. |
| Bounded `sync_channel` plus per-frame groups, capacities 256/512 (`D main.rs:85-87, 2023-2028, 2356-2369`) | **Port the shape**, rewrite against the hub loop (Cut 2) | Exactly invariant 2. |
| `cultnet-impair` (696 lines, seeded loss/burst/jitter/reorder/dup/stall, CSV) | **Port to CultLib** `packages/cultnet-rs/examples/` (Cut 3b) | It is a CultNet transport tool. `media_delivery_probe.rs` already sits there and expects it (its header, `C examples/media_delivery_probe.rs:5`). |
| HID edges, epochs and acks in Muninn + Sleipnir + vendored `rudp.rs` | **Park** until Q1 | Purpose unknown, and it breaks invariant 9. |
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
| Audio parity shard `gamecult.media_audio_parity_shard.v1` | `(stream_id, session_id, base_packet_id, parity_index)` | Emitted after its k-th data packet. It expires with the block's latest deadline. | Same. |
| FEC scheme id (e.g. `rs-gf256-v1`) | A string on each parity record | Changes only with a new schema version. A KAT pins it. | CultLib. |
| `(k, m)` policy | Carried per parity record (`block_data_count`, `parity_count`) | Fixed per stream in the first cut. Adaptive only if Q8 rules so. | Producer (Muninn). |
| Latency budget | `gamecult.media_stream_request.v1.latency_budget_ms` (key 11) | Fixed for a request. A new request restarts the child (teardown-map L533). | **Consumer** asks. Producer honours it, or refuses in `detail`. Default per Q7. |
| Keyframe request | `requested_keyframe` edge on feedback | Counted per receiver. It triggers at most one IDR per cooldown. | Receiver asserts. Producer's gate decides actuation (Cut 6). |
| Encoder bitrate (live) | Process-local, no record | Adjusted within `[floor, ceiling]`. Reported in telemetry. | Producer's controller (Cut 6, Q6). |
| HID edge | Unknown until Q1 | — | — |

---

## Cut 0: correct the Muninn maps against git (docs only)

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

The latency default waits on Q7. The rest does not.

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
    Q7 constant.
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
rules. A release is a cut: Soul passes before the tag (SKILL step 5). Blocked on
Q3, Q4 and Q8. Q2 blocks consumption, not this cut.

**3a. Deletes first:**
- `GameCultMediaVideoParityShardRecord` v2 (`C media_stream_contracts.rs:466-517`),
  its schema const (`:46`), `validate_video_parity_record`
  (`C media_stream_wire.rs:323-388`), the `VideoParity` wire arm's v2 decode
  (`:98-103`, `:183-186`), and the v2 tests in `tests/media_stream_wire.rs`
  and `tests/media_stream_contracts.rs`.
- If Q4 = yes: `GAMECULT_MEDIA_AUDIO_CHANNEL` (`C media_stream_wire.rs:41-44`),
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
  - `MediaFecPolicy { max_block_data: u16, parity_ratio / fixed m }`;
  - `protect_video_frame(&[VideoAccessUnitRecord], &policy) -> Vec<GameCultMediaWireRecord>`
    in **send order**, round-robin over blocks (donor `4e2abee`);
  - `protect_audio_block(&[AudioPacketRecord; k]) -> Vec<AudioParity>`;
  - `recover_video_block` / `recover_audio_block(present) -> Result<Vec<payload>, BeyondRepair>`.
  - Backing: `reed-solomon-erasure = "6"` (Q3), default features, no
    `simd-accel` C build.
- Update the module doc `C media_stream_contracts.rs:34-41`: audio now has
  parity.

**Rules that must die under their own mutation:**
- Every `≤ m` erasure pattern recovers exactly, for each shipped `(k, m)` (at
  least 4+2 and 8+4, and the policy's maximum block). This is exhaustive; the
  probe shows it is cheap.
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

Repo Muninn, `main`. It is gated by Q2 (the pin diamond) and Q5 (after Opus, or
over PCM).

**Deletes first:**
- `MUNINN_VIDEO_PARITY_STRIPES`, `build_video_parity_shards` and
  `video_wire_records_with_parity` (`M media_packetizer.rs:625-754`), and
  their tests;
- `audio_packet_send_payload`'s audio-channel pin (`:1055-1063`) and
  `MUNINN_AUDIO_RUDP_CHANNEL` (`:15-16`) if Q4 = yes;
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
the number that decides Q8.

**Ledger:** −130 XOR, +60 of calls into CultLib. **Net about −70.**

---

## Cut 6: encoder control, IDR on request and bitrate adaptation

Mapped only as far as Q6 allows. It is not dispatchable until Q6 is ruled.

If Q6 = (a), port the donor encoder:
- **Deletes first:** `record_receiver_keyframe_pressure` (`M main.rs:3036-3048`).
  The ffmpeg CLI video child (`rudp_video_ffmpeg_args`, `:10343-10421`) is
  retired **only after** the native encoder is installed by the Idunn recipe.
  Keep one path, not two: the donor kept both behind `video_encoder_path.is_some()`
  (`D main.rs:1942`), which is the split to avoid.
- **Adds:**
  - `native/muninn-video-encoder` from the donor, plus a recipe build step in
    `deployment/idunn/raven-muninn.toml`. **The Idunn recipe is being edited by
    the route-continuity campaign right now; see Sequencing.**
  - An `EncoderControl` port in `MediaSendCore` (Cut 2's seam).
  - The ported `MuninnKeyframeRequestGate`.
  - The rewritten `BitrateController`:
    - its ceiling is `configured × 1/(1+parity_ratio)` from Cut 5's policy, not
      `/2`;
    - it aggregates damage across all hub receivers: any receiver's damage backs
      off, which is recorded as a known limit for the single-encoder,
      many-viewer case;
    - its recovery step and interval are named constants whose values the
      harness picks.
- **Rules:**
  - One invalidation edge gives exactly one `IDR` command per cooldown. Count
    the bytes written to the control port.
  - Damage inside a sample window gives exactly one multiplicative back-off.
  - Stable for the interval gives exactly one additive step, never above the
    ceiling.
  - Assert on the command bytes, as the donor did (`D main.rs:12756-12801`).
- **Only live proves:** that NVENC honours a forced IDR at the next frame and
  reconfigures bitrate without a restart. The donor recorded both on 2026-07-17,
  but on a pre-extraction body.

If Q6 = (b) or (c), Cut 6 is a one-commit deletion: `record_receiver_keyframe_pressure`,
and `requested_keyframes` stays as telemetry only. Retire the "IDR on request"
and "bitrate adaptation" goals from this campaign with the ruling recorded.

---

## HID edge: not mapped (Q1)

The donor's HID work spans Muninn, Sleipnir (in Odin) and odin-core's HID
record, and it hardcodes channel names into the transport (B13). Its purpose
decides everything:
- whether edges are needed or snapshots suffice;
- whether the channel is reliable, or lossy with replay;
- whether this belongs in Muninn at all, or in the Sleipnir/Mimir input path.

Nothing is specified until Q1 is answered. When it is:
- the record moves with named-cut 1 (odin-core → Muninn) **first**, so the edge
  record is born in its owner;
- delivery uses CultLib's profile-owned channel delivery, not hardcoded names.

---

## Sequencing

Against Muninn's named next cuts (teardown-map L644-668, L700-705) and the
StreamPixels ship:

1. **Now, no contention:** Cut 0 and Cut 1 (Muninn docs and subtraction). They
   touch neither the Idunn recipe nor any pin.
2. **Muninn named cut 3, retire `--idunn-rudp-health`:**
   - it can run before or after Cut 1 on the same `main.rs`, serially, never
     in parallel;
   - it removes `IdunnDaemonHealthRecord`, one of the four Odin symbols (B11),
     which shrinks the diamond;
   - the three scripts that still pass the flag (`scripts/health-*.ps1`) go
     with it.
3. **Cut 2 (bounded sender).** Muninn only, no pin. The latency default waits on
   Q7.
4. **Muninn named cut 1, move 16 records out of odin-core:**
   - it must land **before** any HID work (Q1);
   - it is independent of FEC, because the media records are already in CultLib;
   - it does not by itself unblock a CultLib bump, since the Eve records and
     `OdinDocuments` remain (Q2).
5. **Muninn named cut 2, Opus:**
   - it is the prerequisite for Cut 5's audio half if Q5 = after (B2);
   - the video half of Cut 5 does not wait for it.
6. **Cut 3 (CultLib).**
   - It runs in its own CultLib worktree.
   - It is independent of Muninn's Hands, and can run in parallel with
     steps 1-5 in a different repo and worktree.
   - Soul passes before the tag.
7. **Pin gate (Q2).** An odin-core revision at Cut 3's CultLib pin, or a severed
   odin-core.
8. **Cut 4 (Ratatoskr), then Cut 5 (Muninn).** Bump the pins in lockstep.
9. **Cut 6**, only if Q6 = (a), and only after the Cut 5 harness table shows
   visible damage that FEC and repair do not cover.

**StreamPixels ship:**
- Its critical path is Idunn B3, then merge `route/s3-c2`, back up `control.cc`,
  the read-only check, and `idunn up` (morning file item 3).
- This campaign stays off it:
  - no Odin `main` pin change (Odin `main` pins `a8aedda` for StreamPixels'
    RUDP, `379b826`);
  - no Idunn edits;
  - no StreamPixels vendored-CultLib bump.
- Cut 3's tag is read-only to StreamPixels until StreamPixels chooses to move.

**Live verification on Raven waits for two Idunn events:**
- B3 lands;
- `raven-muninn`'s recipe gains its Odin declaration (Idunn `route-continuity-cut.md:32-35`:
  "`raven-muninn` and Heimdall both publish to Odin, so their recipes declare
  it").

Before that, `idunn up raven-muninn` may be refused at admission. **Muninn Hands
must not edit `deployment/idunn/raven-muninn.toml`** while that declaration is in
flight. Cut 6(a)'s recipe step is the only cut here that touches it, and it goes
last.

**Yggdrasil load.** Every harness and mutation run here goes through ygg-verify
(5 slots, shared). While StreamPixels' verification is running, cap this
campaign at one job at a time. All five slots were busy during this pass.

---

## Operator questions

One fork each, with a recommendation. Nothing downstream of an open question
gets written.

**Q1. What is the loss-tolerant HID edge for?**
- Context:
  - Today Muninn sends whole controller snapshots (`buttons` as a held set)
    to Sleipnir.
  - A press and release between two samples, or lost with its packet, never
    reaches the virtual pad.
  - The donor fixes that with ordered, acknowledged edges and epochs, across
    Muninn, Sleipnir (Odin) and a patched transport.
- Candidate purposes, which lead to different machines:
  - (a) **remote play**: a viewer's controller drives a game on another host,
    over LAN or mesh, so quick taps must survive loss;
  - (b) **same-LAN couch input**, where loss is rare and snapshots at a high
    rate suffice;
  - (c) input as **evidence** for Mimir or tracking, not actuation.
- Which hosts are the source and the sink, and over which path?
- Recommendation: answer this before anything is specified. Park the donor's
  HID commits at the parked tag. If (a), the edge record is born in Muninn after
  named-cut 1, and delivery uses CultLib's profile-owned channels.

**Q2. How does Muninn get a CultLib pin with the FEC codec in it?**
- Context:
  - Muninn's `odin-core` comes from Odin `3e96c6c`, reachable only through the
    tag `attic/claude-cultlib-pin-c2a9a6e`. That commit's only change was to bump
    odin-core's CultLib pin.
  - odin-core's records are `DatabaseEntry`, so a Muninn CultLib bump without a
    matching odin-core gives two `cultcache-rs` copies and a type mismatch.
  - Odin `main` pins an **older** CultLib (`a8aedda`) for StreamPixels, and must
    not move during the ship.
- Options:
  - (a) another single-purpose attic-tagged Odin commit that bumps odin-core to
    Cut 3's pin (the `3e96c6c` precedent). Cheap, repeated per CultLib bump,
    and invisible on any branch;
  - (b) sever Muninn from odin-core first: named-cut 1, retire idunn-rudp-health,
    and rehome the Eve records and `OdinDocuments` registration. This delays FEC
    by those cuts;
  - (c) keep the codec local to Muninn and Ratatoskr. **Rejected**: two codecs
    (invariant 5).
- Recommendation: (a) for this campaign, with (b) continuing as Muninn's own
  cuts. Record that each (a) is a debt paid again on every CultLib bump. Also
  consider a `parked/` or `pins/` tag name instead of `attic/`, so the tag reads
  as live.

**Q3. Implement the erasure code, or depend on `reed-solomon-erasure`?**
- Context:
  - The donor's hand-written GF(256) has no multi-erasure video decoder and no
    KAT.
  - The crate was probed on Yggdrasil: exhaustive recovery at 4+2 and 8+4, any
    shard length, about 11 µs per 8+4 block. It pulls `lru 0.7` and
    `parking_lot 0.11`, and its last release is 2022.
  - `reed-solomon-simd` is faster to encode but refuses odd shard sizes and
    decodes in about 230 µs.
- Recommendation: depend on `reed-solomon-erasure` 6, without `simd-accel`, and
  pin the bytes with a KAT so that the library, not our wire, is replaceable.
  The fallback, if dependency age is disqualifying, is to port the donor
  arithmetic plus a general Gaussian decoder (about 150 lines we own).

**Q4. Delete the reliable `audio` channel and carry audio lossy with parity on
`media`?**
- Context:
  - The channel was created on 2026-09-10 so audio could be reliable beside
    lossy video.
  - Its only purpose is reliability. Receivers dispatch by schema, not by
    channel, so a lossy audio channel duplicates `media`.
  - The teardown map's own fix direction (L256-261, L322-324) is audio lossy
    with parity.
- Recommendation: yes. Delete it in CultLib Cut 3 (−15 lines of transport
  special-casing), and move Muninn's audio in Cut 5. This reverses a recorded
  decision, which is why it is asked and not defaulted.

**Q5. Audio parity before Opus, or after?**
- Context:
  - PCM packets are 3,840 bytes, which is 3 datagrams each (B2). Parity would
    protect 3-fragment shards and cost about 1.5 Mbps at 4+2.
  - Opus (Muninn's named cut 2, measured at 183 kbps) gives single-datagram
    packets, and the donor's padded framing ports directly.
- Recommendation: after Opus. Cut 5 ships video parity immediately. The audio
  half follows the Opus cut, and audio stays on today's reliable channel until
  then, which is LAN-proven: 0 expired on 2026-09-11.

**Q6. Encoder control: is IDR-on-request and bitrate adaptation worth a native
encoder?**
- Context:
  - Recovery today is a quarter-second all-IDR GOP (`-g fps/4 -forced-idr 1`).
    A lost reference freezes for at most 250 ms whether or not anyone asks.
  - Asking for an IDR only helps if the GOP gets longer, which saves the bitrate
    those IDRs cost.
  - The ffmpeg CLI cannot take a live IDR or bitrate command. The donor solved
    this with a 265-line C++ libavdevice/NVENC child, proven live on 2026-07-17
    on a pre-extraction body.
- Options:
  - (a) port the native encoder, with an Idunn recipe step on Raven;
  - (b) keep the CLI and the 250 ms GOP; no IDR request, no adaptation;
  - (c) keep the CLI with NVENC `-intra-refresh 1` and a long GOP. This removes
    the IDR spikes and bounds recovery by the refresh period, but still gives
    no requested IDR and no bitrate adaptation.
- Recommendation: (b) now. Decide between (a) and (c) on Cut 5's harness table
  and the live hairpin run. Build (a) only if the stream visibly fails where
  bitrate adaptation would have helped. This is the one cut in the campaign that
  buys a new native target.

**Q7. What is the default latency budget?**
- Context:
  - The producer defaults to 2,000 ms. The receiver gives up at 250 ms (the
    July ledger's recovery budget).
  - The request owns the budget (invariant 1), but Ratatoskr sends 0 and so
    inherits the 2 s default.
  - FEC recovers within one frame. NACK repair needs a few RTTs: under 1 ms on
    LAN, about 75 ms on the mesh hairpin.
- Recommendation: 250 ms default in the advertisement, with both ends deriving
  from the request. A consumer on a slow path asks for more.

**Q8. Parity rate: fixed, or adaptive to observed loss?**
- Context:
  - At 25-50% overhead, RS blocks cut iid frame failure by 2-3 orders of
    magnitude compared with today's XOR (probe table).
  - Overhead against the 28 Mbps hairpin at 12 Mbps video is significant.
  - Adaptive needs new receiver-feedback fields (received, recovered and lost
    chunk counts) and a controller.
- Recommendation: fixed in Cut 5, at `k ≤ 16`, `m = max(2, ⌈0.25k⌉)` for video
  and 4+2 for audio, carried per record. Revisit with the live hairpin numbers.
  Adaptive is a later cut with its own feedback-record extension.

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

## Appendix A: probe logs

In this session's scratchpad:
- `donor-fec-tests.log` (9 pass);
- `muninn-media-tests.log` (78 pass);
- `rs-probe.log` and `rs-probe/` (the scratch crate, commit `656865b`);
- `ratatoskr-tests.log` (31 + 2 pass at `488b5b1`).
