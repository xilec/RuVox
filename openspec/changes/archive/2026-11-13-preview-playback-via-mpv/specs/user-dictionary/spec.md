## MODIFIED Requirements

### Requirement: Editor pronunciation preview

The add and edit forms SHALL render a play control at the right edge of both
the «Слово» and the «Как произносить» fields, next to the existing help icon.
The control SHALL be disabled while its field is empty or invalid: «Слово»
requires one valid source token as defined by "Dictionary entry semantics", and
«Как произносить» requires a non-empty spoken form whose optional `+` mark is
well formed.

Playing «Слово» SHALL speak the current reading of the token: the Rust
normalization pipeline run with the active engine's profile, the saved
dictionary applied and the entry being added absent (in edit mode the saved
entry is part of the dictionary, so its saved `to` is spoken). Playing «Как
произносить» SHALL speak the typed spoken form verbatim, with the `+` mark kept
only when the active engine's text profile renders stress and stripped
otherwise — the same decision the pipeline makes when it inserts the entry's
`to`.

A preview SHALL use the active engine only: it SHALL NOT switch engines, load a
second model, or change any setting. When the selected scope belongs to an
engine other than the active one, the form SHALL state which engine is speaking
(«Прослушивание — движком «Piper»…»). The global scope SHALL never raise the
hint.

A preview SHALL NOT create or modify an entry, timestamps, queue state,
synthesis events or playback state; the narration channel SHALL be untouched
except for the pause described below. While a preview is being synthesized the
control SHALL show progress and both preview controls SHALL be blocked; while
the audio plays the control SHALL show a stop affordance and a second click
SHALL stop the audio. Activating the other field's control while a preview
plays SHALL stop the current audio and play the new preview. Starting a preview
while narration is playing SHALL pause the narration (not stop it) through the
existing playback path at the moment the preview is requested — before the
synthesis starts — and again right before the audio plays; the narration SHALL
NOT be resumed afterwards.

Playback SHALL use the application's audio player (the bundled mpv) in an
instance reserved for previews and SHALL NOT use the webview's HTML media
stack; the narration player's loaded entry, position and events SHALL remain
untouched.

A failing preview SHALL render one short inline message inside the form and
SHALL leave the entered values untouched. Repeating a preview of identical
input on the same engine and voice SHALL be served from an in-memory cache and
SHALL NOT synthesize again; any dictionary write (save or import) SHALL retire
the cache. Closing the editor SHALL stop a playing preview and delete its
temporary WAV; the player instance stays alive for later previews and is
destroyed on application exit. A response that arrives after the editor closed
SHALL be discarded and SHALL NOT leave a clip playing.

#### Scenario: Controls appear next to the help icons

- GIVEN the user opens the add form
- WHEN the form renders
- THEN the «Слово» and «Как произносить» fields each show a play control next to
  their help icon

#### Scenario: Empty field disables its control

- GIVEN the add form is open and both fields are empty
- WHEN the form renders
- THEN both preview controls are disabled

#### Scenario: Invalid field disables its control

- GIVEN the «Слово» field contains "getUserData()" and the «Как произносить»
  field contains a malformed mark
- WHEN the form renders
- THEN the «Слово» control and the «Как произносить» control are both disabled

#### Scenario: Word preview asks for the normalized reading

- GIVEN the «Слово» field contains "IPv6" and the field is valid
- WHEN the user activates its play control
- THEN the preview command is invoked with "IPv6" and mode "normalize"

#### Scenario: Spoken-form preview asks for the spoken reading

- GIVEN the «Как произносить» field contains "айпи ви ш+есть" and the field is
  valid
- WHEN the user activates its play control
- THEN the preview command is invoked with "айпи ви ш+есть" and mode "spoken"

#### Scenario: Edit mode speaks the saved reading

- GIVEN the saved dictionary maps "kubectl" to "куб контрол" and the user is
  editing that entry
- WHEN the user activates the play control of the «Слово» field
- THEN the preview command is invoked with "kubectl" and mode "normalize", and
  the audio speaks "куб контрол"

#### Scenario: Synthesizing blocks both controls

- GIVEN a preview request is in flight
- WHEN the form renders
- THEN the control that started the preview shows progress and both preview
  controls are disabled

#### Scenario: Playing control stops the audio

- GIVEN a preview is playing
- WHEN the user activates the same control again
- THEN the audio stops and the control returns to its play state

#### Scenario: Playing the other field replaces the preview

- GIVEN a preview of «Слово» is playing
- WHEN the user activates the play control of «Как произносить»
- THEN the current audio stops and the new preview plays; only the second
  control shows the playing state

#### Scenario: Playback stays off the webview media stack

- GIVEN the dictionary editor is open with a valid «Слово»
- WHEN the user activates its play control
- THEN the synthesized WAV is handed to the preview player and the dialog
  contains no audio or video element

#### Scenario: Repeated preview uses the cache

- GIVEN a preview of "IPv6" already played on the current engine and voice
- WHEN the user activates the same control again
- THEN the audio plays again without a new preview command call

#### Scenario: A dictionary change retires the cache

- GIVEN a preview of "IPv6" is cached
- WHEN the user saves or imports a dictionary
- THEN the next activation of the same control calls the preview command again

#### Scenario: Narration is paused, not stopped

- GIVEN narration is playing
- WHEN the user starts a preview
- THEN the playback pause command is called before the preview request starts
  synthesizing, the playback stop command is not, and narration stays paused
  after the preview ends

#### Scenario: Over-long preview input reports the limit inline

- GIVEN the «Как произносить» field holds a valid spoken form longer than 200
  code points
- WHEN the user starts a preview
- THEN the inline message reports that the input is too long
  (`preview.text_too_long`) and the entered value is unchanged

#### Scenario: Preview failure stays inline

- GIVEN the active engine's model is not installed
- WHEN the user starts a preview
- THEN one short inline message appears inside the form and the entered values
  are unchanged

#### Scenario: Foreign scope names the speaking engine

- GIVEN the TeraTTS scope is selected and the active engine is Piper
- WHEN the add form renders
- THEN the form states that the preview plays with the Piper engine

#### Scenario: Active scope shows no engine hint

- GIVEN the Piper scope is selected and the active engine is Piper
- WHEN the add form renders
- THEN no engine hint is shown

#### Scenario: Closing the editor stops the preview

- GIVEN a preview is playing in the add form
- WHEN the user closes the editor
- THEN the preview player is stopped, the temporary WAV is deleted, and the
  control returns to its play state

#### Scenario: A late response after close is discarded

- GIVEN a preview request is in flight in the add form
- WHEN the user closes the editor before the command returns
- THEN the response is discarded: no audio starts and no preview state is
  restored
