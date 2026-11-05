#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandAction {
    Open(String),
    OpenNewTab(String),
    TabNew(Option<String>),
    TabNext,
    TabPrev,
    TabClose,
    Tabs,
    WriteQuit,
    Quit,
    Plugin(String),
    Bookmark,
    Clean,
    Custom(String, Vec<String>),
}
