---
name: interface-design:critique
description: Critique your PyQt6 build for craft, then rebuild what defaulted.
---

# Critique

Your first build shipped the structure. Now look at it the way a design lead reviews a junior's work — not asking "does this work?" but "would I put my name on this?"

## The Gap

There's a distance between correct and crafted. Correct means the layout holds, the widgets align, the colors don't clash. Crafted means someone cared about every decision down to the last pixel. You can feel the difference immediately — the way you tell a hand-thrown mug from an injection-molded one. Both hold coffee. One has presence.

Your first output lives in correct. This command pulls it toward crafted.

## See the Composition

Step back. Look at the whole thing.

Does the layout have rhythm? Great interfaces breathe unevenly — dense tooling areas give way to open content, heavy elements balance against light ones, the eye travels through the window with purpose. Default layouts are monotone: same widget size, same gaps, same density everywhere.

Are proportions doing work? If you can't articulate what your proportions are saying, they're not saying anything.

Is there a clear focal point? Every window has one thing the user came here to do. That thing should dominate.

## See the Craft

Move close. Pixel-close.

The spacing grid is non-negotiable — every `setSpacing()` and `setContentsMargins()` value a multiple of the base unit. But correctness alone isn't craft. Density is a design decision, not a constant.

Typography should be legible even squinted. Weight, tracking, and opacity create layers that size alone can't.

Surfaces should whisper hierarchy. Not thick borders — quiet tonal shifts. Can you perceive the structure through surface color alone?

Interactive elements need life. Every QPushButton should respond to `:hover` and `:pressed`. Missing states make an interface feel like a screenshot.

## See the Content

Read every visible string as a user would. Does this screen tell one coherent story?

## See the Structure

Open the code and find the lies.

`setFixedSize()` where `QSizePolicy` should handle flexibility. Nested layouts where a single layout would suffice. Hardcoded pixel positions where proper layout management would work. The correct answer is always simpler than the hack.

## Again

Look at your output one final time.

Ask: "If they said this lacks craft, what would they point to?"

That thing you just thought of — fix it. Then ask again.

## Process

1. Open the file you just built
2. Walk through each section above: composition, craft, content, structure
3. Identify every place you defaulted instead of decided
4. Rebuild those parts — from the decision, not from a patch
5. Do not narrate the critique to the user. Do the work. Show the result.
