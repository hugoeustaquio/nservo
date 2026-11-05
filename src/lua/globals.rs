use std::sync::{Arc, Mutex};
use mlua::{Function, Lua, Value};
use crate::error::Result;
use crate::lua::state::BrowserSharedState;

pub fn register_global_helpers(lua: &Lua, state: Arc<Mutex<BrowserSharedState>>) -> Result<()> {
    let globals = lua.globals();

    // set(key, val)
    let s_set = state.clone();
    let set_fn = lua.create_function(move |_lua, (key, val): (String, Value)| {
        if let Ok(mut st) = s_set.lock() {
            match key.as_str() {
                "smooth_scroll" => {
                    if let Value::Boolean(b) = val {
                        st.options.smooth_scroll = b;
                    }
                }
                "homepage" => {
                    if let Value::String(s) = val {
                        if let Ok(str_val) = s.to_str() {
                            st.options.homepage = str_val.to_string();
                        }
                    }
                }
                "adblock_enabled" => {
                    if let Value::Boolean(b) = val {
                        st.options.adblock_enabled = b;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    })?;
    globals.set("set", set_fn)?;

    // bind(key, action)
    let s_bind = state.clone();
    let bind_fn = lua.create_function(move |_lua, (key, action): (String, String)| {
        if let Ok(mut st) = s_bind.lock() {
            st.keybindings.bind(&key, &action);
        }
        Ok(())
    })?;
    globals.set("bind", bind_fn)?;

    // command(name, fn)
    let s_cmd = state.clone();
    let command_fn = lua.create_function(move |lua, (name, func): (String, Function)| {
        if let Ok(mut st) = s_cmd.lock() {
            if let Ok(reg_key) = lua.create_registry_value(func) {
                st.custom_commands.insert(name, reg_key);
            }
        }
        Ok(())
    })?;
    globals.set("command", command_fn)?;

    // open(url)
    let s_open = state.clone();
    let open_fn = lua.create_function(move |_lua, url: String| {
        if let Ok(mut st) = s_open.lock() {
            st.tab_actions.push(format!("open:{}", url));
        }
        Ok(())
    })?;
    globals.set("open", open_fn)?;

    Ok(())
}
