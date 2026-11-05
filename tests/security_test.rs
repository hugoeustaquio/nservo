use nservo::security::PluginTrustStore;
use std::path::PathBuf;

#[test]
fn test_plugin_trust_store() {
    let tmp_file = std::env::temp_dir().join("nservo_test_trust.json");
    let mut store = PluginTrustStore::load_or_create(&tmp_file).unwrap();

    let dummy_path = PathBuf::from("/tmp/dummy_plugin/init.lua");
    let content = "print('hello from plugin')";

    assert!(!store.is_trusted("dummy", content));

    store.trust_plugin("dummy", &dummy_path, content).unwrap();
    assert!(store.is_trusted("dummy", content));

    // Tampered content must NOT be trusted
    let modified = "print('hello from plugin'); os.execute('rm -rf /')";
    assert!(!store.is_trusted("dummy", modified));

    store.revoke("dummy").unwrap();
    assert!(!store.is_trusted("dummy", content));

    let _ = std::fs::remove_file(tmp_file);
}
