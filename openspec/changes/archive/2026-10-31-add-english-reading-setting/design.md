## Context

`TextProfile::Route` decides who reads a fragment: the pipeline (`Narrate`) or
the engine (`Native`). #307 gave every engine a profile, and the #311 probe
decided per engine what it can read: TeraTTSv2 reads Latin prose, identifiers
and code; Piper reads code but its Russian voice makes transliterated prose
sound better; the Silero family reads neither. The profile therefore encodes
both a capability ("the engine can read English") and a preference ("read
English as English"), and only the first belongs to the engine.

Constraints that shape the implementation:

- The pipeline is shared by all engines and is configured at runtime through
  two doors — startup and `update_config` — the same way `code_block_mode` is.
  A synthesis already in flight must finish on the values it captured.
- Spans are the adapter contract: a `ProseLatin` span means "the engine reads
  this in English". Producing one over transliterated text would break the
  TeraTTS segment path and word highlighting.
- The identifier route is a profile decision and stays one: a user who asks
  for transliterated prose on TeraTTS still gets native `getUserData`, because
  identifiers are not English phrases.
- `CharMapping` behaviour must not change for any existing combination.

## Goals / Non-Goals

**Goals:**

- One explicit user choice for Latin prose, independent of the engine.
- `auto` reproduces today's output byte-for-byte on every engine, so existing
  installs do not change on upgrade.
- The profile declares only what the engine can do.
- The choice reaches the next synthesis without a restart, and a capability
  fallback never rewrites the stored value.

**Non-Goals:**

- Per-user routes for identifiers, code blocks, abbreviations or structural
  fragments.
- Per-engine overrides of the setting, extra UI per voice, or new engines.
- Improving transliteration quality or native English quality.

## Decisions

### D1. `reads_latin_natively: bool` replaces `route.latin_prose`

The profile keeps one route per fragment kind *the engine decides for itself*
(`identifier`, `code_block`) and gains one capability flag for prose. The flag
means "the engine can read an English phrase in English"; the user decides
whether it should. `validate()` requires `"en"` in `languages` when the flag is
set, because a native prose span carries the language it is read in, and
requires the flag for `identifier: Native` — an engine that reads identifiers
natively reads Latin by definition, and the profile may only claim what the
engine can do.

Piper keeps `languages: ["ru", "en"]` (it reads Latin code acceptably) but sets
the flag `false`: the probe rejected English prose on the Russian voice, and
its prose route was `Narrate` for exactly that reason.

Rejected: a two-variant capability enum. The flag has one question and one
answer; a `Reading`-shaped value would invite callers to treat it as a policy
again.

Rejected: keeping `latin_prose` and adding the flag. Two sources of truth for
the same fragment would let the profile and the setting disagree, and the
pipeline would need to define which one wins.

### D2. The pipeline resolves the setting; profiles stay policy-free

`EnglishReading { Auto, Native, Narrated }` lives in the pipeline crate next to
`CodeBlockMode` and is applied to the shared pipeline through
`set_english_reading`, from the same startup call and the same post-save hook
that apply `code_block_mode`. `process_for_engine` resolves it once per call:

| setting | engine reads Latin natively | effective prose route |
|---------|-----------------------------|-----------------------|
| `auto` | yes | `Native` |
| `auto` | no | `Narrate` |
| `native` | yes | `Native` |
| `native` | no | `Narrate` (fallback) |
| `narrated` | any | `Narrate` |

Resolution happens inside the pipeline lock, so the text, its spans and the
generation snapshot of one run cannot straddle two settings.

Rejected: passing a resolved `Route` into `process_for_engine`. Every caller
(`run_normalization`, `preview_normalization`, the golden harness, the probe
tests) would have to know the setting, and the dictionary editor's
`builtin_contains_for` — which also needs the resolved route — has no such
argument. The pipeline already owns runtime text policy; the setting is one
more value of it.

Rejected: resolving in the commands layer before the lock. A `update_config`
racing a synthesis could then hand the pipeline half a configuration.

### D3. The identifier route survives a narrated prose setting

With `narrated` on TeraTTS, phase 15 leaves identifiers verbatim and records
their spans, and phase 16 (English) would then transliterate the same Latin
text — the old profile invariant `identifier: Native ⇒ prose: Native` existed
precisely to prevent that. The pipeline now protects the native identifier
matches on the phase-16 snapshot: identifier-shaped tokens (`getUserData`,
`snake_case`, `kebab-case`) are excluded from the English match set, while
plain prose words and letter-digit tokens are transliterated as today.

Rejected: making the identifier route follow the prose route. It would honour
the setting but silently overrule the profile on fragments the user did not ask
about, and it would make `narrated` on TeraTTS differ from `narrated` on Piper
for identifiers — the opposite of engine independence.

Rejected: replacing identifiers with placeholders before phase 16 and
splicing them back after mapping. It works, but `CharMapping` would collapse a
whole identifier onto its source range, coarsening word highlighting for a
fragment that is verbatim in the output; the filter keeps the one-to-one
mapping.

### D4. The capability reaches the UI through `get_available_engines`

`EngineAvailability` gains `reads_latin_natively`, stamped from
`profiles::profile_for(kind)` in the probe (`tts/availability.rs`). Settings
reads the active engine's flag to decide whether the «Нативно» choice needs
the fallback hint.

Rejected: a new command. The probe already exists, is cheap, and is loaded by
the dialog on every open; a second command would add a second source of the
same fact.

Rejected: hard-coding engine names in the dialog. It would drift as soon as
another Latin-capable engine ships — the exact failure mode this change
removes from the pipeline.

### D5. «Нативно» stays selectable on incapable engines, with a hint

The setting is a listener preference and must survive an engine switch: picking
«Нативно» while on Silero, then switching to TeraTTS, should read English
natively. Disabling the option on an incapable engine would make the choice
engine-dependent again and would leave a user who enabled it on TeraTTS
looking at a disabled selected item.

Instead, when the engine has no native Latin reading and the choice is
«Нативно», the dialog renders a hint under the control: the next synthesis will
read English in Russian, and the stored value stays `native`. «Авто» needs no
hint — it means "follow the engine", and the engine's behaviour is the answer.

Rejected: silently rewriting the stored value to `narrated` when a
non-capable engine becomes active. A preference that changes itself is a
preference the user cannot trust.

## Risks / Trade-offs

- **A new mixed mode** (native identifiers, narrated prose) exists only on
  engines that route identifiers natively and is guarded by one filter plus
  unit tests; every other combination keeps today's code path.
- **`auto` is the default**, so an upgrade changes nothing; a user who never
  opens Settings keeps the behaviour of the engine they already run.
- **The hint can be missed.** A user who selects «Нативно» on Silero hears
  Russian prose; the hint is the only signal. This is the accepted cost of
  keeping the choice engine-independent.

## Migration Plan

No migration: `english_reading` defaults to `"auto"` and unknown values
canonicalize to it on load and on patch, exactly like `code_block_mode`. No
snapshot or history schema changes.
