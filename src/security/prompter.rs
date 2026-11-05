use std::io::{self, Write};

pub struct SecurityPrompter;

impl SecurityPrompter {
    /// Formats the official security warning prompt.
    pub fn prompt_message(plugin_name: &str) -> String {
        format!(
            "Foi detectado um novo plugin: '{}'.\n\n\
            Plugins Lua possuem acesso ao sistema local e podem executar código arbitrário.\n\n\
            Deseja confiar neste plugin?\n\n\
            [y] Sim\n\
            [n] Não\n",
            plugin_name
        )
    }

    /// Asks the user via interactive terminal standard input.
    pub fn ask_user_confirmation(plugin_name: &str) -> bool {
        let msg = Self::prompt_message(plugin_name);
        println!("{}", msg);
        print!("> ");
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let answer = input.trim().to_lowercase();
            answer == "y" || answer == "s" || answer == "sim" || answer == "yes"
        } else {
            false
        }
    }
}
