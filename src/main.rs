use macroquad::window::Conf;

mod debugger;

fn window_conf() -> Conf {
    Conf {
        window_title: "Holani debug".to_owned(),
        window_height: 768,
        window_width: 1200,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    env_logger::init(); 
    debugger::debugger().await;
}

