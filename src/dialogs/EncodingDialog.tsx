import { useEffect, useRef, useState } from 'react';
import { Button, Group, Loader, Modal, Select, Stack, Text, Textarea } from '@mantine/core';
import { notifications } from '@mantine/notifications';
import { commands } from '../lib/tauri';
import type { ReadTextFileResult } from '../lib/tauri';
import { IMPORT_ENCODING_NAMES } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { useT } from '../lib/i18n';

/** Characters of the decoded document shown in the preview pane — enough to
 *  recognize mojibake without rendering megabytes into a modal. */
const PREVIEW_CHARS = 2000;

export interface EncodingDialogProps {
  opened: boolean;
  /** Absolute file path re-decoded whenever the user switches encodings. */
  path: string;
  /** Decode result produced by auto-detection («Файл с кодировкой…» step
   *  before the normalization preview — design D5). */
  initial: ReadTextFileResult;
  onConfirm: (result: ReadTextFileResult) => void;
  onCancel: () => void;
}

/**
 * Manual-encoding dialog: raw decoded preview + a Select preselected with
 * the detected encoding. Confirm hands the (possibly re-decoded) text back
 * so AppShell continues the normal import flow; cancelling aborts entirely.
 */
export function EncodingDialog({ opened, path, initial, onConfirm, onCancel }: EncodingDialogProps) {
  const tt = useT();
  const [result, setResult] = useState<ReadTextFileResult>(initial);
  const [loading, setLoading] = useState(false);
  // Sequence guard: a slow decode for an older Select choice must never
  // overwrite a newer one's preview.
  const requestSeq = useRef(0);

  // Re-arm on each opening / path change.
  useEffect(() => {
    if (opened) {
      requestSeq.current += 1;
      setResult(initial);
      setLoading(false);
    }
    // `initial` changes together with opened/path at the call site.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opened, path]);

  async function changeEncoding(label: string | null) {
    if (!label || label === result.encoding) return;
    const ticket = ++requestSeq.current;
    setLoading(true);
    try {
      const next = await commands.readTextFile(path, label);
      if (requestSeq.current !== ticket) return;
      setResult(next);
    } catch (err) {
      if (requestSeq.current !== ticket) return;
      notifications.show({
        title: tt('errors.title'),
        message: formatError(err),
        color: 'red',
      });
      // Keep the previous decoding visible; restore the working selection.
      setResult((r) => ({ ...r }));
    } finally {
      if (requestSeq.current === ticket) setLoading(false);
    }
  }

  function confirm() {
    onConfirm(result);
  }

  return (
    <Modal
      opened={opened}
      onClose={onCancel}
      title={tt('app.import.encoding.title')}
      centered
      size="lg"
    >
      <Stack gap="sm">
        <Text size="sm" c="dimmed">
          {tt('app.import.encoding.description')}
        </Text>
        <Select
          label={tt('app.import.encoding.label')}
          data={[...IMPORT_ENCODING_NAMES]}
          value={result.encoding}
          onChange={(v) => void changeEncoding(v)}
          allowDeselect={false}
          rightSection={loading ? <Loader size="xs" /> : undefined}
        />
        <Textarea
          aria-label={tt('app.import.encoding.preview_aria')}
          value={result.text.slice(0, PREVIEW_CHARS)}
          readOnly
          autosize
          minRows={8}
          maxRows={16}
          styles={{ input: { fontFamily: 'monospace', fontSize: 'var(--mantine-font-size-sm)' } }}
        />
        <Group justify="flex-end">
          <Button variant="default" onClick={onCancel}>
            {tt('common.cancel')}
          </Button>
          <Button onClick={confirm} loading={loading}>
            {tt('app.import.encoding.confirm')}
          </Button>
        </Group>
      </Stack>
    </Modal>
  );
}
