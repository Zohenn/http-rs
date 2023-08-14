use crate::rules::expr::Value;
use std::collections::HashMap;

pub struct RuleScope<'a> {
    vars: HashMap<String, Value<'a>>,
}

impl<'a> RuleScope<'a> {
    pub fn new() -> Self {
        RuleScope {
            vars: HashMap::new(),
        }
    }

    pub fn get_var(&self, ident: &str) -> Option<&Value> {
        self.vars.get(ident)
    }

    pub fn update_var(&mut self, ident: &str, value: Value<'a>) {
        self.vars.insert(ident.to_owned(), value);
    }
}
