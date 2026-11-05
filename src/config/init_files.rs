use crate::adblock::defaults::DEFAULT_ADS_LIST;
use crate::error::Result;
use crate::greasemonkey::defaults::DEFAULT_USER_SCRIPT;
use crate::platform::ConfigPaths;
use crate::plugins::pass::DEFAULT_PASS_PLUGIN_LUA;
use crate::plugins::readability::DEFAULT_READABILITY_PLUGIN_LUA;
use crate::security::PluginTrustStore;

pub struct ConfigInitializer;

impl ConfigInitializer {
    pub fn ensure_directories_and_files(paths: &ConfigPaths) -> Result<()> {
        let base = &paths.base_dir;
        let _ = std::fs::create_dir_all(base);
        let _ = std::fs::create_dir_all(paths.filters_dir());
        let _ = std::fs::create_dir_all(paths.plugins_dir().join("pass"));
        let _ = std::fs::create_dir_all(paths.plugins_dir().join("readability"));
        let _ = std::fs::create_dir_all(paths.greasemonkey_dir());

        // 1. Default init.lua
        if !paths.init_lua().exists() {
            let default_init = r#"-- NServo Main Configuration File
-- Inspired by Neovim and qutebrowser

set("smooth_scroll", true)
set("homepage", "https://servo.org")

bind("j", "scroll_down")
bind("k", "scroll_up")
bind("h", "scroll_left")
bind("l", "scroll_right")
bind("o", "open_url_current_tab")
bind("O", "open_url_new_tab")
bind("d", "close_tab")

command("gh", function()
    open("https://github.com")
end)
"#;
            let _ = std::fs::write(paths.init_lua(), default_init);
        }

        // 2. Default ads filter list
        if !paths.ads_file().exists() {
            let _ = std::fs::write(paths.ads_file(), DEFAULT_ADS_LIST);
        }

        // 3. Default Pass plugin
        let pass_init = paths.plugins_dir().join("pass").join("init.lua");
        if !pass_init.exists() {
            let _ = std::fs::write(&pass_init, DEFAULT_PASS_PLUGIN_LUA);
        }

        // 4. Default Readability plugin
        let read_init = paths.plugins_dir().join("readability").join("init.lua");
        if !read_init.exists() {
            let _ = std::fs::write(&read_init, DEFAULT_READABILITY_PLUGIN_LUA);
        }

        // 5. Default Greasemonkey script
        let gm_sample = paths.greasemonkey_dir().join("example.user.js");
        if !gm_sample.exists() {
            let _ = std::fs::write(&gm_sample, DEFAULT_USER_SCRIPT);
        }

        // 6. Pre-trust default bundled plugins
        if let Ok(mut trust_store) = PluginTrustStore::load_or_create(&paths.trusted_plugins_file()) {
            let _ = trust_store.trust_plugin("pass", &pass_init, DEFAULT_PASS_PLUGIN_LUA);
            let _ = trust_store.trust_plugin("readability", &read_init, DEFAULT_READABILITY_PLUGIN_LUA);
        }

        Ok(())
    }
}
