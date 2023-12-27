import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite'
import { internalIpV4 } from 'internal-ip'
import Icons from "unplugin-icons/vite";

// @ts-expect-error process is a nodejs global
const mobile = !!/android|ios/.exec(process.env.TAURI_ENV_TARGET_TRIPLE);
console.log(`Mobile ${mobile}`);

export default defineConfig(async () => {

    /** @type {import('vite').UserConfig} */
    const config = {
        server: {
            port: 5173,
            strictPort: true,
            host: mobile ? "0.0.0.0" : false,
            hmr: mobile
                ? {
                    protocol: "ws",
                    host: await internalIpV4(),
                    port: 5183,
                }
                : undefined,
        },
        plugins: [
            sveltekit(),
            Icons({ compiler: "svelte" })
        ],
        test: {
            include: ['src/**/*.{test,spec}.{js,ts}']
        }
    };

    return config;
});
