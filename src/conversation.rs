use crate::state::{Conversation, Message};
use leptos::leptos_dom::ev::SubmitEvent;
use leptos::*;

impl Message {
    fn render(&self) -> impl IntoView {
        // let is_me = self.author.is_me;
        let is_me = false;
        view! {
            <div class="flex items-start m-5 gap-2.5" class:flex-row-reverse=move || is_me>
                <img class="w-8 h-8 rounded-full" src=&self.author.profile alt="Jese image" />
                <div class="flex flex-col gap-1 w-full max-w-[320px]">
                    <div class="flex items-center space-x-2 rtl:space-x-reverse">
                        <span class="text-sm font-semibold text-gray-900 dark:text-white">
                            {&self.author.name}
                        </span>
                        <span class="text-sm font-normal text-gray-500 dark:text-gray-400">
                            11:46
                        </span>
                    </div>
                    <div class="flex flex-col leading-1.5 p-4 border-gray-200 bg-gray-100 rounded-e-xl rounded-es-xl dark:bg-gray-700">
                        <p class="text-sm font-normal text-gray-900 dark:text-white">
                            {&self.content}
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
}

#[component]
pub fn Conversation(
    conversation: Conversation,
    // #[prop(into)] conversation: Signal<Conversation>,
    // mut on_message: T,
) -> impl IntoView {
    let (message, set_message) = create_signal(String::new());

    let update_message = move |ev| {
        let v = event_target_value(&ev);
        set_message.set(v);
    };
    let send_message = move |ev: SubmitEvent| {
        ev.prevent_default();
        // on_message(message.get());
        set_message.set(String::new());
    };

    let messages = move || {
        conversation
            .messages
            .iter()
            .map(|message| message.render())
            .collect::<Vec<_>>()
    };

    view! {
        <div class="h-screen grow flex flex-col scrollbar">
            <main class="grow overflow-auto max-h-screen">{messages}</main>
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
