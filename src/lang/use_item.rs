/// Item selection in a `use` statement.
#[derive(Debug, Clone, PartialEq)]
pub enum UseItem {
    /// Import a single word.
    Single(String),
    /// Import all words from a module.
    All,
}
