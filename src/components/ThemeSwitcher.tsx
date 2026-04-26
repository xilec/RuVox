import { NativeSelect, useMantineColorScheme } from '@mantine/core';

type Scheme = 'light' | 'dark' | 'auto';

export function ThemeSwitcher() {
  const { colorScheme, setColorScheme } = useMantineColorScheme();
  return (
    <NativeSelect
      value={colorScheme}
      onChange={(e) => setColorScheme(e.currentTarget.value as Scheme)}
      size="xs"
      w={120}
      aria-label="Переключение темы"
      data={[
        { label: 'Авто', value: 'auto' },
        { label: 'Светлая', value: 'light' },
        { label: 'Тёмная', value: 'dark' },
      ]}
    />
  );
}
