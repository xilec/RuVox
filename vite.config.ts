import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(() => ({
  plugins: [react()],

  // react-rnd's Draggable reads `process.env.DRAGGABLE_DEBUG` at render
  // time (its `log()` helper). The webview has no Node `process` global, so
  // the bare read throws ReferenceError and React unmounts the whole tree —
  // the app window goes blank the moment the preview dialog opens. Replace
  // the reference at bundle time: top-level `define` covers the production
  // Rollup build, `optimizeDeps.esbuildOptions.define` covers the dev
  // prebundle (esbuild does not apply top-level define to optimized deps).
  define: {
    "process.env.DRAGGABLE_DEBUG": "undefined",
  },
  optimizeDeps: {
    esbuildOptions: {
      define: {
        "process.env.DRAGGABLE_DEBUG": "undefined",
      },
    },
  },

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // tmp/ is the repo-local scratch dir (VM disks, xwin caches with
      // symlink loops, etc.) — watching it once crashed the dev server
      // with ELOOP on a recursive symlink. src-tauri/ is covered by
      // cargo's own watcher.
      ignored: ["**/src-tauri/**", "**/tmp/**"],
    },
  },
}));
