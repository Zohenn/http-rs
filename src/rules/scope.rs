use crate::rules::value::Type;
use std::collections::HashMap;

#[derive(Default)]
pub struct RuleScope {
    vars: HashMap<String, Type>,
}

impl RuleScope {
    pub fn new() -> Self {
        RuleScope {
            vars: HashMap::new(),
        }
    }

    pub fn get_var(&self, ident: &str) -> Option<&Type> {
        self.vars.get(ident)
    }

    pub fn update_var(&mut self, ident: &str, value: Type) {
        self.vars.insert(ident.to_owned(), value);
    }
}
