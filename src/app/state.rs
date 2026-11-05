use std::sync::{Arc, Mutex};
use crate::adblock::AdblockEngine;
use crate::bookmarks::BookmarksDatabase;
use crate::command::AutocompleteEngine;
use crate::config::{ConfigInitializer, ConfigOptions, KeybindingMap};
use crate::error::Result;
use crate::greasemonkey::GreasemonkeyManager;
use crate::history::HistoryDatabase;
use crate::lua::{BrowserSharedState, LuaRuntime};
use crate::modes::ModeStateMachine;
use crate::platform::ConfigPaths;
use crate::plugins::PluginManager;
use crate::security::PluginTrustStore;
use crate::session::SessionStorage;
use crate::tabs::TabManager;
use crate::ui::{Canvas, GuiRenderer};

pub struct App {
    pub paths: ConfigPaths,
    pub tabs: TabManager,
    pub modes: ModeStateMachine,
    pub options: ConfigOptions,
    pub keybindings: KeybindingMap,
    pub history: Arc<Mutex<HistoryDatabase>>,
    pub bookmarks: Arc<Mutex<BookmarksDatabase>>,
    pub adblock: AdblockEngine,
    pub greasemonkey: GreasemonkeyManager,
    pub plugins: PluginManager,
    pub trust_store: PluginTrustStore,
    pub session: SessionStorage,
    pub lua: LuaRuntime,
    pub canvas: Canvas,
    pub cmd_buffer: String,
    pub suggestions: Vec<String>,
    pub status_message: Option<String>,
    pub confirm_modal: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(paths: ConfigPaths) -> Result<Self> {
        ConfigInitializer::ensure_directories_and_files(&paths)?;

        let history_db = Arc::new(Mutex::new(HistoryDatabase::open(&paths.history_db())?));
        let bookmarks_db = Arc::new(Mutex::new(BookmarksDatabase::open(&paths.bookmarks_db())?));
        let adblock = AdblockEngine::load_from_file(&paths.ads_file())?;
        let mut greasemonkey = GreasemonkeyManager::new();
        let _ = greasemonkey.load_directory(&paths.greasemonkey_dir());

        let trust_store = PluginTrustStore::load_or_create(&paths.trusted_plugins_file())?;
        let mut plugins = PluginManager::new(paths.plugins_dir());
        let _ = plugins.discover(&trust_store);

        let session = SessionStorage::new(paths.session_file());

        let shared_state = Arc::new(Mutex::new(BrowserSharedState::new()));
        {
            let mut st = shared_state.lock().unwrap();
            st.history = Some(history_db.clone());
            st.bookmarks = Some(bookmarks_db.clone());
        }

        let lua = LuaRuntime::new(shared_state)?;
        let _ = lua.execute_file(&paths.init_lua());

        let mut app = Self {
            paths,
            tabs: TabManager::new(),
            modes: ModeStateMachine::new(),
            options: ConfigOptions::new(),
            keybindings: KeybindingMap::new(),
            history: history_db,
            bookmarks: bookmarks_db,
            adblock,
            greasemonkey,
            plugins,
            trust_store,
            session,
            lua,
            canvas: Canvas::new(1280, 720),
            cmd_buffer: String::new(),
            suggestions: Vec::new(),
            status_message: None,
            confirm_modal: None,
            should_quit: false,
        };

        if !app.session.restore_into_tabs(&mut app.tabs)? {
            app.tabs.open_tab(&app.options.homepage)?;
        }

        Ok(app)
    }

    pub fn update_suggestions(&mut self) {
        let hist_lock = self.history.lock().ok();
        let bm_lock = self.bookmarks.lock().ok();
        self.suggestions = AutocompleteEngine::suggest(
            &self.cmd_buffer,
            hist_lock.as_deref(),
            bm_lock.as_deref(),
        );
    }

    pub fn render_frame(&mut self) {
        GuiRenderer::render_frame(
            &mut self.canvas,
            &self.tabs,
            self.modes.current(),
            &self.cmd_buffer,
            &self.suggestions,
            self.status_message.as_deref(),
            self.confirm_modal.as_deref(),
        );
    }
}
