use std::fs;
use std::path::{Path, PathBuf};

const MAX_LINES_PER_FILE: usize = 300;

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

#[test]
fn every_rust_source_file_is_at_most_300_lines() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_rust_files(&manifest_dir.join("src"), &mut files);
    collect_rust_files(&manifest_dir.join("tests"), &mut files);

    assert!(!files.is_empty(), "expected at least one Rust file");

    let mut violations = Vec::new();
    for file in files {
        let content = fs::read_to_string(&file).expect("failed to read source file");
        let line_count = content.lines().count();
        if line_count > MAX_LINES_PER_FILE {
            violations.push(format!(
                "{} has {} lines (max {})",
                file.display(),
                line_count,
                MAX_LINES_PER_FILE
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Files exceeding {MAX_LINES_PER_FILE} lines:\n{}",
        violations.join("\n")
    );
}
