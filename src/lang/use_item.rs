use serde::{Deserialize, Serialize};

/// Item selection in a `use` statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UseItem {
    /// Import a single word.
    Single(String),
    /// Import all words from a module.
    All,
}
