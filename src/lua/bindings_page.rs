use std::sync::{Arc, Mutex};
use mlua::{Lua, Table};
use crate::error::Result;
use crate::lua::state::BrowserSharedState;

pub fn register_nservo_page(lua: &Lua, nservo: &Table, state: Arc<Mutex<BrowserSharedState>>) -> Result<()> {
    let page_table = lua.create_table()?;

    // nservo.page.url()
    let s_url = state.clone();
    let url_fn = lua.create_function(move |_lua, ()| {
        let url = s_url.lock().map(|st| st.current_url.clone()).unwrap_or_default();
        Ok(url)
    })?;
    page_table.set("url", url_fn)?;

    // nservo.page.title()
    let s_title = state.clone();
    let title_fn = lua.create_function(move |_lua, ()| {
        let title = s_title.lock().map(|st| st.current_title.clone()).unwrap_or_default();
        Ok(title)
    })?;
    page_table.set("title", title_fn)?;

    // nservo.page.html()
    let s_html = state.clone();
    let html_fn = lua.create_function(move |_lua, ()| {
        let html = s_html.lock().map(|st| st.current_html.clone()).unwrap_or_default();
        Ok(html)
    })?;
    page_table.set("html", html_fn)?;

    // nservo.page.eval_js(script)
    let s_eval = state.clone();
    let eval_fn = lua.create_function(move |_lua, script: String| {
        if let Ok(mut st) = s_eval.lock() {
            st.pending_js_eval = Some(script);
        }
        Ok("true".to_string())
    })?;
    page_table.set("eval_js", eval_fn)?;

    nservo.set("page", page_table)?;
    Ok(())
}
