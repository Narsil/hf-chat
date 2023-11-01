export const prerender = false
export const ssr = false
import { invoke } from '@tauri-apps/api/primitives'


export const load: LayoutServerLoad = async ({ locals, depends, url }) => {
    // When using the Tauri API npm package:
    const data = await invoke("load");
    return data;


};

