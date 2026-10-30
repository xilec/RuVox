## Context

`UserDictionary` is a flat map and the pipeline owns one instance. The engine
profile already decides which built-in tables run, but the dictionary is
engine-agnostic and the editor's override badges are computed from a fixed
built-in set.

## Goals / Non-Goals

**Goals:**
- Store more than one dictionary collection, each named and scoped to global
  or one engine.
- Apply global entries everywhere and engine entries only for that engine,
  with engine-bound entries winning on the same `from`.
- Keep the user dictionary above built-in tables for every profile.
- Make the editor's scope explicit and migrate the existing file without
  changing behavior.

**Non-Goals:**
- Dictionary content changes or auto-generated dictionaries.
- Per-entry scope; an entry belongs to exactly one collection.
- A full dictionary-collection manager (create/rename/delete arbitrary
  collections); the UI exposes the global and engine scopes, and the storage
  format can carry named collections without a schema change.

## Decisions

1. **Scopes are global or an engine kind.** The fixed set matches the engines
   the app ships and keeps the merge rule unambiguous: global first, engine
   second, engine wins on the same primary key. A named collection carries a
   display name so the storage format is not tied to engine ids.
2. **The pipeline keeps a single merged map.** `DictionaryBook::merged_for`
   builds the active map at startup, after every dictionary change, and after
   an engine switch. The hot path stays one map lookup; no per-token scope
   branching.
3. **Version 2 is an array of collections.** `[[dictionaries]]` with `name`,
   `scope` and `entries` is forward-extensible; version 1's `[entries]` loads
   as one global collection named `Global`. Saving always writes version 2.
4. **Import/export operate on the selected collection.** Export writes the
   historical v1 single-dictionary shape so files stay portable; import reads
   that shape and applies it to the selected scope. Merge/replace semantics are
   unchanged inside a collection.
5. **Override markers use the scope's profile.** A global entry is marked
   against the active engine's profile; an engine entry against that engine's
   profile. This matches what the pipeline will actually do at synthesis time.
6. **Empty engine collections are omitted from disk.** Selecting an engine
   scope and saving an empty list removes its collection, so the file only
   carries meaningful data.

## Risks / Trade-offs

- Two collections can both list the same form; the merge order makes the
  engine collection's primary win, while an alias from the global collection
  may still resolve first. Save validation stays per collection; this matches
  the "engine wins on the same `from`" rule and avoids rejecting a global
  dictionary because an engine adds a colliding form.
- A v1 file is rewritten as v2 on the first save. An older build sees an
  unsupported version and loads nothing rather than corrupting the file, which
  is the existing forward-compatibility rule.
- The editor exposes fixed scopes, not arbitrary named collections; the
  storage and merge model already support names, so adding a collection
  manager later does not need another format change.
