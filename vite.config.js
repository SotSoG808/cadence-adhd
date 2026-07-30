import { defineConfig } from 'vite'

export default defineConfig({
  // Source files live in src/
  root: 'src',
  build: {
    // Bundled output goes to dist/ which Tauri embeds into the MSI
    outDir: '../dist',
    emptyOutDir: true,
  },
  // Prevent Vite from clearing the screen during tauri dev
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
})
