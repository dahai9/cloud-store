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
    eprintln!("admin frontend is a web-only app; run with dx serve --platform web");
}
