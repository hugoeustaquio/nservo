use nservo::app::{App, GuiWindow};
use nservo::platform::ConfigPaths;

fn main() -> nservo::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-v") {
        println!("NServo 0.1.0 (Servo engine, wgpu/WebRender, Graphical GUI)");
        return Ok(());
    }

    let paths = ConfigPaths::new();
    let mut app = App::new(paths)?;

    // If a URL was passed on the command line, open it in active/new tab
    if args.len() > 1 && !args[1].starts_with('-') {
        let _ = app.tabs.open_tab(&args[1]);
    }

    // Launch in native graphical desktop window
    GuiWindow::run(app)
}
