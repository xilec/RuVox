import { useEffect, useState } from 'react';
import {
  Button,
  Checkbox,
  Grid,
  Group,
  Loader,
  Modal,
  ScrollArea,
  Text,
  Textarea,
} from '@mantine/core';
import { useForm } from '@mantine/form';
import classes from './PreviewDialog.module.css';
import { commands } from '../lib/tauri';

export interface PreviewDialogProps {
  /** Raw clipboard text to preview and optionally edit before synthesis. */
  text: string;
  opened: boolean;
  /**
   * Called when the user confirms synthesis.
   * `finalText` is either the original or the user-edited version.
   * `skipShortTexts` is true when the user checked the "skip for short texts" box.
   */
  onSynthesize: (finalText: string, skipShortTexts: boolean) => void;
  /** Called when the user cancels the dialog. */
  onCancel: () => void;
}

interface FormValues {
  editedText: string;
  skipShortTexts: boolean;
}

/**
 * Preview dialog (FF 1.1).
 *
 * Shows original text on the left and the pipeline-normalized version on the right.
 * The user can edit the original before confirming synthesis, or cancel entirely.
 */
export function PreviewDialog({
  text,
  opened,
  onSynthesize,
  onCancel,
}: PreviewDialogProps) {
  const [normalized, setNormalized] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [editMode, setEditMode] = useState(false);

  const form = useForm<FormValues>({
    initialValues: {
      editedText: text,
      skipShortTexts: false,
    },
  });

  // Re-populate form and fetch normalization whenever dialog opens with new text.
  useEffect(() => {
    if (!opened) return;

    setEditMode(false);
    setNormalized('');
    form.setValues({ editedText: text, skipShortTexts: false });

    setLoading(true);
    commands
      .previewNormalize(text)
      .then((result) => setNormalized(result.normalized))
      .catch(() => setNormalized('(ошибка нормализации)'))
      .finally(() => setLoading(false));
    // form dependency excluded intentionally: we only re-run when dialog opens with new text.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opened, text]);

  function handleSynthesize() {
    const finalText = editMode ? form.values.editedText.trim() : text;
    onSynthesize(finalText || text, form.values.skipShortTexts);
  }

  function handleEdit() {
    setEditMode(true);
  }

  return (
    <Modal
      opened={opened}
      onClose={onCancel}
      title="Предпросмотр нормализации"
      size="xl"
      closeOnClickOutside={false}
      closeOnEscape
    >
      <Grid gutter="md">
        <Grid.Col span={6}>
          <Text className={classes.paneLabel}>Оригинал</Text>
          {editMode ? (
            <Textarea
              classNames={{ input: classes.editTextarea }}
              minRows={12}
              maxRows={16}
              autosize
              {...form.getInputProps('editedText')}
            />
          ) : (
            <pre className={classes.textPane}>{text}</pre>
          )}
        </Grid.Col>

        <Grid.Col span={6}>
          <Text className={classes.paneLabel}>После нормализации</Text>
          {loading ? (
            <Group justify="center" pt="xl">
              <Loader size="sm" />
            </Group>
          ) : (
            <ScrollArea.Autosize mah={340}>
              <pre className={classes.textPane}>{normalized}</pre>
            </ScrollArea.Autosize>
          )}
        </Grid.Col>
      </Grid>

      <Group justify="space-between" mt="lg">
        <Checkbox
          label="Не показывать для коротких текстов"
          {...form.getInputProps('skipShortTexts', { type: 'checkbox' })}
        />

        <Group gap="sm">
          <Button variant="default" onClick={onCancel}>
            Отмена
          </Button>
          {!editMode && (
            <Button variant="outline" onClick={handleEdit}>
              Редактировать
            </Button>
          )}
          <Button onClick={handleSynthesize} disabled={loading}>
            Синтезировать
          </Button>
        </Group>
      </Group>
    </Modal>
  );
}
