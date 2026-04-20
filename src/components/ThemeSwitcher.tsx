import { Select, useMantineColorScheme } from '@mantine/core';

type Scheme = 'light' | 'dark' | 'auto';

export function ThemeSwitcher() {
  const { colorScheme, setColorScheme } = useMantineColorScheme();
  return (
    <Select
      value={colorScheme}
      onChange={(v) => v && setColorScheme(v as Scheme)}
      size="xs"
      w={110}
      allowDeselect={false}
      aria-label="Переключение темы"
      data={[
        { label: 'Авто', value: 'auto' },
        { label: 'Светлая', value: 'light' },
        { label: 'Тёмная', value: 'dark' },
      ]}
    />
  );
}
