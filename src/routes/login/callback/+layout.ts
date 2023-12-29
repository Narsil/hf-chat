export const prerender = false
export const ssr = false
import { invoke } from '@tauri-apps/api/primitives'


export const load = async ({ locals, depends, url }) => {
    // When using the Tauri API npm package:
    console.log("Callbacked", url);
    const queryString = window.location.search;
    const urlParams = new URLSearchParams(queryString);
    console.log("Callbacked", urlParams);
    const code = urlParams.get("code");
    const state = urlParams.get("state");
    if (code != undefined && state != undefined) {
        const data = await invoke("login_callback", { code, state });
        window.location = "/";
    }
};
