use std::sync::{Arc, Mutex};
use mlua::{Function, Lua, Table};
use crate::error::Result;
use crate::lua::state::BrowserSharedState;

pub fn register_nservo_core(lua: &Lua, nservo: &Table, state: Arc<Mutex<BrowserSharedState>>) -> Result<()> {
    // nservo.notify(msg)
    let s_notify = state.clone();
    let notify_fn = lua.create_function(move |_lua, msg: String| {
        if let Ok(mut st) = s_notify.lock() {
            st.notifications.push(msg);
        }
        Ok(())
    })?;
    nservo.set("notify", notify_fn)?;

    // nservo.bind(key, action)
    let s_bind = state.clone();
    let bind_fn = lua.create_function(move |_lua, (key, action): (String, String)| {
        if let Ok(mut st) = s_bind.lock() {
            st.keybindings.bind(&key, &action);
        }
        Ok(())
    })?;
    nservo.set("bind", bind_fn)?;

    // nservo.command(name, callback)
    let s_cmd = state.clone();
    let cmd_fn = lua.create_function(move |lua, (name, func): (String, Function)| {
        if let Ok(mut st) = s_cmd.lock() {
            if let Ok(reg_key) = lua.create_registry_value(func) {
                st.custom_commands.insert(name, reg_key);
            }
        }
        Ok(())
    })?;
    nservo.set("command", cmd_fn)?;

    Ok(())
}
