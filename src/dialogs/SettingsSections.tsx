// Section bodies of the Settings dialog. The orchestrator (Settings.tsx) owns
// the form state, the config loading and the submit path; this module only
// renders the controls of one section and keeps each row in the search index
// (`data-settings-field` marks it for the reachability test).

import { Fragment, type ReactNode } from 'react';
import {
  Alert,
  Badge,
  Button,
  Divider,
  Group,
  NumberInput,
  SegmentedControl,
  Select,
  Stack,
  Switch,
  Text,
  Tooltip,
} from '@mantine/core';
import type { UseFormReturnType } from '@mantine/form';
import type { MessageKey } from '../i18n/ru';
import type { AvailabilityMap } from '../lib/engineSelection';
import { useT } from '../lib/i18n';
import type {
  SettingsFormValues,
  SettingsRowId,
  SettingsSectionId,
} from '../lib/settingsSections';
import { rowMatchesQuery, splitHighlight } from '../lib/settingsSections';
import type { EngineKind } from '../lib/tauri';
import { formatError } from '../lib/errors';
import { ReadingScaleControl, ReadingWidthControl } from '../components/ReadingControls';
import classes from './Settings.module.css';

interface SelectOption {
  value: string;
  label: string;
}

export interface SettingsSectionsProps {
  section: SettingsSectionId;
  query: string;
  form: UseFormReturnType<SettingsFormValues>;
  availability: AvailabilityMap;
  availabilityProbed: boolean;
  coercedAlert: boolean;
  engineOptions: SelectOption[];
  piperVoiceOptions: SelectOption[];
  piperVoiceRecommended: boolean;
  speakerOptions: SelectOption[];
  sampleRateOptions: SelectOption[];
  terattsVoiceOptions: SelectOption[];
  cacheDir: string;
  logDir: string;
  appVersion: string;
  updaterEnabled: boolean;
  dictionaryCount: number | null;
  onEngineChange: (next: EngineKind) => void;
  onSampleRateChange: (next: number) => void;
  onOpenModels: () => void;
  onOpenCacheDir: () => void;
  onOpenLogDir: () => void;
  onOpenCleanup: () => void;
  onOpenDictionary: () => void;
  onCheckUpdates: () => void;
  onLanguageChange: (next: string) => void;
}

/** Matched substring highlight; the row always matches, the label may not. */
function HighlightedLabel({ text, query }: { text: string; query: string }) {
  const parts = splitHighlight(text, query);
  if (!parts.some((part) => part.match)) return <>{text}</>;
  return (
    <>
      {parts.map((part, index) =>
        part.match ? (
          <mark key={index} className={classes.highlight}>
            {part.text}
          </mark>
        ) : (
          <Fragment key={index}>{part.text}</Fragment>
        ),
      )}
    </>
  );
}

/** Filters a row by the query; the tests use the same `data-settings-field`. */
function SettingsRow({
  field,
  query,
  children,
}: {
  field: SettingsRowId;
  query: string;
  children: ReactNode;
}) {
  if (!rowMatchesQuery(field, query)) return null;
  return (
    <div className={classes.row} data-settings-field={field}>
      {children}
    </div>
  );
}

/** Renders the controls of one section; callers render exactly one at a time. */
export function SettingsSections(props: SettingsSectionsProps) {
  const tt = useT();
  const { form, query } = props;
  const label = (key: MessageKey) => <HighlightedLabel text={tt(key)} query={query} />;

  switch (props.section) {
    case 'engine':
      return (
        <Stack gap="md">
          <SettingsRow field="models" query={query}>
            <Group justify="space-between" wrap="nowrap" gap="sm">
              <div>
                <Text size="sm" fw={500}>
                  {tt('settings.models.title')}
                </Text>
                <Text size="xs" c="dimmed">
                  {tt('settings.models.description')}
                </Text>
              </div>
              <Button variant="default" size="xs" onClick={props.onOpenModels}>
                {tt('settings.models.open')}
              </Button>
            </Group>
          </SettingsRow>

          <SettingsRow field="engine" query={query}>
            {props.coercedAlert && (
              <Alert color="yellow" variant="light" mb="sm">
                {tt('settings.coerced_alert')}
              </Alert>
            )}
            <Select
              label={label('settings.engine.label')}
              description={tt('settings.engine.description')}
              data={props.engineOptions}
              value={form.values.engine}
              onChange={(value) => value && props.onEngineChange(value as EngineKind)}
            />
            {!props.availability.silero.available && props.availability.silero.reason && (
              <Text size="xs" c="dimmed" mt={-8}>
                {tt('settings.engine.reason_silero', [
                  formatError(props.availability.silero.reason),
                ])}
              </Text>
            )}
            {!props.availability.silero_native.available &&
              props.availability.silero_native.reason && (
                <Text size="xs" c="dimmed" mt={-8}>
                  {tt('settings.engine.reason_silero_native', [
                    formatError(props.availability.silero_native.reason),
                  ])}
                </Text>
              )}
            {!props.availability.teratts.available && props.availability.teratts.reason && (
              <Text size="xs" c="dimmed" mt={-8}>
                {tt('settings.engine.reason_teratts', [
                  formatError(props.availability.teratts.reason),
                ])}
              </Text>
            )}
          </SettingsRow>

          <SettingsRow field="voice" query={query}>
            {form.values.engine === 'teratts' ? (
              <Select
                label={label('settings.teratts_voice.label')}
                description={tt('settings.teratts_voice.description')}
                data={props.terattsVoiceOptions}
                key={form.key('teratts_voice')}
                {...form.getInputProps('teratts_voice')}
              />
            ) : form.values.engine === 'piper' ? (
              <Select
                label={label('settings.piper_voice.label')}
                description={tt('settings.piper_voice.description')}
                data={props.piperVoiceOptions}
                key={form.key('piper_voice')}
                {...form.getInputProps('piper_voice')}
                rightSection={
                  props.piperVoiceRecommended ? (
                    <Tooltip label={tt('settings.piper_voice.recommended_tooltip')}>
                      <Badge size="xs" color="blue" variant="light">
                        {tt('settings.piper_voice.recommended_badge')}
                      </Badge>
                    </Tooltip>
                  ) : null
                }
                rightSectionWidth={60}
              />
            ) : (
              <Select
                label={label('settings.speaker.label')}
                data={props.speakerOptions}
                key={form.key('speaker')}
                {...form.getInputProps('speaker')}
              />
            )}
          </SettingsRow>

          <SettingsRow field="sample_rate" query={query}>
            <Select
              label={label('settings.sample_rate.label')}
              data={props.sampleRateOptions}
              value={String(form.values.sample_rate)}
              onChange={(value) =>
                props.onSampleRateChange(value ? parseInt(value, 10) : 24000)
              }
              error={form.errors.sample_rate}
            />
          </SettingsRow>
        </Stack>
      );

    case 'reading':
      return (
        <Stack gap="md">
          <SettingsRow field="code_block_mode" query={query}>
            <Stack gap={4}>
              <Text size="sm">{label('settings.code_block.label')}</Text>
              <SegmentedControl
                name="code_block_mode"
                value={form.values.code_block_mode}
                onChange={(value) => form.setFieldValue('code_block_mode', value)}
                data={[
                  { label: tt('settings.code_block.brief'), value: 'brief' },
                  { label: tt('settings.code_block.read'), value: 'read' },
                ]}
              />
              <Text size="xs" c="dimmed">
                {tt('settings.code_block.description')}
              </Text>
            </Stack>
          </SettingsRow>

          <SettingsRow field="english_reading" query={query}>
            <Stack gap={4}>
              <Text size="sm">{label('settings.english_reading.label')}</Text>
              <SegmentedControl
                name="english_reading"
                value={form.values.english_reading}
                onChange={(value) => form.setFieldValue('english_reading', value)}
                data={[
                  { label: tt('settings.english_reading.auto'), value: 'auto' },
                  { label: tt('settings.english_reading.native'), value: 'native' },
                  { label: tt('settings.english_reading.narrated'), value: 'narrated' },
                ]}
              />
              <Text size="xs" c="dimmed">
                {tt('settings.english_reading.description')}
              </Text>
              {props.availabilityProbed &&
                form.values.english_reading === 'native' &&
                !props.availability[form.values.engine].reads_latin_natively && (
                  <Text size="xs" c="orange.7">
                    {tt('settings.english_reading.fallback_hint')}
                  </Text>
                )}
            </Stack>
          </SettingsRow>

          <SettingsRow field="preview_dialog_enabled" query={query}>
            <Switch
              label={label('settings.preview_dialog_enabled')}
              key={form.key('preview_dialog_enabled')}
              {...form.getInputProps('preview_dialog_enabled', { type: 'checkbox' })}
            />
          </SettingsRow>

          {!query.trim() && <Divider />}

          <SettingsRow field="reading_width" query={query}>
            <Stack gap={4}>
              <Text size="sm">{label('viewer.reading_width.label')}</Text>
              <ReadingWidthControl
                name="settings_reading_width"
                value={form.values.reading_width}
                onChange={(value) => form.setFieldValue('reading_width', value)}
                size="sm"
              />
              <Text size="xs" c="dimmed">
                {tt('settings.reading.width.description')}
              </Text>
            </Stack>
          </SettingsRow>

          <SettingsRow field="reading_scale" query={query}>
            <Stack gap={4}>
              <Text size="sm">{label('viewer.reading_scale.label')}</Text>
              <ReadingScaleControl
                value={form.values.reading_scale}
                onChange={(value) => form.setFieldValue('reading_scale', value)}
                size="compact-sm"
              />
              <Text size="xs" c="dimmed">
                {tt('settings.reading.scale.description')}
              </Text>
            </Stack>
          </SettingsRow>
        </Stack>
      );

    case 'notifications':
      return (
        <Stack gap="md">
          <SettingsRow field="notify_on_ready" query={query}>
            <Switch
              label={label('settings.notify_on_ready')}
              key={form.key('notify_on_ready')}
              {...form.getInputProps('notify_on_ready', { type: 'checkbox' })}
            />
          </SettingsRow>

          <SettingsRow field="notify_on_error" query={query}>
            <Switch
              label={label('settings.notify_on_error')}
              key={form.key('notify_on_error')}
              {...form.getInputProps('notify_on_error', { type: 'checkbox' })}
            />
          </SettingsRow>
        </Stack>
      );

    case 'storage':
      return (
        <Stack gap="md">
          <SettingsRow field="max_cache_size_mb" query={query}>
            <NumberInput
              label={label('settings.max_cache_size.label')}
              description={tt('settings.max_cache_size.description')}
              min={100}
              key={form.key('max_cache_size_mb')}
              {...form.getInputProps('max_cache_size_mb')}
            />
          </SettingsRow>

          {props.cacheDir && (
            <SettingsRow field="data_folder" query={query}>
              <Stack gap={4}>
                <Text size="xs" c="dimmed">
                  {tt('settings.data_folder')}
                </Text>
                <Text size="sm" className={classes.path}>
                  {props.cacheDir}
                </Text>
                <Group>
                  <Button variant="default" size="xs" onClick={props.onOpenCacheDir}>
                    {tt('settings.open_folder')}
                  </Button>
                </Group>
              </Stack>
            </SettingsRow>
          )}

          <SettingsRow field="cleanup" query={query}>
            <Group>
              <Button variant="default" onClick={props.onOpenCleanup}>
                {tt('settings.cleanup.button')}
              </Button>
            </Group>
          </SettingsRow>
        </Stack>
      );

    case 'interface':
      return (
        <Stack gap="md">
          <SettingsRow field="hide_to_tray_on_close" query={query}>
            <Switch
              label={label('settings.hide_to_tray_on_close')}
              description={tt('settings.hide_to_tray_on_close.description')}
              key={form.key('hide_to_tray_on_close')}
              {...form.getInputProps('hide_to_tray_on_close', { type: 'checkbox' })}
            />
          </SettingsRow>

          <SettingsRow field="theme" query={query}>
            <Select
              label={label('settings.theme.label')}
              data={[
                { value: 'light', label: tt('settings.theme.light') },
                { value: 'dark', label: tt('settings.theme.dark') },
                { value: 'auto', label: tt('settings.theme.auto') },
              ]}
              key={form.key('theme')}
              {...form.getInputProps('theme')}
            />
          </SettingsRow>

          <SettingsRow field="language" query={query}>
            <Select
              label={label('settings.language.label')}
              data={[
                { value: 'ru', label: 'Русский' },
                { value: 'en', label: 'English' },
              ]}
              key={form.key('language')}
              value={form.values.language}
              onChange={(value) => value && props.onLanguageChange(value)}
            />
          </SettingsRow>
        </Stack>
      );

    case 'dictionary':
      return (
        <Stack gap="md">
          <SettingsRow field="dictionary_entries" query={query}>
            <Text size="sm" c="dimmed">
              {tt('settings.dictionary.entries_count', [props.dictionaryCount ?? '—'])}
            </Text>
          </SettingsRow>

          <SettingsRow field="dictionary_open" query={query}>
            <Group>
              <Button variant="default" onClick={props.onOpenDictionary}>
                {tt('settings.dictionary.open')}
              </Button>
            </Group>
          </SettingsRow>
        </Stack>
      );

    case 'about':
      return (
        <Stack gap="md">
          <SettingsRow field="version" query={query}>
            <Text size="sm">{tt('settings.version', [props.appVersion || '—'])}</Text>
          </SettingsRow>

          {props.updaterEnabled && (
            <SettingsRow field="updates" query={query}>
              <Group>
                <Button variant="default" onClick={props.onCheckUpdates}>
                  {tt('settings.check_updates')}
                </Button>
              </Group>
            </SettingsRow>
          )}

          {props.logDir && (
            <SettingsRow field="logs_folder" query={query}>
              <Stack gap={4}>
                <Text size="xs" c="dimmed">
                  {tt('settings.logs_folder')}
                </Text>
                <Text size="sm" className={classes.path}>
                  {props.logDir}
                </Text>
                <Group>
                  <Button variant="default" size="xs" onClick={props.onOpenLogDir}>
                    {tt('settings.open_folder')}
                  </Button>
                </Group>
              </Stack>
            </SettingsRow>
          )}
        </Stack>
      );
  }
}
