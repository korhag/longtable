import tailwindcss from '@tailwindcss/vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

declare const process: { env: { LONGTABLE_API?: string } };

const api = process.env.LONGTABLE_API ?? 'http://127.0.0.1:8080';

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  server: {
    fs: { allow: ['..'] },
    port: 5173,
    proxy: {
      '/api': api,
      '/packs': api,
      '/ws': { target: api.replace(/^http/, 'ws'), ws: true },
    },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
