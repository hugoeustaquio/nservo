use std::fs;
use std::path::{Path, PathBuf};

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.exists() {
        return;
    }
    for entry in fs::read_dir(dir).expect("failed to read directory") {
        let path = entry.expect("failed to read entry").path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Strip line comments (`//...`) so documentation mentions do not trigger false positives.
fn strip_line_comment(line: &str) -> &str {
    line.split("//").next().unwrap_or(line)
}

fn find_forbidden_references(dir: &Path, forbidden: &[&str]) -> Vec<String> {
    let mut files = Vec::new();
    collect_rust_files(dir, &mut files);
    assert!(!files.is_empty(), "expected Rust files in {}", dir.display());

    let mut violations = Vec::new();
    for file in files {
        let content = fs::read_to_string(&file).expect("failed to read source file");
        for (idx, line) in content.lines().enumerate() {
            let code = strip_line_comment(line);
            for pattern in forbidden {
                if code.contains(pattern) {
                    violations.push(format!(
                        "{}:{} references forbidden dependency `{}`",
                        file.display(),
                        idx + 1,
                        pattern
                    ));
                }
            }
        }
    }
    violations
}

#[test]
fn core_layer_is_decoupled_from_gtk_servo_engine_and_ui() {
    let core_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core");
    let forbidden = [
        "gtk::",
        "gdk::",
        "gio::",
        "glib::",
        "servo_gtk",
        "crate::ui",
        "crate::engine",
    ];
    let violations = find_forbidden_references(&core_dir, &forbidden);
    assert!(
        violations.is_empty(),
        "src/core must remain pure domain logic without UI/GTK/Servo dependencies:\n{}",
        violations.join("\n")
    );
}

#[test]
fn engine_layer_does_not_depend_on_ui() {
    let engine_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/engine");
    let violations = find_forbidden_references(&engine_dir, &["crate::ui"]);
    assert!(
        violations.is_empty(),
        "src/engine must not depend on src/ui:\n{}",
        violations.join("\n")
    );
}
