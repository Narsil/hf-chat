use crate::invoke;
use leptos::logging::log;
use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct LoginArgs {
    url: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginCallbackArgs {
    pub code: String,
    pub state: String,
}

#[component]
pub fn Login() -> impl IntoView {
    let click = move |_| {
        spawn_local(async {
            let location = window().location();
            let protocol = location.protocol().expect("protocol");
            let host = location.host().expect("host");
            let url = format!("{protocol}//{host}/login/callback");
            let args = serde_wasm_bindgen::to_value(&LoginArgs { url }).unwrap();
            let value = invoke("login", args).await;
            let redirect: String =
                serde_wasm_bindgen::from_value(value).expect("Correct conversations");

            log!("Redirect {redirect:?}");
            location.set_href(&redirect).expect("Redirect");
        });
    };
    view! {
        <div class="flex items-center justify-center w-56 h-56 border border-gray-200  bg-gray-50 dark:bg-gray-800 dark:border-gray-700 w-full h-screen">
            <button
                type="button"
                class="text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800"
                on:click=click
            >
                Login
            </button>
        </div>
    }
}
