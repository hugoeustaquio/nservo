use std::ffi::c_void;
use std::ptr;

/// Initialize the `epoxy` OpenGL function loader required by `servo-gtk`.
#[cfg(unix)]
pub fn init() -> Result<(), String> {
    let library = unsafe { libloading::os::unix::Library::new("libepoxy.so.0") }
        .map_err(|err| format!("failed to load libepoxy.so.0: {err}"))?;

    // Intentionally leak the handle so GL function pointers stay valid for the
    // full lifetime of the process.
    let library: &'static libloading::os::unix::Library = Box::leak(Box::new(library));

    epoxy::load_with(|name| {
        unsafe { library.get::<*const c_void>(name.as_bytes()) }
            .map(|symbol| *symbol)
            .unwrap_or(ptr::null())
    });

    Ok(())
}

#[cfg(not(unix))]
pub fn init() -> Result<(), String> {
    Err("OpenGL epoxy bootstrap is only implemented on Unix platforms".to_string())
}
