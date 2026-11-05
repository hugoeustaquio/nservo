use std::num::NonZeroU32;
use std::rc::Rc;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowBuilder;
use crate::app::event_handler::{AppEventHandler, GuiKey};
use crate::app::state::App;
use crate::error::Result;

pub struct GuiWindow;

impl GuiWindow {
    pub fn run(mut app: App) -> Result<()> {
        let event_loop = match EventLoop::new() {
            Ok(el) => el,
            Err(e) => {
                eprintln!("[NServo GUI] Notice: Could not connect to display server: {}", e);
                eprintln!("[NServo GUI] Running headless. Web core and modules are active.");
                return Ok(());
            }
        };

        let window = match WindowBuilder::new()
            .with_title("NServo Browser")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
            .build(&event_loop)
        {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("[NServo GUI] Notice: Window creation failed: {}", e);
                return Ok(());
            }
        };

        let context = match softbuffer::Context::new(window.clone()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[NServo GUI] Notice: Graphics context initialization failed: {}", e);
                return Ok(());
            }
        };

        let mut surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[NServo GUI] Notice: Surface creation failed: {}", e);
                return Ok(());
            }
        };

        event_loop.run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);

            if app.should_quit {
                elwt.exit();
                return;
            }

            match event {
                Event::WindowEvent { window_id, event } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => {
                            let _ = app.session.save_from_tabs(&app.tabs);
                            elwt.exit();
                        }
                        WindowEvent::Resized(size) => {
                            let w = size.width.max(320);
                            let h = size.height.max(240);
                            app.canvas.resize(w as usize, h as usize);
                            if let (Some(nw), Some(nh)) = (NonZeroU32::new(w), NonZeroU32::new(h)) {
                                let _ = surface.resize(nw, nh);
                            }
                            window.request_redraw();
                        }
                        WindowEvent::RedrawRequested => {
                            app.render_frame();
                            let size = window.inner_size();
                            if let (Some(_w), Some(_h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) {
                                if let Ok(mut buffer) = surface.buffer_mut() {
                                    let copy_len = buffer.len().min(app.canvas.buffer.len());
                                    buffer[..copy_len].copy_from_slice(&app.canvas.buffer[..copy_len]);
                                    let _ = buffer.present();
                                }
                            }
                        }
                        WindowEvent::KeyboardInput {
                            event: KeyEvent { state: ElementState::Pressed, logical_key, .. },
                            ..
                        } => {
                            if let Some(gui_key) = Self::map_key(&logical_key) {
                                let _ = AppEventHandler::handle_key(&mut app, gui_key);
                                if app.should_quit {
                                    elwt.exit();
                                } else {
                                    window.request_redraw();
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }).map_err(|e| crate::error::NServoError::Engine(e.to_string()))?;

        Ok(())
    }

    fn map_key(key: &Key) -> Option<GuiKey> {
        match key {
            Key::Named(NamedKey::Enter) => Some(GuiKey::Enter),
            Key::Named(NamedKey::Escape) => Some(GuiKey::Escape),
            Key::Named(NamedKey::Backspace) => Some(GuiKey::Backspace),
            Key::Character(s) => {
                if let Some(c) = s.chars().next() {
                    Some(GuiKey::Char(c))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
