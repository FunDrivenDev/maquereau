import { svelte } from "@sveltejs/vite-plugin-svelte";
import process from "node:process";
import { defineConfig } from "vite";

// In development Tauri loads the dev server from the port scripts/dev.sh picks and passes as MAQUEREAU_PORT, free
// so that other projects' dev servers can run alongside; in release, it loads `dist/`.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: Number(process.env.MAQUEREAU_PORT ?? 5174), strictPort: true, host: "127.0.0.1" },
  build: { target: "safari16", outDir: "dist", emptyOutDir: true, sourcemap: false },
});
