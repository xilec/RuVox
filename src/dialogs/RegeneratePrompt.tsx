import { Button, Group, Modal, Stack, Text } from '@mantine/core';
import { useT } from '../lib/i18n';

interface Props {
  opened: boolean;
  /** Display title of the entry the offer will re-synthesize (its stored
   *  title, or the derived first line for legacy entries). */
  entryTitle: string;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Post-save offer of the viewer quick-add (user-dictionary spec, "Viewer
 * quick-add"): the entry's recording was synthesized with the old dictionary,
 * so the app offers to re-synthesize it right away. Confirming runs the
 * existing regeneration path; declining changes nothing and the previous
 * audio stays playable.
 *
 * Deliberately a bespoke Modal instead of `modals.openConfirmModal`: the offer
 * is raised from `DictionaryModal`'s close handler, in the same commit that
 * may restore the Settings modal underneath, while the editor is still running
 * its exit transition. A global confirm modal is owned by `ModalsProvider`,
 * cannot be sequenced after the editor's close, and lands below the editor's
 * top-level z-index (400) if it appears while that transition is still on
 * screen; the local `opened` state lets AppShell raise the offer exactly once,
 * after the editor is gone. Keep it bespoke unless the dictionary editor
 * stops using its own top-level z-index.
 */
export function RegeneratePrompt({
  opened,
  entryTitle,
  onConfirm,
  onCancel,
}: Props) {
  const tt = useT();
  return (
    <Modal
      opened={opened}
      onClose={onCancel}
      title={tt('dictionary.quickadd.regen.title')}
      size="md"
      withinPortal
    >
      <Stack gap="md">
        <Text size="sm">
          {tt('dictionary.quickadd.regen.message', [entryTitle])}
        </Text>
        <Group justify="flex-end">
          <Button variant="default" onClick={onCancel}>
            {tt('dictionary.quickadd.regen.cancel')}
          </Button>
          <Button onClick={onConfirm}>
            {tt('dictionary.quickadd.regen.confirm')}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
