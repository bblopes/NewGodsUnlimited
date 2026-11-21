use macroquad::prelude::*;
use new_gods_unlimited::lab;

fn window_conf() -> Conf {
    Conf {
        window_title: "Laboratório de Experimentos — Ecossistema".to_owned(),
        window_width: 1440,
        window_height: 900,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    lab::run().await;
}
