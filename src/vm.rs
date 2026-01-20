use crate::ast::{Node, Program, UseItem, Value};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::runtime_error::RuntimeError;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

pub struct VM {
    stack: Vec<Value>,
    // "word" or "Module.word" -> body
    words: HashMap<String, Vec<Node>>,
    // "word" -> "Module.word" (from 'use')
    aliases: HashMap<String, String>,
    imported: HashSet<PathBuf>,
    current_dir: Option<PathBuf>,
    // Print full abstract syntax tree
    imported_programs: Vec<(PathBuf, Program)>,
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            words: HashMap::new(),
            aliases: HashMap::new(),
            imported: HashSet::new(),
            current_dir: None,
            imported_programs: Vec::new(),
        }
    }

    pub fn set_current_dir(&mut self, path: &Path) {
        self.current_dir = if path.is_dir() {
            Some(path.to_path_buf())
        } else {
            path.parent().map(|p| p.to_path_buf())
        }
    }

    pub fn load(&mut self, program: &Program) -> Result<(), RuntimeError> {
        for def in &program.definitions {
            self.process_definition(def)?;
        }
        Ok(())
    }

    fn process_definition(&mut self, def: &Node) -> Result<(), RuntimeError> {
        match def {
            Node::Def { name, body } => {
                self.words.insert(name.clone(), body.clone());
            }
            Node::Import(path) => {
                self.import_file(path)?;
            }
            Node::Module { name, definitions } => {
                self.register_module(name, definitions)?;
            }
            Node::Use { module, item } => {
                self.handle_use(module, item)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn import_file(&mut self, path: &str) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn register_module(
        &mut self,
        module_name: &str,
        definitions: &[Node],
    ) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn handle_use(&mut self, module: &str, item: &UseItem) -> Result<(), RuntimeError> {
        Ok(())
    }
}
