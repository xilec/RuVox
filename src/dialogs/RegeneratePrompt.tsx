import { Button, Group, Modal, Stack, Text } from '@mantine/core';
import { useT } from '../lib/i18n';

interface Props {
  opened: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/**
 * Post-save offer of the viewer quick-add (user-dictionary spec, "Viewer
 * quick-add"): the entry's recording was synthesized with the old dictionary,
 * so the app offers to re-synthesize it right away. Confirming runs the
 * existing regeneration path; declining changes nothing and the previous
 * audio stays playable.
 */
export function RegeneratePrompt({ opened, onConfirm, onCancel }: Props) {
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
        <Text size="sm">{tt('dictionary.quickadd.regen.message')}</Text>
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
