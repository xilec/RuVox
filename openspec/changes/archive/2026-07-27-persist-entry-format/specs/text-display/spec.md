# Delta: text-display

## MODIFIED Requirements

### Requirement: Display mode switching

The `TextViewer` SHALL provide a `SegmentedControl` with the modes `Plain`,
`Markdown`, and `HTML`. Switching modes SHALL re-render the same original
text instantly and SHALL persist the choice on the entry via
`set_entry_format`. When an entry is selected, the viewer SHALL show the
entry's persisted `format`; entries with `format: null` SHALL fall back to
the viewer default mode (Markdown). The persisted choice
is display-only: it does not change the stored text, audio, or timestamps.

#### Scenario: Switch display mode
- GIVEN an entry is displayed in Markdown mode
- WHEN the user selects `Plain` in the `SegmentedControl`
- THEN the original text is re-rendered as-is and `set_entry_format` persists `"plain"` on the entry

#### Scenario: Selection restores the saved mode
- GIVEN an entry persisted with `format: "html"`
- WHEN the user selects that entry in the queue
- THEN the `SegmentedControl` shows `HTML` and the text renders in HTML mode

#### Scenario: Entry without a saved mode uses the default
- GIVEN an entry with `format: null` (e.g. written by an older build)
- WHEN the user selects that entry
- THEN the viewer renders it in the viewer default mode (Markdown)
