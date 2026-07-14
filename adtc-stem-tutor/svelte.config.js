import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://svelte.dev/docs/kit/integrations#preprocessors
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		// adapter-static configures SvelteKit to build a zero-dependency SPA
		adapter: adapter({
			pages: 'build',
			assets: 'build',
			fallback: 'index.html', // Essential fallback for client-side routing
			precompress: false,
			strict: true
		})
	}
};

export default config;