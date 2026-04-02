mod api;
mod models;
mod pages;

#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
fn main() {
    launch(pages::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Reference modules to avoid dead_code warnings during host-side cargo check
    let _ = (api::load_initial_session, models::SessionState::new, pages::App);
    eprintln!("frontend is a web-only app; run with dx serve --platform web");
}
