import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath, URL } from 'url';

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
    },
  },
  // Exclude lucide-svelte from dep optimization — it ships Svelte 4 syntax ($$props)
  // which conflicts with the global runes:true compiler setting
  optimizeDeps: {
    exclude: ['lucide-svelte'],
  },
  // Tauri: prevent vite from obscuring Rust errors
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: true, // expose to Docker network (0.0.0.0)
    watch: {
      // Tauri expects a static serve on this port — ignore Rust changes
      ignored: ['**/src-tauri/**'],
      // Docker needs polling for file change detection
      usePolling: true,
    },
  },
});
