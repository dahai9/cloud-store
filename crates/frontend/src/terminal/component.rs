use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::terminal::cell::Cell;
#[cfg(target_arch = "wasm32")]
use crate::terminal::parser::Terminal;
#[cfg(target_arch = "wasm32")]
use futures_channel::mpsc::{unbounded, UnboundedSender};
#[cfg(target_arch = "wasm32")]
use futures_util::{SinkExt, StreamExt};
#[cfg(target_arch = "wasm32")]
use gloo_net::websocket::futures::WebSocket;
#[cfg(target_arch = "wasm32")]
use gloo_net::websocket::Message;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::sleep;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;
#[cfg(target_arch = "wasm32")]
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use vte::Parser;
#[cfg(target_arch = "wasm32")]
use web_sys::console;
#[cfg(target_arch = "wasm32")]
use web_sys::wasm_bindgen::JsCast;

#[cfg(not(target_arch = "wasm32"))]
#[component]
pub fn TerminalView(url: String) -> Element {
    rsx! {
        div { "Terminal is only supported in web environments." }
    }
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn TerminalView(url: String) -> Element {
    const RESIZE_SYNC_INTERVAL: Duration = Duration::from_millis(250);
    const TERMINAL_PADDING_X: f32 = 30.0;
    const TERMINAL_PADDING_Y: f32 = 30.0;
    const MIN_INNER_WIDTH: f32 = 168.0;
    const MIN_INNER_HEIGHT: f32 = 128.0;

    let mut term = use_signal(|| Terminal::new(24, 80));
    let mut parser = use_signal(|| Parser::new());
    let ws_tx = use_hook(|| Rc::new(RefCell::new(None::<UnboundedSender<Message>>)));
    let mut error = use_signal(|| None::<String>);
    let mut transport_open = use_signal(|| false);
    let mut backend_ready = use_signal(|| false);
    let mut retry_count = use_signal(|| 0);
    let mut loading_tick = use_signal(|| 0usize);
    let mut ctrl_active = use_signal(|| false);

    let ws_tx_for_connection = ws_tx.clone();

    let mut connection = use_future(move || {
        let ws_tx = ws_tx_for_connection.clone();
        let url = url.clone();
        let _ = retry_count(); // Depend on retry_count to allow manual reconnect

        async move {
            transport_open.set(false);
            backend_ready.set(false);
            error.set(None);
            console::log_1(&format!("Connecting to terminal: {}", url).into());

            match WebSocket::open(&url) {
                Ok(ws) => {
                    console::log_1(&"WebSocket opened successfully".into());
                    transport_open.set(true);
                    let (tx, mut rx) = ws.split();
                    let (input_tx, mut input_rx) = unbounded::<Message>();
                    *ws_tx.borrow_mut() = Some(input_tx);

                    let _input_forwarder = spawn(async move {
                        let mut tx = tx;
                        while let Some(msg) = input_rx.next().await {
                            if tx.send(msg).await.is_err() {
                                break;
                            }
                        }
                    });

                    while let Some(msg) = rx.next().await {
                        match msg {
                            Ok(Message::Bytes(bytes)) => {
                                let mut t = term.write();
                                parser.write().advance(&mut *t, &bytes);
                            }
                            Ok(Message::Text(text)) => {
                                if text == "__cloud_store_console_ready__" {
                                    console::log_1(&"Console backend ready".into());
                                    backend_ready.set(true);
                                    continue;
                                }

                                let mut t = term.write();
                                parser.write().advance(&mut *t, text.as_bytes());
                            }
                            Err(err) => {
                                console::log_1(
                                    &format!("WebSocket message error: {:?}", err).into(),
                                );
                                error.set(Some(format!("Connection lost: {}", err)));
                                break;
                            }
                        }
                    }
                    transport_open.set(false);
                    backend_ready.set(false);
                    *ws_tx.borrow_mut() = None;
                }
                Err(err) => {
                    console::log_1(&format!("WebSocket connection failed: {:?}", err).into());
                    error.set(Some(format!(
                        "Connection failed: {}. (Check if you trusted the node certificate)",
                        err
                    )));
                }
            }
        }
    });

    let _loading_animation = use_future(move || {
        let _ = (backend_ready(), error(), retry_count());

        async move {
            loop {
                if backend_ready() || error().is_some() {
                    break;
                }

                sleep(Duration::from_millis(250)).await;

                if backend_ready() || error().is_some() {
                    break;
                }

                loading_tick.set((loading_tick() + 1) % 4);
            }
        }
    });

    let ws_tx_for_resize = ws_tx.clone();

    let _resize_sync = use_future(move || {
        let ws_tx = ws_tx_for_resize.clone();
        let _ = (backend_ready(), error(), retry_count());

        async move {
            let mut last_term_size = (0usize, 0usize);
            let mut last_viewport_size: Option<(i32, i32)> = None;

            loop {
                // Reset last_term_size if backend just became ready to force an initial sync
                if !backend_ready() {
                    last_term_size = (0, 0);
                    last_viewport_size = None;
                }

                if backend_ready() && error().is_none() {
                    let Some(win) = web_sys::window() else {
                        sleep(RESIZE_SYNC_INTERVAL).await;
                        continue;
                    };

                    let Some(doc) = win.document() else {
                        sleep(RESIZE_SYNC_INTERVAL).await;
                        continue;
                    };
                    let Some(el) = doc.get_element_by_id("terminal-viewport") else {
                        sleep(RESIZE_SYNC_INTERVAL).await;
                        continue;
                    };
                    let Ok(html) = el.dyn_into::<web_sys::HtmlElement>() else {
                        sleep(RESIZE_SYNC_INTERVAL).await;
                        continue;
                    };

                    let viewport_size = (html.client_width(), html.client_height());
                    if viewport_size.0 <= 0 || viewport_size.1 <= 0 {
                        sleep(RESIZE_SYNC_INTERVAL).await;
                        continue;
                    }

                    let initial_sync_needed = last_term_size == (0, 0);
                    let viewport_changed = last_viewport_size != Some(viewport_size);

                    if !initial_sync_needed && !viewport_changed {
                        sleep(RESIZE_SYNC_INTERVAL).await;
                        continue;
                    }

                    last_viewport_size = Some(viewport_size);

                    // Use layout box size instead of content-derived width to avoid
                    // feedback loops where terminal content growth keeps expanding cols.
                    let rect = html.get_bounding_client_rect();
                    let inner_width =
                        (rect.width() as f32 - TERMINAL_PADDING_X).max(MIN_INNER_WIDTH);
                    let inner_height =
                        (rect.height() as f32 - TERMINAL_PADDING_Y).max(MIN_INNER_HEIGHT);

                    if inner_width <= 0.0 || inner_height <= 0.0 {
                        sleep(RESIZE_SYNC_INTERVAL).await;
                        continue;
                    }

                    // For 14px monospace, chars are roughly 8.4px wide and 16px high.
                    // Using 8.5 provides a slight safety margin to prevent right-edge clipping.
                    let cols = (inner_width / 8.5).floor().max(20.0) as usize;
                    let rows = (inner_height / 16.0).floor().max(8.0) as usize;
                    let next_size = (rows, cols);

                    if last_term_size != next_size {
                        let mut t = term.write();
                        t.resize(rows, cols);
                        last_term_size = next_size;

                        if let Some(tx) = ws_tx.borrow().as_ref().cloned() {
                            let resize_message = serde_json::json!({
                                "kind": "resize",
                                "rows": rows,
                                "cols": cols,
                            })
                            .to_string();

                            let _ = tx.unbounded_send(Message::Text(resize_message));
                        }
                    }
                }

                sleep(RESIZE_SYNC_INTERVAL).await;
            }
        }
    });

    let ws_tx_for_input = ws_tx.clone();
    let on_keydown = move |evt: KeyboardEvent| {
        if let Some(tx) = ws_tx_for_input.borrow().as_ref().cloned() {
            let key_str = evt.key().to_string();
            let ctrl = evt
                .data()
                .downcast::<web_sys::KeyboardEvent>()
                .map(|e| e.ctrl_key() || e.meta_key())
                .unwrap_or(false)
                || ctrl_active();

            if ctrl_active() {
                ctrl_active.set(false);
            }

            // Map terminal input to raw bytes so control combinations (e.g. Ctrl+C = 0x03)
            // are delivered to the shell correctly.
            let to_send = if ctrl && key_str.len() == 1 {
                let b = key_str.as_bytes()[0].to_ascii_lowercase();
                match b {
                    b'a'..=b'z' => Some(vec![b - b'a' + 1]),
                    b'[' => Some(vec![0x1b]),
                    b'\\' => Some(vec![0x1c]),
                    b']' => Some(vec![0x1d]),
                    b'^' => Some(vec![0x1e]),
                    b'_' => Some(vec![0x1f]),
                    _ => None,
                }
            } else {
                match key_str.as_str() {
                    "Enter" => Some(b"\r".to_vec()),
                    "Backspace" => Some(b"\x7f".to_vec()),
                    "Tab" => Some(b"\t".to_vec()),
                    "Escape" => Some(b"\x1b".to_vec()),
                    "ArrowUp" => Some(b"\x1b[A".to_vec()),
                    "ArrowDown" => Some(b"\x1b[B".to_vec()),
                    "ArrowRight" => Some(b"\x1b[C".to_vec()),
                    "ArrowLeft" => Some(b"\x1b[D".to_vec()),
                    k if k.len() == 1 => Some(k.as_bytes().to_vec()),
                    _ => None,
                }
            };

            if let Some(bytes) = to_send {
                evt.prevent_default();
                let _ = tx.unbounded_send(Message::Bytes(bytes));
            }
        }
    };

    let term_val = term.read();
    let cursor_row = term_val.grid.cursor_row;
    let cursor_col = term_val.grid.cursor_col;
    let is_ready = backend_ready();
    let loading_frame = match loading_tick() {
        0 => "|",
        1 => "/",
        2 => "-",
        _ => "\\",
    };
    let loading_dots = ".".repeat(loading_tick() + 1);

    let mut reconnect_action = move || {
        error.set(None);
        transport_open.set(false);
        backend_ready.set(false);
        retry_count.set(retry_count() + 1);
        connection.restart();
    };

    let ws_tx_for_toolbar = ws_tx.clone();
    let send_raw = std::rc::Rc::new(move |bytes: Vec<u8>| {
        if let Some(tx) = ws_tx_for_toolbar.borrow().as_ref().cloned() {
            let _ = tx.unbounded_send(Message::Bytes(bytes));
        }
        // Return focus to terminal after clicking a button
        if let Some(win) = web_sys::window() {
            if let Some(doc) = win.document() {
                if let Some(el) = doc.get_element_by_id("terminal-viewport") {
                    if let Ok(html) = el.dyn_into::<web_sys::HtmlElement>() {
                        let _ = html.focus();
                    }
                }
            }
        }
    });

    let sr_esc = send_raw.clone();
    let sr_tab = send_raw.clone();
    let sr_up = send_raw.clone();
    let sr_down = send_raw.clone();
    let sr_left = send_raw.clone();
    let sr_right = send_raw.clone();
    let sr_pipe = send_raw.clone();
    let sr_dash = send_raw.clone();
    let sr_slash = send_raw.clone();

    rsx! {
        div { style: "width: 100%; min-height: 600px; display: flex; flex-direction: column; position: relative; background: #000000; border-radius: 8px; border: 1px solid #333; overflow: hidden;",

            if let Some(err) = error() {
                div {
                    class: "terminal-error",
                    style: "min-height: 600px; display: flex; align-items: center; justify-content: center; color: #ff8787; background: #1a1111; border: 1px solid #5c2b2b; border-radius: 12px; padding: 20px; text-align: center;",
                    div {
                        div { style: "font-weight: 700; margin-bottom: 8px;",
                            "Console connection failed"
                        }
                        div { "{err}" }
                        button {
                            class: "btn-secondary",
                            style: "margin-top: 16px; padding: 8px 14px; font-size: 13px;",
                            onclick: move |_| {
                                reconnect_action();
                            },
                            "Try Reconnect"
                        }
                    }
                }
            } else if !is_ready {
                div {
                    class: "console-loading",
                    style: "min-height: 600px; display: flex; align-items: center; justify-content: center; flex-direction: column; gap: 14px; background: #111; border: 1px solid #2b2b2b; border-radius: 12px; color: #e5e7eb; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \"Liberation Mono\", \"Courier New\", monospace; font-variant-ligatures: none; font-feature-settings: 'liga' 0;",
                    div { style: "font-size: 34px; font-weight: 700; line-height: 1;",
                        "{loading_frame}"
                    }
                    div { style: "font-size: 18px; font-weight: 700; letter-spacing: 0.03em;",
                        "Connecting to console"
                    }
                    div { style: "font-size: 13px; color: rgba(229, 231, 235, 0.68);",
                        "Waiting for backend websocket and Incus console session{loading_dots}"
                    }
                }
            } else {
                div {
                    class: "terminal-mobile-toolbar",
                    style: "display: flex; gap: 8px; padding: 8px 12px; background: #1a1a1a; border-bottom: 1px solid #333; overflow-x: auto; flex-shrink: 0;",

                    button {
                        class: "terminal-btn",
                        style: if ctrl_active() {
                            "padding: 6px 12px; background: #555; color: #fff; border: 1px solid #666; border-radius: 4px; font-family: monospace; font-size: 14px; cursor: pointer; white-space: nowrap; user-select: none;"
                        } else {
                            "padding: 6px 12px; background: #333; color: #eee; border: 1px solid #444; border-radius: 4px; font-family: monospace; font-size: 14px; cursor: pointer; white-space: nowrap; user-select: none;"
                        },
                        onclick: move |_| {
                            ctrl_active.set(!ctrl_active());
                            // Return focus to terminal
                            if let Some(win) = web_sys::window() {
                                if let Some(doc) = win.document() {
                                    if let Some(el) = doc.get_element_by_id("terminal-viewport") {
                                        if let Ok(html) = el.dyn_into::<web_sys::HtmlElement>() {
                                            let _ = html.focus();
                                        }
                                    }
                                }
                            }
                        },
                        "Ctrl"
                    }
                    button {
                        class: "terminal-btn",
                        style: "padding: 6px 12px; background: #333; color: #eee; border: 1px solid #444; border-radius: 4px; font-family: monospace; font-size: 14px; cursor: pointer; white-space: nowrap; user-select: none;",
                        onclick: move |_| sr_esc(b"\x1b".to_vec()),
                        "Esc"
                    }
                    button {
                        class: "terminal-btn",
                        style: "padding: 6px 12px; background: #333; color: #eee; border: 1px solid #444; border-radius: 4px; font-family: monospace; font-size: 14px; cursor: pointer; white-space: nowrap; user-select: none;",
                        onclick: move |_| sr_tab(b"\t".to_vec()),
                        "Tab"
                    }
                    button {
                        class: "terminal-btn",
                        style: "padding: 6px 12px; background: #333; color: #eee; border: 1px solid #444; border-radius: 4px; font-family: monospace; font-size: 14px; cursor: pointer; white-space: nowrap; user-select: none;",
                        onclick: move |_| sr_up(b"\x1b[A".to_vec()),
                        "↑"
                    }
                    button {
                        class: "terminal-btn",
                        style: "padding: 6px 12px; background: #333; color: #eee; border: 1px solid #444; border-radius: 4px; font-family: monospace; font-size: 14px; cursor: pointer; white-space: nowrap; user-select: none;",
                        onclick: move |_| sr_down(b"\x1b[B".to_vec()),
                        "↓"
                    }
                    button {
                        class: "terminal-btn",
                        style: "padding: 6px 12px; background: #333; color: #eee; border: 1px solid #444; border-radius: 4px; font-family: monospace; font-size: 14px; cursor: pointer; white-space: nowrap; user-select: none;",
                        onclick: move |_| sr_left(b"\x1b[D".to_vec()),
                        "←"
                    }
                    button {
                        class: "terminal-btn",
                        style: "padding: 6px 12px; background: #333; color: #eee; border: 1px solid #444; border-radius: 4px; font-family: monospace; font-size: 14px; cursor: pointer; white-space: nowrap; user-select: none;",
                        onclick: move |_| sr_right(b"\x1b[C".to_vec()),
                        "→"
                    }
                    button {
                        class: "terminal-btn",
                        style: "padding: 6px 12px; background: #333; color: #eee; border: 1px solid #444; border-radius: 4px; font-family: monospace; font-size: 14px; cursor: pointer; white-space: nowrap; user-select: none;",
                        onclick: move |_| sr_pipe(b"|".to_vec()),
                        "|"
                    }
                    button {
                        class: "terminal-btn",
                        style: "padding: 6px 12px; background: #333; color: #eee; border: 1px solid #444; border-radius: 4px; font-family: monospace; font-size: 14px; cursor: pointer; white-space: nowrap; user-select: none;",
                        onclick: move |_| sr_dash(b"-".to_vec()),
                        "-"
                    }
                    button {
                        class: "terminal-btn",
                        style: "padding: 6px 12px; background: #333; color: #eee; border: 1px solid #444; border-radius: 4px; font-family: monospace; font-size: 14px; cursor: pointer; white-space: nowrap; user-select: none;",
                        onclick: move |_| sr_slash(b"/".to_vec()),
                        "/"
                    }
                }
                div {
                    class: "terminal-wrapper-inner",
                    style: "position: relative; width: 100%; flex: 1; min-width: 0; overflow: hidden; background: #000000;",
                    div {
                        class: "terminal-container-rust",
                        id: "terminal-viewport",
                        style: "position: absolute; inset: 0; box-sizing: border-box; padding: 15px; font-family: \"DejaVu Sans Mono\", \"Noto Sans Mono\", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \"Liberation Mono\", \"Courier New\", monospace; font-variant-ligatures: none; font-feature-settings: 'liga' 0; font-size: 14px; line-height: 1.15; overflow: hidden; color: #ccc; outline: none; display: flex; flex-direction: column;",

                        tabindex: "0",
                        autofocus: "true",
                        onkeydown: on_keydown,
                        onclick: move |evt: MouseEvent| {
                            if let Some(mouse_evt) = evt.data().downcast::<web_sys::MouseEvent>() {
                                if let Some(target) = mouse_evt.current_target() {
                                    if let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() {
                                        let _ = el.focus();
                                    }
                                }
                            }
                        },

                        pre { style: "margin: 0; width: 100%; min-width: 0; max-width: 100%; white-space: pre; line-height: 1.15; letter-spacing: 0; flex: 1; overflow: hidden;",
                            {
                                term_val
                                    .grid
                                    .lines
                                    .iter()
                                    .enumerate()
                                    .map(|(r_idx, line)| {
                                        rsx! {
                                            div {
                                                key: "{r_idx}",
                                                style: "height: 1.15em; display: block; position: relative; width: 100%; overflow: hidden; white-space: pre; word-wrap: normal; word-break: keep-all; flex-shrink: 0;",
                                                {render_line(line)}
                                                if r_idx == cursor_row {
                                                    div { style: "position: absolute; width: 1ch; height: 1.15em; background: rgba(255, 255, 255, 0.6); left: {cursor_col}ch; top: 0; z-index: 1;" }
                                                }
                                            }
                                        }

                                    })
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn render_line(line: &[Cell]) -> Element {
    if line.is_empty() {
        return rsx! { span { "" } };
    }

    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut current_attrs = line[0].attrs;
    let mut current_is_ascii = true;

    for (i, cell) in line.iter().enumerate() {
        let is_ascii = cell.c.is_ascii();

        if i == 0 {
            current_is_ascii = is_ascii;
            current_text.push(cell.c);
            continue;
        }

        // Break if attributes change, OR if current is non-ASCII, OR if previous was non-ASCII.
        // This isolates every non-ASCII character into its own 1-char span to prevent fallback fonts
        // from breaking the monospace grid alignment.
        let should_break = cell.attrs != current_attrs || !is_ascii || !current_is_ascii;

        if should_break {
            if !current_text.is_empty() {
                spans.push((current_text.clone(), current_attrs));
                current_text.clear();
            }
            current_attrs = cell.attrs;
            current_is_ascii = is_ascii;
        }
        current_text.push(cell.c);
    }
    if !current_text.is_empty() {
        spans.push((current_text, current_attrs));
    }

    rsx! {
        {
            spans
                .into_iter()
                .enumerate()
                .map(|(idx, (text, attrs))| {
                    let chars_count = text.chars().count();
                    // Center non-ASCII characters (like box drawing) so they align visually even if fallback fonts are narrower.
                    // Left-align standard text.
                    let align = if chars_count == 1 && !text.chars().next().unwrap().is_ascii() {
                        "center"
                    } else {
                        "left"
                    };

                    let mut style = format!(
                        "display: inline-block; width: {}ch; text-align: {}; ",
                        chars_count, align
                    );

                    if attrs.bold {
                        style.push_str("font-weight: bold; ");
                    }
                    if attrs.italic {
                        style.push_str("font-style: italic; ");
                    }
                    if attrs.underline {
                        style.push_str("text-decoration: underline; ");
                    }
                    style.push_str(&format!("color: {}; ", color_to_css(attrs.fg)));
                    if attrs.bg != crate::terminal::cell::Color::Default {
                        style.push_str(&format!("background-color: {}; ", color_to_css(attrs.bg)));
                    }

                    // Use non-breaking space for spaces to ensure they take up space in HTML
                    let display_text = text.replace(' ', "\u{00A0}");

                    rsx! {
                        span { key: "{idx}", style: "{style}", "{display_text}" }
                    }
                })
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn color_to_css(color: crate::terminal::cell::Color) -> String {
    match color {
        crate::terminal::cell::Color::Default => "#ccc".to_string(),
        crate::terminal::cell::Color::Black => "#000".to_string(),
        crate::terminal::cell::Color::Red => "#ff5555".to_string(),
        crate::terminal::cell::Color::Green => "#50fa7b".to_string(),
        crate::terminal::cell::Color::Yellow => "#f1fa8c".to_string(),
        crate::terminal::cell::Color::Blue => "#bd93f9".to_string(),
        crate::terminal::cell::Color::Magenta => "#ff79c6".to_string(),
        crate::terminal::cell::Color::Cyan => "#8be9fd".to_string(),
        crate::terminal::cell::Color::White => "#bbbbbb".to_string(),
        crate::terminal::cell::Color::BrightBlack => "#555555".to_string(),
        crate::terminal::cell::Color::BrightRed => "#ff5555".to_string(),
        crate::terminal::cell::Color::BrightGreen => "#50fa7b".to_string(),
        crate::terminal::cell::Color::BrightYellow => "#f1fa8c".to_string(),
        crate::terminal::cell::Color::BrightBlue => "#bd93f9".to_string(),
        crate::terminal::cell::Color::BrightMagenta => "#ff79c6".to_string(),
        crate::terminal::cell::Color::BrightCyan => "#8be9fd".to_string(),
        crate::terminal::cell::Color::BrightWhite => "#ffffff".to_string(),
        crate::terminal::cell::Color::Rgb(r, g, b) => format!("rgb({r}, {g}, {b})"),
        crate::terminal::cell::Color::Ansi256(index) => ansi256_to_css(index),
    }
}

#[cfg(target_arch = "wasm32")]
fn ansi256_to_css(index: u8) -> String {
    match index {
        0 => "#000000".to_string(),
        1 => "#800000".to_string(),
        2 => "#008000".to_string(),
        3 => "#808000".to_string(),
        4 => "#000080".to_string(),
        5 => "#800080".to_string(),
        6 => "#008080".to_string(),
        7 => "#c0c0c0".to_string(),
        8 => "#808080".to_string(),
        9 => "#ff0000".to_string(),
        10 => "#00ff00".to_string(),
        11 => "#ffff00".to_string(),
        12 => "#0000ff".to_string(),
        13 => "#ff00ff".to_string(),
        14 => "#00ffff".to_string(),
        15 => "#ffffff".to_string(),
        16..=231 => {
            let i = index - 16;
            let r = i / 36;
            let g = (i % 36) / 6;
            let b = i % 6;
            let to_rgb = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            format!("rgb({}, {}, {})", to_rgb(r), to_rgb(g), to_rgb(b))
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            format!("rgb({gray}, {gray}, {gray})")
        }
    }
}
