/** @type {import('@sveltejs/vite-plugin-svelte').SvelteConfig} */
const config = {
  // Enable runes mode only for our source files, not node_modules
  // lucide-svelte uses legacy $$props syntax incompatible with forced runes mode
  compilerOptions: ({ filename }) => {
    if (filename && filename.includes('node_modules')) {
      return {};
    }
    return { runes: true };
  },
};

export default config;
