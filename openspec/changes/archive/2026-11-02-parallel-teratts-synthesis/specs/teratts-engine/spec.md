## MODIFIED Requirements

### Requirement: Chunking under the seconds limit

The engine SHALL honor its profile's `Limit::Secs(20)` by deriving a
conservative codepoint budget from a measured chars-per-second constant,
splitting with the shared splitter, and validating every chunk with the
duration predictor. A chunk predicted above the model's hard 25-second call
limit SHALL be re-split at the last sentence boundary inside it (then a clause
boundary, then whitespace, then a hard split) and re-validated. The engine
SHALL insert the profile's paragraph pause (250 ms) after a chunk that ended at
a paragraph break and the segment pause (200 ms) between chunks, and SHALL
check cancellation before dispatching each chunk to a lane. A cancellation
raised while chunks are running SHALL discard their audio and still return the
cancellation error.

#### Scenario: A long article is split into bounded chunks

- GIVEN a multi-paragraph text whose single-call duration would exceed 25 s
- WHEN it is synthesized
- THEN every chunk is predicted at or below 25 s and the chunks cover the text
  in order without dropping content

#### Scenario: An over-budget chunk is re-split

- GIVEN one sentence whose predicted duration exceeds 25 s
- WHEN the chunker validates it
- THEN it is split at an inner boundary and each part is re-validated

#### Scenario: Pauses are inserted between chunks

- GIVEN two consecutive chunks without a paragraph break
- WHEN their audio is joined
- THEN 200 ms of silence separates them; after a paragraph break the pause is
  250 ms

#### Scenario: Cancellation stops between chunks

- GIVEN a synthesis in progress with several chunks
- WHEN cancellation is requested after the first chunk
- THEN synthesis stops with a cancellation error, chunks that had not been
  dispatched are not synthesized, and no output WAV is written

## ADDED Requirements

### Requirement: Parallel chunk synthesis across lanes

The engine SHALL synthesize the chunks of one utterance concurrently across a
pool of independent ONNX session sets ("lanes"), at most one chunk per lane at
a time. The pool SHALL default to four lanes — or fewer when the machine's
thread budget cannot give every lane at least one intra-op thread — and the
engine's intra-op thread budget (half the logical cores, as before) SHALL be
split evenly across the lanes, never below one thread per lane. Every chunk
SHALL be planned before any lane starts: split under the seconds limit,
validated by the duration predictor, and assigned its segment's language and
synthesis options. Planned chunks SHALL be assembled strictly in text order;
lane scheduling MUST NOT change the produced samples, the pause placement or
the timestamps. An engine loaded with `n` lanes and `t` intra-op threads per
lane SHALL therefore produce the same WAV bytes and timestamps as an engine
loaded with one lane and the same `t` threads.

#### Scenario: The default pool splits the thread budget

- GIVEN a thread budget of 12 intra-op threads
- WHEN the default pool is derived from it
- THEN it has 4 lanes with 3 threads each; a budget of 3 threads yields 3 lanes
  with one thread each, and a budget of 1 thread yields a single lane

#### Scenario: Lane count does not change the audio

- GIVEN a multi-chunk text synthesized by an engine with one lane and by an
  engine with four lanes, both at one intra-op thread
- WHEN their outputs are compared
- THEN the WAV samples, the pause lengths and the timestamps are identical

#### Scenario: Buffered and streamed output stay identical on a lane pool

- GIVEN an engine loaded with the default pool
- WHEN the same text is synthesized through the buffered and the streaming
  paths
- THEN both emit the same samples in the same order, with the same pause parts
  and part offsets

#### Scenario: Timestamps keep one ordered timeline across lanes

- GIVEN a mixed-language utterance whose chunks complete out of order
- WHEN the timestamps are inspected
- THEN they are sorted, monotonic, non-overlapping, and each word's range stays
  inside its chunk's audio range
