use crate::conversation::Conversation as Conv;
use crate::loading::Loading;
use crate::login::{Login, LoginCallbackArgs};
use crate::nav::Nav;
use crate::state::{Conversation, User};
use leptos::logging::log;
use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI_INTERNALS__"])]
    fn convertFileSrc(filepath: &str, protocol: &str) -> JsValue;


    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "log"])]
    pub async fn trace(log: &str);

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "log"])]
    pub async fn debug(log: &str);

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "log"])]
    pub async fn info(log: &str);

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "log"])]
    pub async fn warn(log: &str);

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "log"])]
    pub async fn error(log: &str);
}

#[derive(Serialize, Deserialize)]
struct CreateConversation {
    modelid: u32,
}

pub fn asset(filepath: &str) -> String {
    if filepath.starts_with('/') {
        let value = convertFileSrc(filepath, "asset");
        serde_wasm_bindgen::from_value(value).expect("Convertion")
    } else {
        filepath.to_owned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Load {
    conversations: Vec<Conversation>,
    user: Option<User>,
    users: Vec<User>,
}

#[component]
pub fn App() -> impl IntoView {
    let (conversation, set_conversation): (
        ReadSignal<Option<Conversation>>,
        WriteSignal<Option<Conversation>>,
    ) = create_signal(None);

    let (sigload, set_sigload) = create_signal(0);

    if let Ok(search) = window().location().search() {
        let url = url::Url::parse(&format!("http://someUrl.com{search}")).expect("Parse");
        let params = url.query_pairs();
        let mut code = None;
        let mut state = None;
        for (key, value) in params {
            match &key[..] {
                "code" => code = Some(value.to_string()),
                "state" => state = Some(value.to_string()),
                string => log!("Unexpected param {string}: {value}"),
            }
        }
        if let (Some(code), Some(state)) = (code, state) {
            spawn_local(async {
                let args =
                    serde_wasm_bindgen::to_value(&LoginCallbackArgs { code, state }).unwrap();
                invoke("login_callback", args).await;
                let location = window().location();
                let protocol = location.protocol().expect("protocol");
                let host = location.host().expect("host");
                let url = format!("{protocol}//{host}/");
                location.set_href(&url).expect("set href");
            });
        }
    }

    let load = create_resource(
        move || sigload.get(),
        |_| async move {
            let args = JsValue::undefined();
            let value = invoke("load", args).await;
            let load: Load = serde_wasm_bindgen::from_value(value).expect("Correct conversations");
            load
        },
    );

    let on_select_conv = move |index: usize| {
        let conversation: Option<Conversation> = load
            .get()
            .map(|load| load.conversations.get(index).cloned())
            .flatten();
        set_conversation.set(conversation);
    };
    let create_conv = move |model_id: u32| {
        spawn_local(async move {
            let args =
                serde_wasm_bindgen::to_value(&CreateConversation { modelid: model_id }).unwrap();
            log!("Args {args:?}");
            let conv_value = invoke("create_conversation", args).await;
            let conversation =
                serde_wasm_bindgen::from_value(conv_value).expect("Conversation created");
            set_conversation.set(conversation);
            set_sigload.update(|sig| *sig = *sig + 1);
        });
    };
    view! {
        <div class="flex flex-row">
            <Suspense fallback=move || {
                view! { <Loading /> }
            }>
                {move || {
                    load.get()
                        .map(|load| {
                            let conversations = load.conversations;
                            if let Some(user) = load.user {
                                view! { <Nav conversations user on_select_conv create_conv /> }
                            } else {
                                view! { <Login /> }
                            }
                        })
                }}
            </Suspense>
            <Suspense fallback=move || {
                view! { <Loading /> }
            }>
                {move || {
                    conversation
                        .get()
                        .map(|conversation| {
                            view! {
                                <Conv
                                    conversationid=conversation.id
                                    me=(move || load.get().unwrap().user.unwrap().id)()
                                    model=conversation.user_id
                                    users=(move || load.get().unwrap().users)()
                                />
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}
