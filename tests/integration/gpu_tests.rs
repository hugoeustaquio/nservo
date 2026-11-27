use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nservo::engine::gpu::{RenderBackend, classify_gsk_renderer, has_drm_render_node_in};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dri_dir() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("nservo-dri-{pid}-{id}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dri dir");
    dir
}

#[test]
fn classifies_gtk_renderers_into_gpu_or_cpu() {
    assert_eq!(
        classify_gsk_renderer("GskVulkanRenderer"),
        RenderBackend::Gpu("Vulkan")
    );
    assert_eq!(
        classify_gsk_renderer("GskGLRenderer"),
        RenderBackend::Gpu("GL")
    );
    assert_eq!(
        classify_gsk_renderer("GskNglRenderer"),
        RenderBackend::Gpu("GL")
    );
    assert_eq!(
        classify_gsk_renderer("GskCairoRenderer"),
        RenderBackend::Cpu("Cairo")
    );
    assert_eq!(RenderBackend::Gpu("Vulkan").short_label(), "GPU:Vulkan");
    assert_eq!(RenderBackend::Cpu("Cairo").short_label(), "CPU:Cairo");
}

#[test]
fn detects_drm_render_nodes_in_directory() {
    let dir = temp_dri_dir();
    assert!(!has_drm_render_node_in(&dir));

    fs::write(dir.join("card0"), b"").expect("write card0");
    assert!(!has_drm_render_node_in(&dir));

    fs::write(dir.join("renderD128"), b"").expect("write renderD128");
    assert!(has_drm_render_node_in(&dir));

    let _ = fs::remove_dir_all(&dir);
}
