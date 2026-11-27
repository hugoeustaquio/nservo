pub mod action;
pub mod adblock;
mod adblock_match;
pub mod bookmarks;
pub mod command;
pub mod completion;
pub mod config;
pub mod debugger;
pub mod hints;
pub mod history;
pub mod inspector;
pub mod keymap;
pub mod load_state;
pub mod mode;
pub mod plugins;
pub mod scroll_state;
pub mod search;
pub mod session;
pub mod storage;
pub mod tab_model;
pub mod text_input;

pub use action::{BrowserAction, ZOOM_STEP, step_zoom};
pub use adblock::{
    AdblockFilter, decode_filter_filename, destination_to_type_mask, encode_filter_filename,
    filter_file_path, list_filter_files,
};
pub use bookmarks::{AddBookmarkOutcome, Bookmark, BookmarkError, BookmarkStore};
pub use command::{CommandError, normalize_url, parse_command};
pub use completion::{
    CompletionItem, CompletionKind, CompletionSources, MAX_COMPLETIONS, complete_command_input,
    fuzzy_score,
};
pub use config::{BrowserConfig, ConfigEvent, SHORTCUT_SPECS, ShortcutSpec, parse_config_message};
pub use debugger::{
    CallFrame, DebuggerEvent, ScopeBinding, ScriptSource, StepAction, encode_rdp_packet,
    format_scope_bindings, format_source_with_lines, parse_environment_bindings,
    parse_frames_array, parse_sources_array, try_decode_rdp_packet,
};
pub use hints::{HintEvent, generate_hint_labels, parse_hint_message};
pub use history::{
    HistoryEntry, HistoryError, HistoryStore, HistorySync, is_recordable_history_url,
};
pub use inspector::{
    ConsoleEntry, ConsoleLevel, ConsoleStore, DomNodeSummary, ElementDetails, InspectorEvent,
    parse_inspector_message,
};
pub use keymap::{
    COUNT_SEQUENCE_TIMEOUT, KEY_SEQUENCE_TIMEOUT, KeyInput, KeyMapper, KeyOutcome, MAX_KEY_COUNT,
};
pub use load_state::{LoadCoordinator, LoadTarget};
pub use mode::Mode;
pub use plugins::{
    DEFAULT_EXEC_TIMEOUT_MS, InstalledPlugin, PLUGIN_MANIFEST_FILENAME, PluginAction,
    PluginManifest, PluginMode, discover_plugins, extract_origin, load_plugin_from_dir,
    normalize_plugin_repo_url, parse_plugin_actions, validate_plugin_name,
};
pub use scroll_state::{ScrollMetrics, parse_scroll_metrics};
pub use search::{SearchEvent, SearchState, format_search_status, parse_search_message};
pub use session::{
    DEFAULT_SESSION_NAME, SavedTab, Session, SessionError, SessionStore, validate_session_name,
};
pub use storage::{
    ADBLOCK_DIRNAME, CONFIG_FILENAME, DB_FILENAME, PLUGINS_DIRNAME, STARTPAGE_FILENAME,
    STYLE_FILENAME, StorageError, default_adblock_dir, default_config_path, default_db_path,
    default_plugins_dir, default_session_data_dir, default_startpage_path, default_style_path,
    resolve_adblock_dir, resolve_config_dir, resolve_config_path, resolve_db_path,
    resolve_plugins_dir, resolve_session_data_dir, resolve_startpage_path, resolve_style_path,
};
pub use tab_model::{ClosedTab, TabInfo, TabModel};
pub use text_input::{
    DeadKey, DeadKeyComposer, LineEdge, WordDirection, compose_dead_key, decode_utf16_units,
    encode_utf16_units, needs_utf16_injection,
};
