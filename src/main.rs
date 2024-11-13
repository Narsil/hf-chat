mod app;
mod conversation;
mod loading;
mod login;
mod message;
mod nav;
mod state;

use app::*;
use leptos::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! { <App /> }
    })
}
