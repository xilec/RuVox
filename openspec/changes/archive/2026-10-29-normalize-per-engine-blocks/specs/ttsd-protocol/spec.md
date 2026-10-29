# Delta: ttsd-protocol

## MODIFIED Requirements

### Requirement: Synthesize Command

The `synthesize` request SHALL carry the normalized text and synthesis
parameters, and ttsd SHALL write the resulting WAV file to the given path:

```json
{
  "cmd": "synthesize",
  "text": "<normalized text>",
  "speaker": "xenia",
  "sample_rate": 48000,
  "out_wav": "/absolute/path/<uuid>.wav",
  "char_mapping": [ { "norm_start": 0, "norm_end": 1, "orig_start": 0, "orig_end": 3 } ],
  "chunk_limit": 900
}
```

`text`, `speaker`, `sample_rate`, and `out_wav` are required; `char_mapping`
and `chunk_limit` are optional and omitted from the JSON entirely when absent.
`chunk_limit` SHALL be a positive integer in Unicode codepoints and SHALL bound
the size of every chunk the sidecar produces; when absent, the sidecar SHALL
apply its documented default (900) so an older caller keeps working. The limit
SHALL come from the caller's text profile — the sidecar SHALL NOT be the home of
the number. The parent directory of `out_wav` is guaranteed to exist.

On success ttsd SHALL respond:

```json
{
  "ok": true,
  "timestamps": [ { "word": "привет", "start": 0.0, "end": 0.5, "original_pos": [0, 6] } ],
  "duration_sec": 0.9
}
```

`timestamps` may be empty for very short text; `duration_sec` is the total WAV
duration in seconds. Before synthesizing, ttsd SHALL reject empty/whitespace
text with `bad_input` and SHALL reject any request while the model is not
loaded with `model_not_loaded`.

#### Scenario: successful synthesis

- GIVEN the model is loaded
- WHEN a valid `synthesize` request arrives
- THEN ttsd writes the WAV file at `out_wav` and responds with word timestamps
  and `duration_sec`

#### Scenario: synthesis before warmup

- GIVEN the model is not loaded
- WHEN a `synthesize` request arrives
- THEN ttsd responds `{ "ok": false, "error": "model_not_loaded", "message":
  "Silero model is not loaded; send warmup first" }`

#### Scenario: empty text rejected

- GIVEN the model is loaded
- WHEN a `synthesize` request with empty or whitespace-only `text` arrives
- THEN ttsd responds with `error: "bad_input"` and no file is written

#### Scenario: caller's chunk limit bounds every chunk

- GIVEN a `synthesize` request with `chunk_limit` 400 and a text longer than 400
  codepoints
- WHEN the sidecar synthesizes it
- THEN no chunk exceeds 400 codepoints and the concatenated audio covers the
  whole text

#### Scenario: omitted chunk limit falls back to the default

- GIVEN a `synthesize` request without `chunk_limit` and a text longer than 900
  codepoints
- WHEN the sidecar synthesizes it
- THEN chunks are bounded by the 900-codepoint default

#### Scenario: invalid chunk limit is rejected

- GIVEN a `synthesize` request with `chunk_limit` 0 or a negative value
- WHEN the request arrives
- THEN ttsd responds with `error: "bad_input"` and no file is written
