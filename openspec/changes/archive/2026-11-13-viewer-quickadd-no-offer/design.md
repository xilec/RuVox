## Context

The paired change `drop-quickadd-regen-prompt` deletes the post-save
regeneration offer and its plumbing. Its delta removes the "Viewer quick-add"
requirement because OpenSpec 1.6 applies a MODIFIED requirement as a full block
replacement and treats scenarios as append/modify-only: a change that both
removes and re-adds the requirement is rejected as ambiguous. The requirement
must therefore return in a second change, archived right after the removal, on
the same branch.

## Goals / Non-Goals

**Goals**

- The main spec describes the viewer quick-add exactly as it behaves after the
  paired change: the menu action, the prefill, the immediate save and a quiet
  close, with no offer scenarios left.
- The removal and the addition archive in order on one branch, so no merged
  state ever lacks the requirement.

**Non-Goals**

- Changing any behavior or code.
- Restating the offer in another form.

## Decisions

**Add the requirement back verbatim apart from the offer.** The retained
scenarios keep their original names and bodies, so the archive history shows a
requirement that lost only its offer flows. One new scenario, "Closing after a
save offers nothing", pins the behavior that replaced them.

**Archive after the removal change.** `openspec archive` applies deltas in file
order the operator chooses; the removal must land first or the addition
collides with the existing requirement. Tasks in both changes name the order.

## Risks / Trade-offs

- [A reader browsing one change alone sees half the story] → each proposal
  names the paired change and explains the split.
- [Archiving out of order fails] → the removal change's tasks pin the archive
  order; the failure is loud (ADDED collides with an existing requirement).

## Migration Plan

None: the spec ends in the state a single MODIFIED change would have produced.

## Open Questions

None.
