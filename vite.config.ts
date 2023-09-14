import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite'
import { internalIpV4 } from 'internal-ip'
import Icons from "unplugin-icons/vite";

export default defineConfig(async () => {
    /** @type {import('vite').UserConfig} */
    var config = {
        plugins: [
            sveltekit(),
            Icons({ compiler: "svelte" })
        ],
        test: {
            include: ['src/**/*.{test,spec}.{js,ts}']
        }
    };
    if (process.env.MOBILE) {
        const host = await internalIpV4();
        const server = {
            host: '0.0.0.0', // listen on all addresses
            port: 5173,
            strictPort: true,
            hmr: {
                protocol: 'ws',
                host: "192.168.15.18",
                port: 5183,
            },
        };
        config.server = server;
    }


    return config;
});
