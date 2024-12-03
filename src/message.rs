use crate::app::invoke;
use crate::asset;
use crate::state::User;
use chrono::{DateTime, Local, Utc};
use leptos::logging::log;
use leptos::IntoView;
use leptos::*;
use pulldown_cmark::{Options, Parser};
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Serialize, Deserialize)]
pub struct Msg {
    pub content: String,
    pub user: User,
    pub is_me: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
enum Play {
    Stopped,
    Loading,
    Playing(String),
}

#[derive(Debug, Serialize, Clone)]
struct PlayMessage {
    content: String,
}

#[component]
pub fn Message(message: Msg) -> impl IntoView {
    let mut parsed = String::new();
    let options = Options::all();
    let parser = Parser::new_ext(&message.content, options);
    crate::html::push_html(&mut parsed, parser);
    let datemsg = format!(
        "{}",
        DateTime::<Local>::from(message.created_at).format("%H:%M")
    );
    let profile = asset(&message.user.profile);
    let (playing, set_playing) = create_signal(Play::Stopped);

    let content = message.content.clone();
    let play = move |_| {
        let content = content.clone();
        set_playing.update(|p| {
            log!("Updating play {p:?}");
            *p = match p {
                Play::Stopped => {
                    spawn_local(async move {
                        let args = serde_wasm_bindgen::to_value(&PlayMessage { content }).unwrap();
                        let filename = invoke("play_message", args).await.unwrap();
                        let filename: String = serde_wasm_bindgen::from_value(filename).unwrap();
                        set_playing.update(|p| {
                            if let Play::Loading = p {
                                *p = Play::Playing(filename)
                            }
                        })
                    });
                    Play::Loading
                }
                Play::Loading => Play::Stopped,
                Play::Playing(_) => Play::Stopped,
            }
        });
    };
    let ended = move |_| {
        set_playing.update(|p| {
            if let Play::Playing(_) = p {
                *p = Play::Stopped;
            }
        })
    };
    view! {
        <div class="flex items-start m-5 gap-2.5" class:flex-row-reverse=move || message.is_me>
            <img class="w-8 h-8 rounded-full" src=profile alt="User avatar" />
            <div class="flex flex-col gap-1 max-w-[90%]">
                <div class="flex items-center space-x-2 rtl:space-x-reverse">
                    <span class="text-sm font-semibold text-gray-900 dark:text-white">
                        {message.user.name}
                    </span>
                    <span class="text-sm font-normal text-gray-500 dark:text-gray-400">
                        {datemsg.clone()}
                    </span>
                </div>
                <div class="flex flex-col leading-1.5 p-4 border-gray-200 bg-gray-100 rounded-e-xl rounded-es-xl dark:bg-gray-700">
                    <p class="text-sm font-normal text-gray-900 dark:text-white">
                        <div inner_html=parsed />
                        <div on:click=play>
                            {move || {
                                match playing.get() {
                                    Play::Stopped => {
                                        view! {
                                            <svg
                                                class="h-4 w-4 text-gray-500"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="2"
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                            >
                                                <polygon points="5 3 19 12 5 21 5 3" />
                                            </svg>
                                        }
                                            .into_any()
                                    }
                                    Play::Loading => {

                                        view! {
                                            <svg
                                                aria-hidden="true"
                                                class="w-4 h-4 text-gray-200 animate-spin dark:text-gray-600 fill-blue-600"
                                                viewBox="0 0 100 101"
                                                fill="none"
                                                xmlns="http://www.w3.org/2000/svg"
                                            >
                                                <path
                                                    d="M100 50.5908C100 78.2051 77.6142 100.591 50 100.591C22.3858 100.591 0 78.2051 0 50.5908C0 22.9766 22.3858 0.59082 50 0.59082C77.6142 0.59082 100 22.9766 100 50.5908ZM9.08144 50.5908C9.08144 73.1895 27.4013 91.5094 50 91.5094C72.5987 91.5094 90.9186 73.1895 90.9186 50.5908C90.9186 27.9921 72.5987 9.67226 50 9.67226C27.4013 9.67226 9.08144 27.9921 9.08144 50.5908Z"
                                                    fill="currentColor"
                                                />
                                                <path
                                                    d="M93.9676 39.0409C96.393 38.4038 97.8624 35.9116 97.0079 33.5539C95.2932 28.8227 92.871 24.3692 89.8167 20.348C85.8452 15.1192 80.8826 10.7238 75.2124 7.41289C69.5422 4.10194 63.2754 1.94025 56.7698 1.05124C51.7666 0.367541 46.6976 0.446843 41.7345 1.27873C39.2613 1.69328 37.813 4.19778 38.4501 6.62326C39.0873 9.04874 41.5694 10.4717 44.0505 10.1071C47.8511 9.54855 51.7191 9.52689 55.5402 10.0491C60.8642 10.7766 65.9928 12.5457 70.6331 15.2552C75.2735 17.9648 79.3347 21.5619 82.5849 25.841C84.9175 28.9121 86.7997 32.2913 88.1811 35.8758C89.083 38.2158 91.5421 39.6781 93.9676 39.0409Z"
                                                    fill="currentFill"
                                                />
                                            </svg>
                                        }
                                            .into_any()
                                    }
                                    Play::Playing(_filename) => {
                                        view! {
                                            <div>
                                                <svg
                                                    class="h-4 w-4 text-gray-500"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="2"
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                >
                                                    <rect x="6" y="4" width="4" height="16" />
                                                    <rect x="14" y="4" width="4" height="16" />
                                                </svg>
                                            </div>
                                        }
                                            .into_any()
                                    }
                                }
                            }}
                        </div>
                        {move || {
                            if let Play::Playing(filename) = playing.get() {
                                view! {
                                    <audio autoplay on:ended=ended>
                                        <source src=asset(&filename) />
                                        Your browser does not support the audio element.
                                    </audio>
                                }.into_any()
                            } else {
                                view! { <div /> }.into_any()
                            }
                        }}

                    </p>
                </div>
                <span class="text-sm invisible font-normal text-gray-500 dark:text-gray-400">
                    Delivered
                </span>
            </div>
            <button
                id="dropdownMenuIconButton"
                data-dropdown-toggle="dropdownDots"
                data-dropdown-placement="bottom-start"
                class="inline-flex self-center items-center p-2 text-sm font-medium text-center text-gray-900 bg-white rounded-lg hover:bg-gray-100 focus:ring-4 focus:outline-none dark:text-white focus:ring-gray-50 dark:bg-gray-900 dark:hover:bg-gray-800 dark:focus:ring-gray-600"
                type="button"
            >
                <svg
                    class="w-4 h-4 text-gray-500 dark:text-gray-40 invisible hover:visible 0"
                    aria-hidden="true"
                    xmlns="http://www.w3.org/2000/svg"
                    fill="currentColor"
                    viewBox="0 0 4 15"
                >
                    <path d="M3.5 1.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0Zm0 6.041a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0Zm0 5.959a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0Z" />
                </svg>
            </button>
            <div
                id="dropdownDots"
                class="z-10 hidden bg-white divide-y divide-gray-100 rounded-lg shadow w-40 dark:bg-gray-700 dark:divide-gray-600"
            >
                <ul
                    class="py-2 text-sm text-gray-700 dark:text-gray-200"
                    aria-labelledby="dropdownMenuIconButton"
                >
                    <li>
                        <a
                            href="#"
                            class="block px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-600 dark:hover:text-white"
                        >
                            Reply
                        </a>
                    </li>
                    <li>
                        <a
                            href="#"
                            class="block px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-600 dark:hover:text-white"
                        >
                            Forward
                        </a>
                    </li>
                    <li>
                        <a
                            href="#"
                            class="block px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-600 dark:hover:text-white"
                        >
                            Copy
                        </a>
                    </li>
                    <li>
                        <a
                            href="#"
                            class="block px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-600 dark:hover:text-white"
                        >
                            Report
                        </a>
                    </li>
                    <li>
                        <a
                            href="#"
                            class="block px-4 py-2 hover:bg-gray-100 dark:hover:bg-gray-600 dark:hover:text-white"
                        >
                            Delete
                        </a>
                    </li>
                </ul>
            </div>
        </div>
    }
}
