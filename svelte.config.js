import staticAdapter from "@sveltejs/adapter-static";
import nodeAdapter from "@sveltejs/adapter-node";
import { vitePreprocess } from '@sveltejs/kit/vite';


const adapter = (process.env.TAURI) ?
    staticAdapter({ fallback: "index.html" }) : nodeAdapter();

/** @type {import('@sveltejs/kit').Config} */
const config = {
    // Consult https://kit.svelte.dev/docs/integrations#preprocessors
    // for more information about preprocessors
    preprocess: vitePreprocess(),

    kit: {
        // adapter-auto only supports some environments, see https://kit.svelte.dev/docs/adapter-auto for a list.
        // If your environment is not supported or you settled on a specific environment, switch out the adapter.
        // See https://kit.svelte.dev/docs/adapters for more information about adapters.

        adapter,
    }
};

export default config;
