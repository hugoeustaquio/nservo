use std::sync::{Arc, Mutex};
use mlua::{Lua, Table};
use crate::error::Result;
use crate::lua::state::BrowserSharedState;

pub fn register_nservo_storage(lua: &Lua, nservo: &Table, state: Arc<Mutex<BrowserSharedState>>) -> Result<()> {
    // Bookmarks table
    let bm_table = lua.create_table()?;
    let s_bm = state.clone();
    let add_bm_fn = lua.create_function(move |_lua, (url, title, tags): (String, String, Option<Vec<String>>)| {
        if let Ok(st) = s_bm.lock() {
            if let Some(ref bm_db) = st.bookmarks {
                if let Ok(db) = bm_db.lock() {
                    let _ = db.add(&url, &title, &tags.unwrap_or_default());
                }
            }
        }
        Ok(())
    })?;
    bm_table.set("add", add_bm_fn)?;
    nservo.set("bookmarks", bm_table)?;

    // History table
    let hist_table = lua.create_table()?;
    let s_hist = state.clone();
    let search_hist_fn = lua.create_function(move |lua, query: String| {
        let results_table = lua.create_table()?;
        if let Ok(st) = s_hist.lock() {
            if let Some(ref hist_db) = st.history {
                if let Ok(db) = hist_db.lock() {
                    if let Ok(matches) = db.search(&query, 20) {
                        for (i, entry) in matches.iter().enumerate() {
                            let entry_t = lua.create_table()?;
                            entry_t.set("url", entry.url.clone())?;
                            entry_t.set("title", entry.title.clone())?;
                            entry_t.set("timestamp", entry.timestamp)?;
                            entry_t.set("visits", entry.visit_count)?;
                            results_table.set(i + 1, entry_t)?;
                        }
                    }
                }
            }
        }
        Ok(results_table)
    })?;
    hist_table.set("search", search_hist_fn)?;
    nservo.set("history", hist_table)?;

    Ok(())
}
