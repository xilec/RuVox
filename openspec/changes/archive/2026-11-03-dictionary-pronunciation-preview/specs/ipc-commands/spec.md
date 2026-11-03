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

The command SHALL reject input longer than 200 Unicode code points with
`preview.text_too_long` before running the pipeline or the engine.

Synthesis SHALL go through the active engine's `synthesize` with the voice
resolved by the shared per-engine voice rule and the sample rate from the
config, writing into a unique temporary file under the application cache
directory, reading its bytes, deleting the file, and returning the bytes as
base64 together with the spoken text and the duration. A failed read of the
temporary file SHALL be rejected with `preview.audio_read_failed`.

The command SHALL NOT call the queue synthesis path: it SHALL NOT create or
modify an entry, its status, timestamps, queue state, synthesis events or the
generation snapshot, and SHALL NOT transcode the WAV.

The command and the queue's engine stage SHALL share one engine serialization:
the queue SHALL hold it around its `synthesize` call, and the command SHALL
acquire it without waiting and SHALL reject with the typed error
`preview.engine_busy` instead of synthesizing while it is held. A preview
therefore never queues behind a narration; in the other direction the queue MAY
wait for at most one short preview. Concurrent previews SHALL be serialized by
the same mechanism.

Failures SHALL be typed `CommandError` objects. The command's own validation
and I/O failures SHALL serialize as `type = "internal"` with `code` one of
`preview.empty_text`, `preview.invalid_mode`, `preview.invalid_stress`,
`preview.text_too_long`, `preview.engine_busy`, `preview.dir_create_failed`,
`preview.audio_read_failed`. An engine failure SHALL serialize as
`type = "synthesis_error"` and SHALL keep the engine's site code in `code`
(`voice_not_installed`, `bundle_not_installed`, `model_not_loaded`,
`teratts_cancelled`, `native.*`), falling back to `preview.synthesis_failed`
when the engine reports no site code.

#### Scenario: normalize mode returns the pipeline reading

- GIVEN the saved dictionary maps "kubectl" to "куб контрол" and the frontend
  invokes `preview_pronunciation("kubectl", "normalize")`
- WHEN the command returns
- THEN `spoken_text` is the pipeline output "куб контрол", `wav_base64`
  decodes to a WAV file and `duration_sec` is positive

#### Scenario: spoken mode keeps the mark for a stress-capable engine

- GIVEN the active engine's text profile accepts an explicit stress mark
  (`Stress::Auto` for Silero Native, `Stress::Manual` for TeraTTS)
- WHEN `preview_pronunciation("айпи ви ш+есть", "spoken")` is invoked
- THEN `spoken_text` is "айпи ви ш+есть" with the mark preserved

#### Scenario: spoken mode strips the mark for a stress-less engine

- GIVEN the active engine's text profile has `Stress::None` (Piper or Silero
  through ttsd)
- WHEN `preview_pronunciation("айпи ви ш+есть", "spoken")` is invoked
- THEN `spoken_text` is "айпи ви шесть"

#### Scenario: malformed mark is rejected

- GIVEN the text "Моск+ва" carries a mark that is not immediately before a
  vowel
- WHEN `preview_pronunciation("Моск+ва", "spoken")` is invoked
- THEN the command rejects with `code = "preview.invalid_stress"` and no
  synthesis runs

#### Scenario: input over the limit is rejected

- GIVEN a text of 201 Unicode code points
- WHEN `preview_pronunciation(text, "normalize")` is invoked
- THEN the command rejects with `type = "internal"` and
  `code = "preview.text_too_long"`, and neither the pipeline nor the engine
  runs

#### Scenario: preview leaves no trace

- GIVEN a successful preview of "IPv6"
- WHEN `get_entries` is called afterwards
- THEN no new entry, timestamp file or queue state exists, no `entry_updated`
  or synthesis event was emitted, and the temporary audio file was deleted

#### Scenario: preview is refused while the queue synthesizes

- GIVEN a queue entry is inside the engine's `synthesize` call
- WHEN `preview_pronunciation("IPv6", "normalize")` is invoked
- THEN the command rejects with `preview.engine_busy` without waiting, and the
  engine is not called a second time

#### Scenario: the queue waits for a preview

- GIVEN a preview is inside the engine's `synthesize` call
- WHEN a queue entry reaches its engine stage
- THEN the queue waits for the preview to finish and only then calls the
  engine, so one engine never serves two callers at once

#### Scenario: a failed WAV read is reported

- GIVEN the engine succeeded but the temporary WAV cannot be read
- WHEN `preview_pronunciation("IPv6", "normalize")` returns
- THEN the command rejects with `type = "internal"` and
  `code = "preview.audio_read_failed"`, and the temporary file is removed

#### Scenario: engine failures keep their site code

- GIVEN the engine rejects the request with `voice_not_installed`
- WHEN `preview_pronunciation("IPv6", "normalize")` returns
- THEN the command rejects with `type = "synthesis_error"` and
  `code = "voice_not_installed"`
