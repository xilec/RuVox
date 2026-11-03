// Shared reading-typography controls: the viewer toolbar (#354 / #357) and
// the Settings «Чтение» block use the same widgets, so the ladder, the bounds
// and the accessible names cannot drift between the two surfaces.

import { Button, Group, SegmentedControl, Text } from '@mantine/core';
import { useT } from '../lib/i18n';
import type { ReadingWidth } from '../lib/tauri';
import {
  canonicalReadingWidth,
  clampReadingScale,
  higherReadingScale,
  lowerReadingScale,
  READING_SCALE_DEFAULT,
  READING_SCALE_MAX,
  READING_SCALE_MIN,
} from '../lib/reading';

interface ReadingWidthControlProps {
  value: ReadingWidth;
  onChange: (next: ReadingWidth) => void;
  /** Radio group name, unique per surface (viewer toolbar vs Settings). */
  name: string;
  size?: 'xs' | 'sm';
}

export function ReadingWidthControl({
  value,
  onChange,
  name,
  size = 'xs',
}: ReadingWidthControlProps) {
  const tt = useT();
  return (
    <Group
      role="group"
      aria-label={tt('viewer.reading_width.label')}
      gap={0}
      wrap="nowrap"
    >
      <SegmentedControl
        name={name}
        size={size}
        value={value}
        onChange={(v) => onChange(canonicalReadingWidth(v))}
        data={[
          { label: tt('viewer.reading_width.narrow'), value: 'narrow' },
          { label: tt('viewer.reading_width.wide'), value: 'wide' },
        ]}
      />
    </Group>
  );
}

interface ReadingScaleControlProps {
  value: number;
  onChange: (next: number) => void;
  size?: 'compact-xs' | 'compact-sm';
}

export function ReadingScaleControl({
  value,
  onChange,
  size = 'compact-xs',
}: ReadingScaleControlProps) {
  const tt = useT();
  // Defensive: a stale prop (or a hand-edited config) must not render a value
  // the backend would reject.
  const scale = clampReadingScale(value);
  const atMin = scale <= READING_SCALE_MIN;
  const atMax = scale >= READING_SCALE_MAX;
  return (
    <Group
      role="group"
      aria-label={tt('viewer.reading_scale.label')}
      gap={4}
      wrap="nowrap"
    >
      <Button
        variant="default"
        size={size}
        px={8}
        disabled={atMin}
        aria-label={tt('viewer.reading_scale.decrease')}
        title={tt('viewer.reading_scale.decrease')}
        onClick={() => onChange(lowerReadingScale(scale))}
      >
        −
      </Button>
      <Text
        component="span"
        size={size === 'compact-sm' ? 'sm' : 'xs'}
        fw={500}
        miw={48}
        ta="center"
        aria-live="polite"
        aria-atomic="true"
      >
        {scale} %
      </Text>
      <Button
        variant="default"
        size={size}
        px={8}
        disabled={atMax}
        aria-label={tt('viewer.reading_scale.increase')}
        title={tt('viewer.reading_scale.increase')}
        onClick={() => onChange(higherReadingScale(scale))}
      >
        +
      </Button>
      <Button
        variant="subtle"
        size={size}
        px={6}
        disabled={scale === READING_SCALE_DEFAULT}
        aria-label={tt('viewer.reading_scale.reset')}
        title={tt('viewer.reading_scale.reset')}
        onClick={() => onChange(READING_SCALE_DEFAULT)}
      >
        ↺
      </Button>
    </Group>
  );
}
