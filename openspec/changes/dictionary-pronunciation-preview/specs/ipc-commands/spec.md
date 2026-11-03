# Delta: ipc-commands

## ADDED Requirements

### Requirement: Pronunciation Preview Command

The system SHALL provide `preview_pronunciation(text, mode)` returning
`{ spoken_text, wav_base64, duration_sec }` — a one-word audio preview for the
dictionary editor. `mode` SHALL be `"normalize"` or `"spoken"`; any other value
SHALL be rejected with a typed error.

With `mode = "normalize"` the command SHALL run the same Rust pipeline pass as
`preview_normalize` under the active engine's text profile (saved dictionary
applied, no entry being added) and SHALL speak the resulting text as-is. With
`mode = "spoken"` the command SHALL validate the text with
`tts_normalize::dictionary::spoken_form` and SHALL keep the `+` mark only when
the active engine's text profile has `stress != Stress::None`, using the
stripped form otherwise; a malformed mark SHALL be rejected with a typed error.
Text that resolves to an empty spoken form SHALL be rejected with a typed
error.

Synthesis SHALL go through the active engine's `synthesize` with the voice
resolved by the shared per-engine voice rule and the sample rate from the
config, writing into a unique temporary file under the application cache
directory, reading its bytes, deleting the file, and returning the bytes as
base64 together with the spoken text and the duration.

The command SHALL NOT call the queue synthesis path: it SHALL NOT create or
modify an entry, its status, timestamps, queue state, synthesis events or the
generation snapshot, and SHALL NOT transcode the WAV.

While a queue entry is inside the engine's `synthesize` call the command SHALL
reject with the typed error `preview.engine_busy` instead of synthesizing, and
concurrent previews SHALL be serialized.

#### Scenario: normalize mode returns the pipeline reading

- GIVEN the saved dictionary maps "kubectl" to "куб контрол" and the frontend
  invokes `preview_pronunciation("kubectl", "normalize")`
- WHEN the command returns
- THEN `spoken_text` is the pipeline output "куб контрол", `wav_base64`
  decodes to a WAV file and `duration_sec` is positive

#### Scenario: spoken mode keeps the mark for a stress-capable engine

- GIVEN the active engine's text profile renders stress (Piper)
- WHEN `preview_pronunciation("айпи ви ш+есть", "spoken")` is invoked
- THEN `spoken_text` is "айпи ви ш+есть" with the mark preserved

#### Scenario: spoken mode strips the mark for a stress-less engine

- GIVEN the active engine's text profile has `Stress::None` (Silero Native)
- WHEN `preview_pronunciation("айпи ви ш+есть", "spoken")` is invoked
- THEN `spoken_text` is "айпи ви шесть"

#### Scenario: malformed mark is rejected

- GIVEN the text "Моск+ва" carries a mark that is not immediately before a
  vowel
- WHEN `preview_pronunciation("Моск+ва", "spoken")` is invoked
- THEN the command rejects with a typed error and no synthesis runs

#### Scenario: preview leaves no trace

- GIVEN a successful preview of "IPv6"
- WHEN `get_entries` is called afterwards
- THEN no new entry, timestamp file or queue state exists, no `entry_updated`
  or synthesis event was emitted, and the temporary audio file was deleted

#### Scenario: preview is refused while the queue synthesizes

- GIVEN a queue entry is inside the engine's `synthesize` call
- WHEN `preview_pronunciation("IPv6", "normalize")` is invoked
- THEN the command rejects with `preview.engine_busy` and the engine is not
  called a second time
