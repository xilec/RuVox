import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';

export default tseslint.config(
  // tmp/ is the repo-designated scratch dir (spikes, benches, .venv's) —
  // none of it is app code to lint. Same for Python .venv's elsewhere
  // (e.g. silero-native/export) whose vendored torch .mjs files trip the
  // type-aware parser.
  { ignores: ['dist/', 'node_modules/', 'src-tauri/', 'silero-native/', 'ttsd/', 'scripts/', 'tmp/', '**/.venv/'] },
  js.configs.recommended,
  ...tseslint.configs.recommendedTypeChecked,
  reactHooks.configs.flat.recommended,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      // React/Tauri-idiomatic: async event handlers in JSX attributes, async
      // Mantine modal callbacks (onConfirm), and async listeners in the Tauri
      // event pub/sub are all intentional fire-and-forget.
      '@typescript-eslint/no-misused-promises': [
        'error',
        { checksVoidReturn: { attributes: false, properties: false, arguments: false } },
      ],
      // Data-fetch-on-mount and reset-state-on-dialog-open are deliberate
      // patterns in this codebase; the React Compiler-era alternatives (render
      //-time adjustment, key remounts) are churn with regression risk here.
      'react-hooks/set-state-in-effect': 'off',
    },
  },
  {
    files: ['**/*.test.ts', '**/*.test.tsx'],
    rules: {
      // vitest mocks are referenced unbound by design
      '@typescript-eslint/unbound-method': 'off',
    },
  },
  {
    files: ['**/*.js'],
    ...tseslint.configs.disableTypeChecked,
  },
);
