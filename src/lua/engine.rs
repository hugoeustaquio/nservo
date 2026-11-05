use std::sync::{Arc, Mutex};
use mlua::Lua;
use crate::error::Result;
use crate::lua::bindings_nservo::register_nservo_core;
use crate::lua::bindings_page::register_nservo_page;
use crate::lua::bindings_storage::register_nservo_storage;
use crate::lua::bindings_tabs::register_nservo_tabs;
use crate::lua::globals::register_global_helpers;
use crate::lua::state::BrowserSharedState;

pub struct LuaRuntime {
    lua: Lua,
    state: Arc<Mutex<BrowserSharedState>>,
}

impl LuaRuntime {
    pub fn new(state: Arc<Mutex<BrowserSharedState>>) -> Result<Self> {
        let lua = Lua::new();

        // 1. Create global `nservo` table
        let nservo_table = lua.create_table()?;
        register_nservo_core(&lua, &nservo_table, state.clone())?;
        register_nservo_page(&lua, &nservo_table, state.clone())?;
        register_nservo_tabs(&lua, &nservo_table, state.clone())?;
        register_nservo_storage(&lua, &nservo_table, state.clone())?;
        lua.globals().set("nservo", nservo_table)?;

        // 2. Register global shorthand helpers: set(), bind(), command(), open()
        register_global_helpers(&lua, state.clone())?;

        Ok(Self { lua, state })
    }

    pub fn execute_file(&self, path: &std::path::Path) -> Result<()> {
        if path.exists() {
            let script = std::fs::read_to_string(path)?;
            self.lua.load(&script).exec()?;
        }
        Ok(())
    }

    pub fn execute_string(&self, code: &str) -> Result<()> {
        self.lua.load(code).exec()?;
        Ok(())
    }

    pub fn state(&self) -> Arc<Mutex<BrowserSharedState>> {
        self.state.clone()
    }
}
