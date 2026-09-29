import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

// Tauri serves the dev server on a fixed port and loads `dist/` in release.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 5174, strictPort: true, host: "127.0.0.1" },
  build: { target: "safari16", outDir: "dist", emptyOutDir: true, sourcemap: false },
});
