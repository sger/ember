use rkyv::{Archive, Deserialize, Serialize};

/// Item selection in a `use` statement.
#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
#[archive(check_bytes)]
pub enum UseItem {
    /// Import a single word.
    Single(String),
    /// Import all words from a module.
    All,
}
