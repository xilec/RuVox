## REMOVED Requirements

### Requirement: Viewer quick-add

**Reason**: The post-save regeneration offer is removed: closing the
dictionary editor no longer asks «Переозвучить сейчас?» and never issues a
regeneration command. The specification schema applies a MODIFIED requirement
as a whole-block replacement and refuses to drop scenarios the current spec
still has, so the requirement is removed here and re-added without the offer
scenarios in the paired change `viewer-quickadd-no-offer`.

**Migration**: Regenerate a stale recording from the entry's context menu
(«Перегенерировать аудио»), which keeps its existing preview and confirmation
semantics.
