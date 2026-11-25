use std::fs;
use std::path::Path;

use gtk::prelude::*;

const DRI_DIR: &str = "/dev/dri";

/// Active graphics backend used by the GTK4 compositor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackend {
    Gpu(&'static str),
    Cpu(&'static str),
}

impl RenderBackend {
    /// Compact label displayed in the statusline (e.g. `GPU:Vulkan`, `CPU:Cairo`).
    pub fn short_label(self) -> String {
        match self {
            Self::Gpu(driver) => format!("GPU:{driver}"),
            Self::Cpu(driver) => format!("CPU:{driver}"),
        }
    }
}

/// Returns `true` if Linux exposes at least one DRM render node (`/dev/dri/renderD*`).
pub fn has_drm_render_node() -> bool {
    has_drm_render_node_in(Path::new(DRI_DIR))
}

/// Testable helper checking `dir` for any `renderD*` DRM render node entry.
pub fn has_drm_render_node_in(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .any(|name| name.starts_with("renderD"))
}

/// Classify a GTK4 `GskRenderer` GType name into a [`RenderBackend`].
pub fn classify_gsk_renderer(type_name: &str) -> RenderBackend {
    let lower = type_name.to_ascii_lowercase();
    if lower.contains("vulkan") {
        RenderBackend::Gpu("Vulkan")
    } else if lower.contains("gl") {
        RenderBackend::Gpu("GL")
    } else if lower.contains("cairo") {
        RenderBackend::Cpu("Cairo")
    } else {
        RenderBackend::Gpu("Auto")
    }
}

/// Configure GPU-first rendering with automatic CPU fallback before GTK initializes.
///
/// Must be called early in `main()` while the process is still single-threaded.
/// Respects explicit `GSK_RENDERER` / `LIBGL_ALWAYS_SOFTWARE` overrides.
pub fn configure_render_environment(epoxy_ok: bool) -> RenderBackend {
    let explicit_renderer = std::env::var("GSK_RENDERER").ok();
    let forced_sw = std::env::var("LIBGL_ALWAYS_SOFTWARE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if let Some(ref renderer) = explicit_renderer {
        let backend = classify_gsk_renderer(renderer);
        log::info!("Using user-configured GSK_RENDERER={renderer} ({backend:?})");
        return backend;
    }

    if epoxy_ok && has_drm_render_node() && !forced_sw {
        // Leave GSK_RENDERER unset so GTK4 tries GPU backends (Vulkan -> GL/NGL)
        // and automatically falls back to Cairo (CPU) if GPU init fails.
        log::info!("GPU render node detected; preferring hardware GPU with CPU fallback");
        RenderBackend::Gpu("Auto")
    } else {
        log::warn!(
            "Hardware GPU unavailable (epoxy_ok={epoxy_ok}, forced_sw={forced_sw}); falling back to CPU renderer"
        );
        // SAFETY: invoked at process startup before spawning background threads.
        unsafe {
            std::env::set_var("GSK_RENDERER", "cairo");
            std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
        }
        RenderBackend::Cpu("Cairo")
    }
}

/// Observe the actual `GskRenderer` selected once `window` realizes and notify `on_detected`.
pub fn watch_window_backend(
    window: &gtk::ApplicationWindow,
    on_detected: impl Fn(RenderBackend) + 'static,
) {
    window.connect_realize(move |win| {
        if let Some(renderer) = win.renderer() {
            let type_name = renderer.type_().name();
            let backend = classify_gsk_renderer(type_name);
            log::info!(
                "Active GTK4 renderer: {type_name} ({})",
                backend.short_label()
            );
            on_detected(backend);
        }
    });
}
