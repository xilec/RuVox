import { SegmentedControl, useMantineColorScheme } from '@mantine/core';

type Scheme = 'light' | 'dark' | 'auto';

export function ThemeSwitcher() {
  const { colorScheme, setColorScheme } = useMantineColorScheme();
  return (
    <SegmentedControl
      value={colorScheme}
      onChange={(v) => setColorScheme(v as Scheme)}
      size="xs"
      data={[
        { label: 'Светлая', value: 'light' },
        { label: 'Тёмная', value: 'dark' },
        { label: 'Авто', value: 'auto' },
      ]}
      aria-label="Переключение темы"
    />
  );
}
