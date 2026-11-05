/// Lua source code for the built-in `readability` reading mode plugin.
pub const DEFAULT_READABILITY_PLUGIN_LUA: &str = r#"-- NServo Default Plugin: readability
-- Distills article content into a clean, distraction-free reading view.

local plugin = {}

function plugin.run()
    nservo.notify("[readability] Activating reader mode...")

    local clean_js = [[
        (() => {
            // Remove noisy elements
            const noisySelectors = [
                'nav', 'header', 'footer', 'aside', '.sidebar', '.ads', '.ad-banner',
                '.cookie-banner', '.popup', '#comments', '.social-share'
            ];
            noisySelectors.forEach(sel => {
                document.querySelectorAll(sel).forEach(el => el.remove());
            });

            // Apply e-book clean typography and styling
            const style = document.createElement('style');
            style.id = 'nservo-readability-style';
            style.textContent = `
                body {
                    max-width: 760px !important;
                    margin: 40px auto !important;
                    padding: 0 20px !important;
                    font-family: 'Merriweather', 'Georgia', serif !important;
                    font-size: 19px !important;
                    line-height: 1.7 !important;
                    color: #2b2b2b !important;
                    background-color: #fbf0d9 !important; /* Solarized light sepia */
                }
                h1, h2, h3 { font-family: sans-serif; line-height: 1.3; }
                img { max-width: 100%; height: auto; border-radius: 4px; }
            `;
            document.head.appendChild(style);
            return "Reader mode applied";
        })();
    ]]

    local res = nservo.page.eval_js(clean_js)
    nservo.notify("[readability] Distillation complete")
end

nservo.command("readability", function()
    plugin.run()
end)

return plugin
"#;
