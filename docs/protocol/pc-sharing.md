# Computer-to-computer camera, microphone, sound and notifications

> Status: **draft.** Builds on [protocol v0](v0.md), [mic](mic.md),
> [webcam](webcam.md), [notifications](notifications.md) and
> [deck](deck.md); breaking changes are allowed until v1. License: CC BY 4.0.

A paired computer can use another paired computer's camera as a webcam, its
microphone, its sound, and show its notifications. **No new wire messages**:
a computer takes the part a phone takes in each of those services, so
everything below is the existing message set with a computer as the source.

Two roles, named as in [desktop control](desktop-control.md): the **source**
is the computer whose camera, microphone, sound or notifications are shared;
the **listener** is the computer that uses them.

## 1. Capabilities

| ID | Offered by | Meaning |
|---|---|---|
| `camera.stream` | source | Streams its camera (existing, [webcam](webcam.md) §1) |
| `mic.stream` | source | Streams its microphone (existing, [mic](mic.md) §1) |
| `sound.share` | source | Plays its sound to a computer that asks, with `pc.audio.set` and the "play here" values (new, §4). Offered only while sound sharing is on, unlike `pc.audio`, which every computer offers for volume control |
| `notify.mirror` | source | Sends `notify.*` events (existing, [notifications](notifications.md) §1) |
| `camera.virtual` | listener | Receives and decodes a webcam stream (existing) |
| `mic.listen` | listener | Takes a microphone stream (existing) |
| `sound.play` | listener | Plays the sound a source streams to it (new, §4) |

A computer offers a source capability (`camera.stream`, `mic.stream`, `sound.share`, `notify.mirror`) only when the user turned that kind of
sharing on **and** the hardware is there (a camera, a microphone). Every kind
is **off until the user turns it on** on the source, as the other sharing
features are.

The capability matrix lists what a pair can do as `desktop.camera`,
`desktop.mic`, `desktop.sound` and `desktop.notifications`.

## 2. Camera

As [webcam](webcam.md), with the source in the phone's place: the listener
sends `webcam.start` (camera, size, frame rate, bitrate), the source answers
`webcam.ok`, and the source opens the `webcam/video` stream with H.264 (or
H.265 when the listener offers `camera.virtual.hevc`).

- A computer's camera is `"front"` in `camera`, whatever its position; `"back"`
  is answered with the same camera. `max_height` in the `Config` packet is
  the camera's own limit.
- The source **never** starts its camera silently. It starts only for a
  listener the user allows (the `webcam` device toggle, on the source) while
  camera sharing is on, and it shows that the camera is in use, with a way to
  stop, for as long as it is.
- Either side stops with `webcam.stop`, or by ending the stream with code
  `12`. The source stops its camera when the stream ends.

## 3. Microphone

As [mic](mic.md): `mic.start` from the listener, the source's `ok`, then
`mic/pcm` packets from the source. A computer streams 48 kHz mono 16-bit PCM,
10 ms a packet, from its default input device.

- The `mic` device toggle on the source decides which computers may use its
  microphone, and sharing must be on.
- The source shows that its microphone is in use, with a way to stop, for as
  long as it is.

## 4. Sound

A listener hears a source's sound with the messages "Phone as speaker" uses
the other way round:

```
listener → source   t = "pc.audio.set"  b = { volume: 201 }        // play it here
listener → source   t = "pc.audio.set"  b = { volume: 200 }        // stop
source   → listener audio.pcm datagrams, or t = "audio.pcm" frames
```

`201` and `200` are `PHONE_SPEAKER_ON` and `PHONE_SPEAKER_OFF` ([deck](deck.md)
§2.5): the name stays, the meaning is "play this computer's sound on the device
that asked". The source captures what it plays (system loopback), converts it
to 16-bit PCM and sends it only to the computer that asked.

- The source needs the `media` device toggle on for the listener to start.
  Stopping is always allowed.
- `audio.pcm` reaches **any** paired peer's event stream, so a listener plays
  frames **only** from a computer it asked, and only while it is listening.
  Anything else is dropped without a sound.
- A listener offers `sound.play`. A source doesn't send to a computer that
  doesn't know it, and the matrix asks that computer's user to update.

## 5. Notifications

The source's notifications go to paired computers as `notify.posted` and
`notify.removed` ([notifications](notifications.md) §3), the way a phone's go
to a PC. A computer shows them as it shows a phone's.

- The `notifications` device toggle applies on both sides.
- The source sends only when the user turned notification sharing on. It must
  not send a notification that came from Noddy itself or from another
  computer's shared notification, so two computers never echo each other's.
- `app` is `pc.windows` on Windows, and the platform's name for others.
- Implementations **MUST NOT** log notification content (v0 §11).

## 6. Privacy

- Every kind is off by default on the source and has its own switch.
- Each paired computer has its own `webcam`, `mic`, `media` and
  `notifications` switch on the source, as phones do.
- The source shows when its camera or microphone is in use, and who is using
  it, until it stops.
- Nothing here crosses the internet: the sessions are the same end-to-end
  encrypted connections as every other feature (v0 §11).
