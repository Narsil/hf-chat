use crate::state::{Conversation, User};
use crate::{asset, invoke};
use ev::MouseEvent;
use leptos::logging::log;
use leptos::*;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Deserialize)]
struct Model {
    id: u32,
    name: String,
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
    let (show, set_show) = create_signal(true);
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
    view! {
        {move || {
            if show.get() {
                view! { <div /> }
            } else {
                view! {
                    <div
                        class="lg:hidden text-gray-500 dark:text-gray-400 p-5 absolute top-0 left-0"
                        on:click=move |_| {
                            set_show
                                .update(|s| {
                                    *s = !*s;
                                })
                        }
                    >
                        <svg viewBox="0 0 10 8" width="20">
                            <path
                                d="M1 1h8M1 4h 8M1 7h8"
                                stroke="currentColor"
                                fill="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                            />
                        </svg>
                    </div>
                }
            }
        }}
        // } else {
        // }
        <div
            class="lg:w-1/5 w-full lg:flex border-e-2 dark:border-gray-800 min-h-dvh max-h-dvh overflow-y-auto dark:text-white"
            class:hidden=move || !show.get()
        >
            <div class="text-center w-full flex flex-col vertical-align">
                <div
                    class="lg:hidden text-gray-500 dark:text-gray-400 p-5"
                    on:click=move |_| {
                        set_show
                            .update(|s| {
                                *s = !*s;
                            })
                    }
                >
                    <svg viewBox="0 0 10 10" width="20">
                        <path
                            d="M1 1L9 9M1 9L9 1"
                            stroke="currentColor"
                            fill="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                        />
                    </svg>
                </div>
                <div class="flex flex-row m-4">

                    <img class="w-10 h-10 rounded-full" src=&user.profile alt="Jese image" />
                    <h5
                        id="drawer-body-scrolling-label"
                        class="text-base py-2.5 font-semibold text-gray-500 uppercase dark:text-gray-400 w-full"
                    >
                        Chat
                    </h5>

                // <button
                // type="button"
                // class="text-white bg-gray-800 hover:bg-gray-900 focus:outline-none focus:ring-4 focus:ring-gray-300 font-medium rounded-lg text-sm px-5 py-2.5 dark:bg-gray-800 dark:hover:bg-gray-700 dark:focus:ring-gray-700 dark:border-gray-700"
                // >
                // +
                // </button>
                </div>
                <div class="py-4 overflow-y-auto grow">
                    <ul class="space-y-2 font-medium">
                        {move || {
                            conversations
                                .iter()
                                .enumerate()
                                .map(|(i, conv)| {
                                    let message = conv.title.clone();
                                    let mut value = on_select_conv.clone();
                                    let profile = asset(&conv.profile);
                                    let onclick = move |ev: MouseEvent| {
                                        ev.prevent_default();
                                        set_show.set(false);
                                        value(i);
                                    };
                                    // Only useful on mobile

                                    view! {
                                        <li on:click=onclick>
                                            <a
                                                href="#"
                                                class="flex items-center p-2 text-gray-900 rounded-lg dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 group"
                                            >
                                                <img
                                                    class="w-8 h-8 rounded-full"
                                                    src=&profile
                                                    alt="Jese image"
                                                />
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
                                    let profile = asset(&model.profile);
                                    let mut value = create_conv.clone();
                                    let model_id = model.id.clone();

                                    view! {
                                        <li
                                            class="flex flex-row dark:text-white text-black hover:bg-gray-900 focus:outline-none focus:ring-4 focus:ring-gray-300 font-medium text-sm px-5 py-2.5 me-2 mb-2 dark:hover:bg-gray-700 dark:focus:ring-gray-700 dark:border-gray-700 w-full"
                                            on:click=move |_| {
                                                value(model_id);
                                            }
                                        >
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
