use crate::state::{Conversation, User};
use crate::{convert_file_src, invoke};
use ev::MouseEvent;
use leptos::logging::log;
use leptos::*;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Deserialize)]
struct Model {
    id: u32,
    name: String,
    endpoint: String,
    profile: String,
}

#[component]
pub fn Nav<T, U>(
    conversations: Vec<Conversation>,
    user: User,
    on_select_conv: T,
    create_conv: U,
) -> impl IntoView
where
    T: FnMut(usize) -> () + 'static + Clone,
    U: FnMut(u32) -> () + 'static + Clone,
{
    let (models, set_models) = create_signal(vec![]);
    let close_models = move |_| {
        set_models.set(vec![]);
    };
    let new_conversation = move |_| {
        let set_models = set_models.clone();
        spawn_local(async move {
            let args = JsValue::null();
            let models: Vec<Model> =
                serde_wasm_bindgen::from_value(invoke("get_models", args).await).expect("models");
            log!("Got {models:?} models");
            set_models.set(models);
        });
    };
    // let profile = convert_file_src("profiles/Llama-3.1-8B-Instruct.png", "asset");
    view! {
        <div class="min-w-[480px] border-e-2 dark:border-gray-800 min-h-screen">
            <div class="text-center flex flex-col vertical-align">
                <div class="flex flex-row m-4">

                    <img class="w-8 h-8 rounded-full" src=&user.profile alt="Jese image" />
                    <h5
                        id="drawer-body-scrolling-label"
                        class="text-base py-2.5 font-semibold text-gray-500 uppercase dark:text-gray-400 w-full"
                    >
                        Chat
                    </h5>
                    <button
                        type="button"
                        class="text-white bg-gray-800 hover:bg-gray-900 focus:outline-none focus:ring-4 focus:ring-gray-300 font-medium rounded-lg text-sm px-5 py-2.5 dark:bg-gray-800 dark:hover:bg-gray-700 dark:focus:ring-gray-700 dark:border-gray-700"
                    >
                        +
                    </button>
                </div>
                <div class="py-4 overflow-y-auto grow">
                    <ul class="space-y-2 font-medium">
                        {move || {
                            conversations
                                .iter()
                                .enumerate()
                                .map(|(i, conv)| {
                                    let message = conv
                                        .messages
                                        .last()
                                        .map(|m| m.content.to_string())
                                        .unwrap_or("Empty".to_string());
                                    let mut value = on_select_conv.clone();
                                    let onclick = move |ev: MouseEvent| {
                                        ev.prevent_default();
                                        value(i);
                                    };

                                    view! {
                                        <li on:click=onclick>
                                            <a
                                                href="#"
                                                class="flex items-center p-2 text-gray-900 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group"
                                            >
                                                <svg
                                                    class="w-5 h-5 text-gray-500 transition duration-75 dark:text-gray-400 group-hover:text-gray-900 dark:group-hover:text-white"
                                                    aria-hidden="true"
                                                    xmlns="http://www.w3.org/2000/svg"
                                                    fill="currentColor"
                                                    viewBox="0 0 22 21"
                                                >
                                                    <path d="M16.975 11H10V4.025a1 1 0 0 0-1.066-.998 8.5 8.5 0 1 0 9.039 9.039.999.999 0 0 0-1-1.066h.002Z" />
                                                    <path d="M12.5 0c-.157 0-.311.01-.565.027A1 1 0 0 0 11 1.02V10h8.975a1 1 0 0 0 1-.935c.013-.188.028-.374.028-.565A8.51 8.51 0 0 0 12.5 0Z" />
                                                </svg>
                                                <span class="ms-3">{message}</span>
                                            </a>
                                        </li>
                                    }
                                })
                                .collect::<Vec<_>>()
                        }}
                    </ul>
                    <div>
                        {move || {
                            let models = models.get();
                            let suggestions = models
                                .iter()
                                .map(|model| {
                                    let profile = convert_file_src(
                                        &model.profile,
                                        "asset",
                                    );
                                    let mut value = create_conv.clone();
                                    let model_id = model.id.clone();

                                    view! {
                                        <li class="flex flex-row text-white hover:bg-gray-900 focus:outline-none focus:ring-4 focus:ring-gray-300 font-medium text-sm px-5 py-2.5 me-2 mb-2 dark:hover:bg-gray-700 dark:focus:ring-gray-700 dark:border-gray-700 w-full" on:click=move |_| {value(model_id);}>
                                            <img
                                                class="w-8 h-8 rounded-full"
                                                src=&profile
                                                alt="Jese image"
                                            />
                                            <span class="w-full text-left h-full p-2">
                                                {&model.name}
                                            </span>
                                            <button
                                                type="button"
                                                class="text-white bg-gray-800 hover:bg-gray-900 focus:outline-none focus:ring-4 focus:ring-gray-300 font-medium rounded-lg text-sm px-5 py-2.5 dark:bg-gray-800 dark:hover:bg-gray-700 dark:focus:ring-gray-700 dark:border-gray-700"
                                            >
                                                +
                                            </button>

                                        </li>
                                    }
                                })
                                .collect::<Vec<_>>();
                            {
                                if models.len() > 0 {
                                    view! {
                                        <div class="flex flex-col">
                                            <div>
                                                <ul class="max-h-64 overflow-y-auto">{suggestions}</ul>
                                                <div>

                                                    <button
                                                        type="button"
                                                        class="text-white bg-gray-800 hover:bg-gray-900 focus:outline-none focus:ring-4 focus:ring-gray-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-gray-800 dark:hover:bg-gray-700 dark:focus:ring-gray-700 dark:border-gray-700"
                                                        on:click=close_models
                                                    >
                                                        Close
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    }
                                } else {
                                    view! {
                                        <div>
                                            <button
                                                type="button"
                                                class="text-white bg-gray-800 hover:bg-gray-900 focus:outline-none focus:ring-4 focus:ring-gray-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-gray-800 dark:hover:bg-gray-700 dark:focus:ring-gray-700 dark:border-gray-700"
                                                on:click=new_conversation
                                            >
                                                + New conversation
                                            </button>
                                        </div>
                                    }
                                }
                            }
                        }}
                    </div>

                </div>
            </div>
        </div>
    }
}
