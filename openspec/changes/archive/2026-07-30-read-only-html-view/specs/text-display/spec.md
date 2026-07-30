# Delta spec: text-display

## ADDED Requirements

### Requirement: Read-only rendered content

In every display mode (plain, markdown, HTML) the rendered content SHALL be
inert: clicking a link (`<a>`) inside the viewer SHALL NOT navigate the
webview or trigger any external handler — the click SHALL be intercepted and
its default action prevented. The link's original `href` (verbatim, as in the
source markup) SHALL be visible on hover (tooltip); resolving it against the
webview origin would yield a meaningless localhost URL for relative links.
Interactive elements in rendered content SHALL be
inoperable: buttons, selects, textareas and inputs SHALL be disabled;
`<video>`/`<audio>` SHALL render without media controls; `<details>` SHALL
not toggle. Neutralization SHALL NOT remove the elements from the rendered
output and SHALL NOT rely on sanitization changes that alter the stored
`html_source`.

#### Scenario: Click on a link does not navigate
- **WHEN** the user clicks a link inside the viewer in any display mode
- **THEN** the webview stays on the application and no navigation or external
  open occurs

#### Scenario: Link URL visible on hover
- **GIVEN** rendered content containing `<a href="/ru/users/maybe_elf/">`
- **WHEN** the user hovers the link
- **THEN** a tooltip shows the original href `/ru/users/maybe_elf/` verbatim

#### Scenario: Form controls are disabled
- **GIVEN** rendered content containing a `<button>` and a `<select>`
- **WHEN** the content is displayed
- **THEN** both controls are disabled and cannot be operated

#### Scenario: Media renders without controls
- **GIVEN** rendered content containing `<video controls src="...">`
- **WHEN** the content is displayed
- **THEN** the video element renders without playback controls

#### Scenario: Mermaid click-to-zoom still works
- **WHEN** the user clicks a rendered mermaid diagram
- **THEN** the zoom modal opens (link interception does not affect non-link
  clicks)
