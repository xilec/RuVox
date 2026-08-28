import { useEffect, useState } from 'react';
import { Button, Modal, Stack, TextInput } from '@mantine/core';
import { useT } from '../lib/i18n';

const URL_RE = /^https?:\/\/\S+$/i;

export interface UrlImportDialogProps {
  opened: boolean;
  /** Receives the trimmed, scheme-validated URL (fetch itself runs in
   *  AppShell so errors surface through the shared notification path). */
  onConfirm: (url: string) => void;
  onClose: () => void;
}

/** «По ссылке…» modal: validates the http(s) shape client-side; backend
 *  fetch_url_text enforces the real policy. Confirm is disabled until the
 *  input parses as an absolute http(s) URL. */
export function UrlImportDialog({ opened, onConfirm, onClose }: UrlImportDialogProps) {
  const tt = useT();
  const [url, setUrl] = useState('');
  const [touched, setTouched] = useState(false);

  useEffect(() => {
    if (opened) {
      setUrl('');
      setTouched(false);
    }
  }, [opened]);

  const trimmed = url.trim();
  const valid = URL_RE.test(trimmed);

  function submit() {
    if (!valid) {
      setTouched(true);
      return;
    }
    onConfirm(trimmed);
  }

  return (
    <Modal opened={opened} onClose={onClose} title={tt('app.import.url.title')} centered>
      <form onSubmit={(e) => { e.preventDefault(); submit(); }}>
        <Stack gap="sm">
          <TextInput
            label={tt('app.import.url.label')}
            placeholder={tt('app.import.url.placeholder')}
            value={url}
            data-autofocus
            onChange={(e) => setUrl(e.currentTarget.value)}
            error={touched && !valid ? 'https://…' : undefined}
            aria-invalid={touched && !valid}
          />
          <Button type="submit" disabled={!valid}>
            {tt('app.import.url.confirm')}
          </Button>
        </Stack>
      </form>
    </Modal>
  );
}
