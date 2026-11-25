use glib::ExitCode;
use gtk::prelude::*;
use gtk::{Application, glib};

const APP_ID: &str = "dev.nservo.Browser";

const LOGGER: glib::GlibLogger = glib::GlibLogger::new(
    glib::GlibLoggerFormat::Plain,
    glib::GlibLoggerDomain::CrateTarget,
);

fn main() -> ExitCode {
    // Must be the very first call in main(): servo-gtk re-executes this
    // binary as its Servo rendering subprocess.
    servo_gtk::run_as_runner_if_requested();

    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Info);

    if let Err(err) = nservo::engine::epoxy::init() {
        eprintln!("Failed to initialize OpenGL loader: {err}");
        return ExitCode::FAILURE;
    }

    let initial_url = std::env::args().nth(1);
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(move |app| {
        nservo::ui::style::install_theme();
        let start_url = initial_url.as_deref();
        nservo::ui::window::BrowserWindow::open(app, start_url);
    });

    app.run()
}
