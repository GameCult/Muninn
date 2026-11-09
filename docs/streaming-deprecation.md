# Streaming deprecation (Meta, 2026-11-06)

Muninn's streaming role and the OBS media receiver are superseded in the streaming
space. Scope: screen and loopback-audio capture, the media sender, and the receiver at
github.com/GameCult/Ratatoskr (forge mirror `GameCult/Ratatoskr-receiver`; the forge repo
`GameCult/Ratatoskr` is an unrelated chat client and is not covered). Nothing has been
deleted. The lists below come from file names and docs, not from a consumer audit;
confirm each consumer before cutting.

## What depends on the streaming role

- Nothing in StreamPixels on main. StreamPixels `tools/showcheck` (PR #2) read Muninn's
  activation log and the receiver's OBS log until it was rescoped on 2026-11-06 (a762f69)
  to server, overlay and OBS only; the Muninn and Ratatoskr hops are gone from it and
  survive only on its parked branch `kiko/showcheck-muninn-chain`.
- Docs here that describe it: `docs/muninn-media-streaming.md`,
  `docs/realtime-robustness-cut.md`, and the receiver sections of `docs/teardown-map.md`.
- Operations: `scripts/activate-muninn-raven-*`, `scripts/health-*`,
  `scripts/restart-*`, and `deployment/idunn` (the health and restart scripts also serve
  the non-streaming daemon, so they stay).
- The receiver repo's own README and the Muninn README link to each other.

## Removal candidates (streaming only; verify consumers first)

- `crates/muninn-daemon/src/media_send.rs`, `media_send/tests.rs`,
  `media_packetizer.rs`, and the media wiring in `main.rs`.
- Media-related records in `crates/muninn-contracts` (`records.rs`, `lib.rs`), if no
  non-streaming consumer reads them.
- `scripts/activate-muninn-raven-av-srt.cmd`, `scripts/activate-muninn-raven-media.ps1`,
  `scripts/wasapi-loopback-capture.ps1`.
- `docs/muninn-media-streaming.md` (move to history rather than delete if it is cited).

## Keep

Sensor serving, PS Move tracking (`muninn-move-tracker`, `muninn-psmoveapi-tracker`,
`native/`), and `sleipnir-daemon` input are outside this notice. Do not remove them under
it. `idunn_presence.rs` matched a media search term only; it is presence, not streaming.
