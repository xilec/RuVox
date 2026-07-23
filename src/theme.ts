import { createTheme } from '@mantine/core';

// Anchor point for Mantine token overrides. Defaults are kept unless the app
// needs a deviation — app-level one-off tokens live as --ruvox-* custom
// properties in globals.css. See docs/ui.md ("Design tokens").
export const theme = createTheme({
  primaryColor: 'blue',
});
