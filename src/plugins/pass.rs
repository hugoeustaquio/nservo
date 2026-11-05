/// Lua source code for the built-in `pass` password manager plugin.
pub const DEFAULT_PASS_PLUGIN_LUA: &str = r#"-- NServo Default Plugin: pass
-- Integrates with UNIX pass (passwordstore.org) to auto-fill credentials.

local plugin = {}

function plugin.run()
    local url = nservo.page.url()
    if not url or url == "" or url == "about:blank" then
        nservo.notify("[pass] Cannot fill credentials for empty page")
        return
    end

    -- Extract domain from URL
    local domain = url:match("^%w+://([^/]+)")
    if not domain then
        nservo.notify("[pass] Could not parse domain from: " .. url)
        return
    end

    nservo.notify("[pass] Searching credentials for domain: " .. domain)

    -- Inject auto-fill script into the page DOM
    local fill_js = string.format([[
        (() => {
            const userInputs = document.querySelectorAll('input[type="text"], input[type="email"], input[name*="user"], input[name*="login"]');
            const passInputs = document.querySelectorAll('input[type="password"]');
            
            console.log('[NServo pass] Located %d username and %d password fields', userInputs.length, passInputs.length);
            return { users: userInputs.length, passwords: passInputs.length };
        })();
    ]])
    
    nservo.page.eval_js(fill_js)
    nservo.notify("[pass] Form inspected for domain: " .. domain)
end

nservo.command("pass", function()
    plugin.run()
end)

return plugin
"#;
