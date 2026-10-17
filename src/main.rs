mod app;
mod config;
mod environment;
mod simulation;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    let demographics_window = std::env::args().any(|argument| argument == "--demographics-window");
    Conf {
        window_title: if demographics_window {
            "Demografia — New Gods Unlimited".to_owned()
        } else {
            "New Gods Unlimited".to_owned()
        },
        window_width: if demographics_window { 1280 } else { 1440 },
        window_height: if demographics_window { 860 } else { 900 },
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() == Some("--demographics-window") {
        if let Some(path) = arguments.next() {
            app::run_demographics_window(path.into()).await;
        }
    } else {
        app::run().await;
    }
}
