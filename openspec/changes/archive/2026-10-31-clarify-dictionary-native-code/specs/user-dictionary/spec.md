# Delta: user-dictionary

## MODIFIED Requirements

### Requirement: Profile-gated built-in tables

Built-in pronunciation tables SHALL run only for fragments the active profile
narrates. A fragment routed `Native` SHALL NOT be rewritten by a built-in
table. The user dictionary SHALL still apply to a native fragment that the
pipeline renders as text — a Latin prose run or a code identifier — but SHALL
NOT apply to a fenced code block routed `Native`: the block reaches the engine
verbatim. The editor's `overridesBuiltin` marker SHALL reflect the profile that
actually runs the tables for the selected scope, so a table that is gated off
is not marked as overridden.

#### Scenario: Native route skips built-ins

- GIVEN a profile that routes Latin prose `Native`
- WHEN "Kubernetes" is normalized
- THEN the built-in English table does not rewrite it
- AND a matching user-dictionary entry still does

#### Scenario: Native code block bypasses the dictionary

- GIVEN a profile that routes fenced code blocks `Native`, the read mode and
  a global entry `let → лет`
- WHEN the block `let x = 42;` is normalized
- THEN the block stays `let x = 42;` and the dictionary does not rewrite it

#### Scenario: Override marker follows the profile

- GIVEN a native-Latin profile where built-in English tables do not run
- WHEN a global entry shadows a built-in word
- THEN the editor does not mark it as overriding a built-in table
