use crate::asset;
use crate::state::User;
use chrono::{DateTime, Local, Utc};
use leptos::IntoView;
use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Msg {
    pub content: String,
    pub user: User,
    pub is_me: bool,
    pub created_at: DateTime<Utc>,
}

#[component]
pub fn Message(message: Msg) -> impl IntoView {
    let parser = pulldown_cmark::Parser::new(&message.content);
    let mut parsed = String::new();
    pulldown_cmark::html::push_html(&mut parsed, parser);
    let datemsg = format!(
        "{}",
        DateTime::<Local>::from(message.created_at).format("%H:%M")
    );
    let profile = asset(&message.user.profile);
    view! {
        <div class="flex items-start m-5 gap-2.5" class:flex-row-reverse=move || message.is_me>
            <img class="w-8 h-8 rounded-full" src=&profile alt="Jese image" />
            <div class="flex flex-col gap-1 max-w-[90%]">
                <div class="flex items-center space-x-2 rtl:space-x-reverse">
                    <span class="text-sm font-semibold text-gray-900 dark:text-white">
                        {&message.user.name}
                    </span>
                    <span class="text-sm font-normal text-gray-500 dark:text-gray-400">
                        {&datemsg}
                    </span>
                </div>
                <div class="flex flex-col leading-1.5 p-4 border-gray-200 bg-gray-100 rounded-e-xl rounded-es-xl dark:bg-gray-700">
                    <p class="text-sm font-normal text-gray-900 dark:text-white">
                        <div inner_html=parsed />
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
        <script>hljs.highlightAll();</script>
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_markdown() {
        // Create parser with example Markdown text.
        let markdown_input = "hello world";
        let parser = pulldown_cmark::Parser::new(markdown_input);

        // Write to a new String buffer.
        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);
        assert_eq!(&html_output, "<p>hello world</p>\n");

        // Create parser with example Markdown text.
        let markdown_input =
            "Compile the program using the `rustc` command:\n\n```bash\nrustc main.rs\n```";
        let parser = pulldown_cmark::Parser::new(markdown_input);

        // Write to a new String buffer.
        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);
        assert_eq!(&html_output, "");
    }
}
