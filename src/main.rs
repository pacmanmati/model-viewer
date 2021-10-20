mod app;
mod render;
use app::Application;

fn main() {
    env_logger::init();
    Application::new("Model Viewer", 60.0).run();
}
