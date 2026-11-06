# Muninn

> **Deprecation notice (Meta, 2026-11-06).** Muninn's *streaming* role (screen and audio
> capture, the media sender, and the OBS media receiver at
> [github.com/GameCult/Ratatoskr](https://github.com/GameCult/Ratatoskr), not the forge chat
> client of the same name) is superseded in the streaming space. Do not build new work on it.
> Nothing is deleted yet; what depends on it and what looks removable is in
> [docs/streaming-deprecation.md](docs/streaming-deprecation.md). The non-streaming roles
> (sensors, PS Move, input for Sleipnir) are not covered by this notice.


**Local capture broker for the GameCult swarm.**

Muninn resolves what data streams a host can offer, advertises them, and serves
them to CultMesh peers on request. Screen and loopback audio, cameras, PS Move
controllers and their optical markers, HID and XInput gamepads, Quest access.

Muninn running does not mean Muninn is broadcasting. Each feed it can reach is
*available* for another service to request. Mimir asks for sensors. Sleipnir asks
for input. A stream asks for screen and audio. Muninn does not decide what any of
them are for.

**Naming note.** Muninn's docs say "Ratatoskr" for the OBS media receiver, which lives at
[github.com/GameCult/Ratatoskr](https://github.com/GameCult/Ratatoskr) (forge mirror:
[GameCult/Ratatoskr-receiver](https://eureka.gamecult.org/forge/GameCult/Ratatoskr-receiver)).
The forge repo `GameCult/Ratatoskr` is an unrelated project, Void's Cinny-based chat client.

Extracted from [Odin](https://github.com/GameCult/Odin) with its full history.
Muninn had lived as three crates inside the repository of the service that
discovers it, with its document schemas owned by `odin-core` — an inversion of
the ownership it needs to hold. Same shape as the
[Idunn](https://github.com/GameCult/Idunn) extraction of 2026-09-05, and done
deliberately after it.

---

## Status

**This repository does not build yet.** It is a faithful history extraction, not
a finished cut. `muninn-daemon` still depends on `odin-core` by path for the
twenty record types and schema constants it uses, and that dependency is one of
the things being removed.

See [docs/teardown-map.md](docs/teardown-map.md) for the working map: current
body, the authorities currently fused into one 14,567-line `main.rs`, schema
decomposition, extraction blast radius, and the measured diagnosis of the audio
decay that prompted the rebuild.

## Authority

- **Owner:** Muninn owns local source enumeration, capability advertisement,
  per-consumer stream grants, and transport of granted streams.
- **Input:** locally reachable devices; typed capture stream commands from
  requesting peers.
- **Output:** CultMesh Media Streams and typed sensor records.
- **Not Muninn's:** the media protocol itself (a general CultMesh contract, owned
  by CultLib), discovery and rendezvous (Odin), deployment and continuity
  (Idunn), the hue programs that decide what colour a Move should be (Mimir),
  and any consumer's lowering — OBS, browser, or otherwise.

Muninn is one producer of a general contract. It is not the contract.
