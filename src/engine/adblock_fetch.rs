use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::adblock::{AdblockFilter, filter_file_path, list_filter_files};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Result of downloading and saving a single ABP filter list.
#[derive(Debug, Clone)]
pub struct DownloadSummary {
    pub url: String,
    pub path: PathBuf,
    pub rule_count: usize,
}

/// Result of updating all saved ABP filter lists inside the adblock directory.
#[derive(Debug, Clone)]
pub struct UpdateSummary {
    pub total: usize,
    pub updated: usize,
    pub failed: Vec<(String, String)>,
    pub total_rules: usize,
}

/// Fetch `url` over HTTP(S) using `curl` (with `wget` fallback) and save it atomically
/// to `<adblock_dir>/<encoded-url>`.
pub fn download_filter_list(adblock_dir: &Path, url: &str) -> Result<DownloadSummary, String> {
    let body = fetch_url_text(url)?;
    save_filter_list(adblock_dir, url, &body)
}

/// Atomically persist downloaded filter list `content` for `url` into `adblock_dir`.
pub fn save_filter_list(
    adblock_dir: &Path,
    url: &str,
    content: &str,
) -> Result<DownloadSummary, String> {
    fs::create_dir_all(adblock_dir)
        .map_err(|e| format!("failed to create {}: {e}", adblock_dir.display()))?;

    let dest = filter_file_path(adblock_dir, url);
    let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = adblock_dir.join(format!(".tmp-{}-{seq}", std::process::id()));

    fs::write(&tmp, content.as_bytes())
        .map_err(|e| format!("failed to write {}: {e}", tmp.display()))?;
    if let Err(err) = fs::rename(&tmp, &dest) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("failed to save {}: {err}", dest.display()));
    }

    let mut filter = AdblockFilter::new();
    filter.add_list(content);
    Ok(DownloadSummary {
        url: url.to_string(),
        path: dest,
        rule_count: filter.rule_count(),
    })
}

/// Re-download all URL-backed filter files currently saved in `adblock_dir`.
pub fn update_all_filter_lists(adblock_dir: &Path) -> UpdateSummary {
    let urls: Vec<String> = list_filter_files(adblock_dir)
        .into_iter()
        .filter_map(|(_, url)| url)
        .collect();

    let total = urls.len();
    let mut updated = 0usize;
    let mut failed = Vec::new();
    for url in urls {
        match download_filter_list(adblock_dir, &url) {
            Ok(_) => updated += 1,
            Err(err) => failed.push((url, err)),
        }
    }

    let (engine, _) = AdblockFilter::load_from_dir(adblock_dir);
    UpdateSummary {
        total,
        updated,
        failed,
        total_rules: engine.rule_count(),
    }
}

fn fetch_url_text(url: &str) -> Result<String, String> {
    match run_fetch("curl", &["-fsSL", "--connect-timeout", "15", "--max-time", "60", url]) {
        Ok(text) => Ok(text),
        Err(FetchError::NotFound) => {
            run_fetch("wget", &["-qO-", "--timeout=30", url]).map_err(|e| e.into_message())
        }
        Err(err) => Err(err.into_message()),
    }
}

enum FetchError {
    NotFound,
    Message(String),
}

impl FetchError {
    fn into_message(self) -> String {
        match self {
            Self::NotFound => "neither 'curl' nor 'wget' was found in PATH".to_string(),
            Self::Message(msg) => msg,
        }
    }
}

fn run_fetch(program: &str, args: &[&str]) -> Result<String, FetchError> {
    let output = Command::new(program).args(args).output().map_err(|err| {
        if err.kind() == ErrorKind::NotFound {
            FetchError::NotFound
        } else {
            FetchError::Message(format!("failed to launch {program}: {err}"))
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            format!("{program} exited with status {}", output.status)
        } else {
            stderr
        };
        return Err(FetchError::Message(detail));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
