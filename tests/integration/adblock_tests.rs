use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use nservo::core::adblock::{
    AdblockFilter, TYPE_DOCUMENT, TYPE_IMAGE, TYPE_SCRIPT, decode_filter_filename,
    destination_to_type_mask, encode_filter_filename, filter_file_path, list_filter_files,
};
use nservo::core::{
    BrowserAction, CommandError, CompletionSources, complete_command_input, parse_command,
    resolve_adblock_dir,
};
use nservo::engine::adblock_fetch::{
    download_filter_list, save_filter_list, update_all_filter_lists,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("nservo-adblock-{label}-{}-{n}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn parse_adblock_commands_and_completion() {
    assert_eq!(
        parse_command(":adblock https://easylist.to/easylist/easylist.txt"),
        Ok(BrowserAction::AddAdblockList(
            "https://easylist.to/easylist/easylist.txt".to_string()
        ))
    );
    assert_eq!(
        parse_command(":adblock easylist.to/easylist/easylist.txt"),
        Ok(BrowserAction::AddAdblockList(
            "https://easylist.to/easylist/easylist.txt".to_string()
        ))
    );
    assert_eq!(
        parse_command(":adblock"),
        Err(CommandError::MissingArgument(":adblock <URL>"))
    );
    assert!(matches!(
        parse_command(":adblock not a valid url"),
        Err(CommandError::InvalidArgument(_))
    ));
    assert_eq!(
        parse_command(":adblock-update"),
        Ok(BrowserAction::UpdateAdblockLists)
    );

    let items = complete_command_input("adbl", &CompletionSources::default());
    let names: Vec<&str> = items.iter().map(|i| i.primary.as_str()).collect();
    assert!(names.contains(&"adblock"));
    assert!(names.contains(&"adblock-update"));
}

#[test]
fn resolves_xdg_adblock_dir_and_roundtrips_filter_filenames() {
    assert_eq!(
        resolve_adblock_dir(Some("/tmp/xdg"), Some("/home/u")),
        PathBuf::from("/tmp/xdg/nservo/adblock")
    );
    assert_eq!(
        resolve_adblock_dir(None, Some("/home/u")),
        PathBuf::from("/home/u/.config/nservo/adblock")
    );

    let url = "https://easylist.to/easylist/easylist.txt?v=1&x=a%20b";
    let encoded = encode_filter_filename(url);
    assert!(!encoded.contains('/'));
    assert_eq!(decode_filter_filename(&encoded).as_deref(), Some(url));
    assert_eq!(decode_filter_filename("manual-list.txt"), None);
}

#[test]
fn abp_filter_blocks_domains_patterns_options_and_respects_allowlists() {
    let mut filter = AdblockFilter::new();
    filter.add_list(
        "\
[Adblock Plus 2.0]
! EasyList-style sample
||doubleclick.net^
@@||safe.doubleclick.net^
||tracker.net^$third-party
/ads/banner*.js$script
||metrics.example.org/collect^$domain=news.com|~exempt.news.com
0.0.0.0 hosts-ad.example.net
||unsupported.example.net^$object
@@||google.com^$generichide
example.com#%#//scriptlet('abort-on-property-read')
##.ad-banner
##div:-abp-has(.sponsored)
",
    );

    // Pure domain rule + subdomain + boundary safety.
    assert!(filter.should_block("https://doubleclick.net/ad.js", None, TYPE_SCRIPT));
    assert!(filter.should_block("https://pubads.doubleclick.net/gampad", None, TYPE_SCRIPT));
    assert!(!filter.should_block("https://notdoubleclick.net/ok.js", None, TYPE_SCRIPT));
    assert!(!filter.should_block("https://doubleclick.net.other.org/ok.js", None, TYPE_SCRIPT));

    // Exception rule overrides block rule.
    assert!(!filter.should_block("https://safe.doubleclick.net/allowed.js", None, TYPE_SCRIPT));

    // Third-party rule only blocks cross-site requests.
    assert!(filter.should_block(
        "https://tracker.net/p.js",
        Some("https://blog.org/post"),
        TYPE_SCRIPT,
    ));
    assert!(!filter.should_block(
        "https://tracker.net/p.js",
        Some("https://www.tracker.net/home"),
        TYPE_SCRIPT,
    ));

    // Wildcard + resource type option.
    assert!(filter.should_block("https://site.com/ads/banner_top.js", None, TYPE_SCRIPT));
    assert!(!filter.should_block("https://site.com/ads/banner_top.js", None, TYPE_IMAGE));

    // Domain option with inclusion & exclusion.
    assert!(filter.should_block(
        "https://metrics.example.org/collect?id=1",
        Some("https://www.news.com/story"),
        TYPE_SCRIPT,
    ));
    assert!(!filter.should_block(
        "https://metrics.example.org/collect?id=1",
        Some("https://exempt.news.com/story"),
        TYPE_SCRIPT,
    ));
    assert!(!filter.should_block(
        "https://metrics.example.org/collect?id=1",
        Some("https://other.org/"),
        TYPE_SCRIPT,
    ));

    // Hosts-file entry.
    assert!(filter.should_block("https://hosts-ad.example.net/pixel.gif", None, TYPE_IMAGE));

    // Rules with unsupported positive options or #%# scriptlets are ignored safely.
    assert!(!filter.should_block("https://unsupported.example.net/app.js", None, TYPE_SCRIPT));
    assert!(!filter.should_block("https://example.com/page.js", None, TYPE_SCRIPT));

    // Internal schemes are never blocked.
    assert!(!filter.should_block("about:blank", None, TYPE_DOCUMENT));
    assert!(!filter.should_block("data:text/html,<h1>Hi</h1>", None, TYPE_DOCUMENT));

    // Cosmetic CSS keeps valid selectors and skips procedural :-abp- extensions.
    let css = filter.cosmetic_css(100);
    assert!(css.contains(".ad-banner { display: none !important; }"));
    assert!(!css.contains(":-abp-has"));
    assert_eq!(destination_to_type_mask("Script"), TYPE_SCRIPT);
    assert_eq!(destination_to_type_mask("Image"), TYPE_IMAGE);
}

#[test]
fn loads_multiple_lists_from_adblock_dir_and_skips_hidden_files() {
    let dir = temp_dir("load");
    let url = "https://lists.example.com/custom.txt";
    let saved = save_filter_list(&dir, url, "||ads.alpha.test^\n").unwrap();
    assert_eq!(saved.path, filter_file_path(&dir, url));
    assert_eq!(saved.rule_count, 1);

    fs::write(dir.join("local.txt"), "||ads.beta.test^\n").unwrap();
    fs::write(dir.join(".tmp-ignored"), "||should-not-load.test^\n").unwrap();

    let files = list_filter_files(&dir);
    assert_eq!(files.len(), 2);

    let (engine, count) = AdblockFilter::load_from_dir(&dir);
    assert_eq!(count, 2);
    assert!(engine.should_block("https://ads.alpha.test/x.js", None, TYPE_SCRIPT));
    assert!(engine.should_block("https://ads.beta.test/y.js", None, TYPE_SCRIPT));
    assert!(!engine.should_block("https://should-not-load.test/z.js", None, TYPE_SCRIPT));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn download_and_update_filter_lists_over_local_http() {
    let Ok(listener) = TcpListener::bind("127.0.0.1:0") else {
        return;
    };
    let addr = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let payloads = [
            "! v1\n||v1.ads.test^\n",
            "! v2\n||v1.ads.test^\n||v2.ads.test^\n",
        ];
        for body in payloads {
            let Ok((mut stream, _)) = listener.accept() else { break; };
            let mut buf = [0u8; 512];
            let _ = stream.read(&mut buf);
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });

    let dir = temp_dir("http");
    let url = format!("http://{addr}/easylist.txt");

    let first = download_filter_list(&dir, &url).unwrap();
    assert_eq!(first.rule_count, 1);

    let updated = update_all_filter_lists(&dir);
    assert_eq!(updated.total, 1);
    assert_eq!(updated.updated, 1);
    assert!(updated.failed.is_empty());
    assert_eq!(updated.total_rules, 2);

    let (engine, _) = AdblockFilter::load_from_dir(&dir);
    assert!(engine.should_block("https://v2.ads.test/a.js", None, TYPE_SCRIPT));

    let _ = server.join();
    let _ = fs::remove_dir_all(&dir);
}
