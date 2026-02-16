# Critique

Your first build shipped the structure. Now look at it the way a design lead reviews a junior's work — not asking "does this work?" but "would I put my name on this?"

---

## The Gap

There's a distance between correct and crafted. Correct means the layout holds, the widgets align, the colors don't clash. Crafted means someone cared about every decision down to the last pixel. You can feel the difference immediately — the way you tell a hand-thrown mug from an injection-molded one. Both hold coffee. One has presence.

Your first output lives in correct. This command pulls it toward crafted.

---

## See the Composition

Step back. Look at the whole thing.

Does the layout have rhythm? Great interfaces breathe unevenly — dense tooling areas give way to open content, heavy elements balance against light ones, the eye travels through the window with purpose. Default layouts are monotone: same widget size, same gaps, same density everywhere. Flatness is the sound of no one deciding.

Are proportions doing work? A 250px sidebar next to full-width content says "navigation serves content." A 350px sidebar says "these are peers." The specific number declares what matters. If you can't articulate what your proportions are saying, they're not saying anything.

Is there a clear focal point? Every window has one thing the user came here to do. That thing should dominate — through size, position, contrast, or the space around it. When everything competes equally, nothing wins.

---

## See the Craft

Move close. Pixel-close.

The spacing grid is non-negotiable — every value a multiple of the base unit, no exceptions — but correctness alone isn't craft. Craft is knowing that a tool panel at `setContentsMargins(8,8,8,8)` feels workbench-tight while the same panel at `setContentsMargins(24,24,24,24)` feels like a brochure. Density is a design decision, not a constant.

Typography should be legible even squinted. If size is the only thing separating your headline from your body from your label, the hierarchy is too weak. Weight, tracking, and opacity create layers that size alone can't.

Surfaces should whisper hierarchy. Not thick borders, not dramatic shadows — quiet tonal shifts where you feel the depth without seeing it. Remove every border from your QSS mentally. Can you still perceive the structure through surface color alone? If not, your surfaces aren't working hard enough.

Interactive elements need life. Every QPushButton, QToolButton, and clickable widget should respond to hover and press. Missing states make an interface feel like a screenshot of software instead of software.

---

## See the Content

Read every visible string as a user would. Not checking for typos — checking for truth.

Does this screen tell one coherent story? Or does the window title belong to one product, the text content to another, and the status bar to a third?

Content incoherence breaks the illusion faster than any visual flaw.

---

## See the Structure

Open the code and find the lies — the places that look right but are held together with tape.

Fixed pixel sizes where `QSizePolicy` should handle flexibility. Nested layouts where a single layout would suffice. `setFixedSize()` where `setMinimumSize()` with proper stretch factors would work. Each is a shortcut where a clean solution exists.

---

## Again

Look at your output one final time.

Ask: "If they said this lacks craft, what would they point to?"

That thing you just thought of — fix it. Then ask again.

The first build was the draft. The critique is the design.

## Process

1. Open the file you just built
2. Walk through each section above: composition, craft, content, structure
3. Identify every place you defaulted instead of decided
4. Rebuild those parts — from the decision, not from a patch
5. Do not narrate the critique to the user. Do the work. Show the result.
