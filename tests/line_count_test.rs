use std::fs;
use std::path::Path;

fn check_dir(dir: &Path, violations: &mut Vec<(String, usize)>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                check_dir(&path, violations);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(content) = fs::read_to_string(&path) {
                    let lines = content.lines().count();
                    if lines > 200 {
                        violations.push((path.display().to_string(), lines));
                    }
                }
            }
        }
    }
}

#[test]
fn test_no_file_exceeds_200_lines() {
    let mut violations = Vec::new();
    let src_dir = Path::new("src");
    check_dir(src_dir, &mut violations);

    if !violations.is_empty() {
        let msg = violations
            .iter()
            .map(|(p, l)| format!("{}: {} lines", p, l))
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "Architecture rule violation! The following files exceed 200 lines:\n{}",
            msg
        );
    }
}
