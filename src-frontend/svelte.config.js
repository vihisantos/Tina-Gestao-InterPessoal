import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    // A janela Tauri carrega arquivos estáticos; fallback cobre rotas client-side.
    adapter: adapter({ fallback: '200.html' }),
    paths: { relative: true }
  }
};

export default config;
