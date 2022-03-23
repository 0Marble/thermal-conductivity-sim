mod app;
mod model;
mod renderer;
mod window;

use renderer::error::Error;

macro_rules! panic_call {
    ($func:expr) => {
        call!($func).unwrap_or_else(|e| panic!("{}", e))
    };
}

fn main() {
    let mut app = panic_call!(app::app::App::new());
    panic_call!(app.run());
}
