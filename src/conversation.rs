use crate::invoke;
use crate::loading::Loading;
use crate::message::{Message, Msg};
use crate::state::{Message as DbMsg, User};
use chrono::Utc;
use leptos::leptos_dom::ev::SubmitEvent;
use leptos::logging::log;
use leptos::*;
use serde::{Deserialize, Serialize};
use web_sys::window;

#[derive(Serialize, Deserialize)]
struct GetMessages {
    conversationid: u32,
}

#[derive(Serialize, Deserialize)]
struct NewMessage {
    conversationid: u32,
    content: String,
    authorid: u32,
}

#[derive(Serialize)]
struct Query {
    conversationid: u32,
}

#[derive(Clone, Serialize, Deserialize)]
struct ConvData {
    messages: Vec<Msg>,
    me: User,
    other: User,
}

#[derive(Serialize, Deserialize)]
struct DbConvData {
    messages: Vec<DbMsg>,
    users: Vec<User>,
}

#[component]
pub fn Conversation(conversationid: u32, me: u32, model: u32) -> impl IntoView {
    let ref_input = create_node_ref::<html::Input>();
    create_effect(move |_| {
        if let Some(ref_input) = ref_input.get() {
            let _ = ref_input.on_mount(|input| {
                input.focus().ok();
            });
        }
    });
    let (message, set_message) = create_signal(String::new());
    let convdata = create_resource(
        move || (),
        move |_| async move {
            let args = serde_wasm_bindgen::to_value(&GetMessages { conversationid }).unwrap();
            let value = invoke("get_messages", args).await.unwrap();
            let convdata: DbConvData =
                serde_wasm_bindgen::from_value(value).expect("Correct conversations");
            log!("Users {:?}", convdata.users.len());

            let me_user = convdata.users.iter().find(|u| u.id == me).expect("Me");
            let other = convdata
                .users
                .iter()
                .find(|u| u.id == model)
                .expect("Other");

            let messages = convdata
                .messages
                .into_iter()
                .map(|message| {
                    let is_me = message.user_id == me_user.id;
                    let user = if is_me {
                        me_user.clone()
                    } else {
                        other.clone()
                    };
                    Msg {
                        created_at: message.created_at,
                        content: message.content,
                        is_me,
                        user,
                    }
                })
                .collect();

            ConvData {
                messages,
                me: me_user.clone(),
                other: other.clone(),
            }
        },
    );

    let update_message = move |ev| {
        let v = event_target_value(&ev);
        set_message.set(v);
    };
    let send_message = move |ev: SubmitEvent| {
        ev.prevent_default();
        let content = message.get();
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&NewMessage {
                conversationid,
                content,
                authorid: me,
            })
            .unwrap();
            invoke("new_message", args).await.unwrap();

            let args = Query { conversationid };
            loop {
                let arg = serde_wasm_bindgen::to_value(&args).unwrap();
                let res = invoke("get_chunk", arg).await;
                if res.is_err() {
                    window().unwrap().location().reload().unwrap();
                }
                let res = res.unwrap();
                let chunk: Option<String> = serde_wasm_bindgen::from_value(res).expect("Chunk");
                if let Some(chunk) = chunk {
                    convdata.update(|convdata| {
                        convdata.as_mut().map(|convdata| {
                            let user = convdata.other.clone();
                            if let Some(message) = convdata.messages.last_mut() {
                                if !message.is_me {
                                    message.content.push_str(&chunk);
                                } else {
                                    convdata.messages.push(Msg {
                                        created_at: Utc::now(),
                                        user,
                                        is_me: false,
                                        content: chunk,
                                    })
                                }
                            } else {
                                convdata.messages.push(Msg {
                                    created_at: Utc::now(),
                                    user,
                                    is_me: false,
                                    content: chunk,
                                })
                            }
                        });
                    });
                } else {
                    break;
                }
            }
        });
        log!("Inserting new message");
        convdata.update(|convdata| {
            convdata.as_mut().map(|convdata| {
                convdata.messages.push(Msg {
                    created_at: Utc::now(),
                    user: convdata.me.clone(),
                    is_me: true,
                    content: message.get(),
                })
            });
        });
        set_message.set(String::new());
    };

    view! {
        <div class="h-dvh max-h-dvh grow flex flex-col scrollbar lg:w-4/5 w-dvw max-w-dvw">
            <main class="grow flex flex-col-reverse overflow-auto max-h-screen">
                <Suspense fallback=move || {
                    view! { <Loading /> }
                }>
                    {move || {
                        convdata
                            .get()
                            .map(|convdata| {
                                convdata
                                    .messages
                                    .into_iter()
                                    .rev()
                                    .map(|message| {

                                        view! { <Message message=message /> }
                                    })
                                    .collect::<Vec<_>>()
                            })
                    }}
                </Suspense>

            </main>
            <form class="w-full" on:submit=send_message>
                <label for="chat" class="sr-only">
                    Your message
                </label>
                <div class="flex items-center px-3 py-2 bg-gray-50 dark:bg-gray-700">
                    <button
                        type="button"
                        class="inline-flex justify-center p-2 text-gray-500 rounded-lg cursor-pointer hover:text-gray-900 hover:bg-gray-100 dark:text-gray-400 dark:hover:text-white dark:hover:bg-gray-600"
                    >
                        <svg
                            class="w-5 h-5"
                            aria-hidden="true"
                            xmlns="http://www.w3.org/2000/svg"
                            fill="none"
                            viewBox="0 0 20 18"
                        >
                            <path
                                fill="currentColor"
                                d="M13 5.5a.5.5 0 1 1-1 0 .5.5 0 0 1 1 0ZM7.565 7.423 4.5 14h11.518l-2.516-3.71L11 13 7.565 7.423Z"
                            />
                            <path
                                stroke="currentColor"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M18 1H2a1 1 0 0 0-1 1v14a1 1 0 0 0 1 1h16a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1Z"
                            />
                            <path
                                stroke="currentColor"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M13 5.5a.5.5 0 1 1-1 0 .5.5 0 0 1 1 0ZM7.565 7.423 4.5 14h11.518l-2.516-3.71L11 13 7.565 7.423Z"
                            />
                        </svg>
                        <span class="sr-only">Upload image</span>
                    </button>
                    <button
                        type="button"
                        class="p-2 text-gray-500 rounded-lg cursor-pointer hover:text-gray-900 hover:bg-gray-100 dark:text-gray-400 dark:hover:text-white dark:hover:bg-gray-600"
                    >
                        <svg
                            class="w-5 h-5"
                            aria-hidden="true"
                            xmlns="http://www.w3.org/2000/svg"
                            fill="none"
                            viewBox="0 0 20 20"
                        >
                            <path
                                stroke="currentColor"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M13.408 7.5h.01m-6.876 0h.01M19 10a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM4.6 11a5.5 5.5 0 0 0 10.81 0H4.6Z"
                            />
                        </svg>
                        <span class="sr-only">Add emoji</span>
                    </button>
                    <input
                        id="chat"
                        rows="1"
                        class="block mx-4 p-2.5 w-full text-sm text-gray-900 bg-white rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-800 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500 resize-none"
                        placeholder="Your message..."
                        _ref=ref_input
                        on:input=update_message
                        prop:value=message
                    />
                    <button
                        type="submit"
                        class="inline-flex justify-center p-2 text-blue-600 rounded-full cursor-pointer hover:bg-blue-100 dark:text-blue-500 dark:hover:bg-gray-600"
                    >
                        <svg
                            class="w-5 h-5 rotate-90 rtl:-rotate-90"
                            aria-hidden="true"
                            xmlns="http://www.w3.org/2000/svg"
                            fill="currentColor"
                            viewBox="0 0 18 20"
                        >
                            <path d="m17.914 18.594-8-18a1 1 0 0 0-1.828 0l-8 18a1 1 0 0 0 1.157 1.376L8 18.281V9a1 1 0 0 1 2 0v9.281l6.758 1.689a1 1 0 0 0 1.156-1.376Z" />
                        </svg>
                        <span class="sr-only">Send message</span>
                    </button>
                </div>
            </form>

        </div>
    }
}
