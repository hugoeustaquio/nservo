use glib::ExitCode;
use gtk::prelude::*;
use gtk::{Application, glib};

const APP_ID: &str = "dev.nservo.Browser";

const LOGGER: glib::GlibLogger = glib::GlibLogger::new(
    glib::GlibLoggerFormat::Plain,
    glib::GlibLoggerDomain::CrateTarget,
);

#[cfg(target_os = "linux")]
fn arm_runner_parent_death_signal() {
    let is_runner = std::env::args()
        .skip(1)
        .take_while(|arg| arg != "--")
        .any(|arg| arg == "--servo-gtk-runner");
    if !is_runner {
        return;
    }
    const PR_SET_PDEATHSIG: i32 = 1;
    const SIGKILL: usize = 9;
    unsafe extern "C" {
        fn prctl(option: i32, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> i32;
    }
    // Ensure a Servo runner subprocess never survives as an orphan if the
    // parent nservo process exits or crashes.
    unsafe {
        prctl(PR_SET_PDEATHSIG, SIGKILL, 0, 0, 0);
    }
}

fn main() -> ExitCode {
    #[cfg(target_os = "linux")]
    arm_runner_parent_death_signal();

    // Must be the very first call in main(): servo-gtk re-executes this
    // binary as its Servo rendering subprocess.
    servo_gtk::run_as_runner_if_requested();

    // Store Servo session state (cookies, cache, webstorage, HSTS) under `$TMPDIR/nservo`.
    servo_gtk::set_config_dir(nservo::core::default_session_data_dir());
    // Apply configured HTTP/JS User-Agent (defaults to `NServo/<version>`).
    let startup_config = nservo::core::BrowserConfig::load_default();
    servo_gtk::set_user_agent(startup_config.user_agent);

    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Info);

    let epoxy_ok = match nservo::engine::epoxy::init() {
        Ok(()) => true,
        Err(err) => {
            log::warn!("OpenGL epoxy loader unavailable ({err}); will fall back to CPU");
            false
        }
    };
    let initial_backend = nservo::engine::gpu::configure_render_environment(epoxy_ok);

    let initial_url = std::env::args().nth(1);
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(move |app| {
        nservo::ui::style::install_theme();
        let start_url = initial_url.as_deref();
        nservo::ui::window::BrowserWindow::open(app, start_url, initial_backend);
    });

    app.run()
}
