export const prerender = false
export const ssr = false
import { invoke } from '@tauri-apps/api/tauri'


export const load = async ({ params, depends, url }) => {
    // When using the Tauri API npm package:
    const data = await invoke("load_conversation", { id: params.id });
    return data;
};

