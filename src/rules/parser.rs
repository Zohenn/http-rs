use crate::rules::error::{format_error_in_file, RuleError};
use crate::rules::grammar::file;
use crate::rules::lexer::tokenize;
use crate::rules::Rule;
use std::fs::File;
use std::io::Read;

#[derive(Default)]
pub struct Rules {
    pub rules: Vec<Rule>,
    pub file: String,
}

pub fn parse_file(path: &str) -> Result<Rules, String> {
    let mut file = File::open(path).unwrap();

    let mut file_contents = String::new();

    file.read_to_string(&mut file_contents).unwrap();

    let rules = parse_str(&file_contents).map_err(|err| {
        format_error_in_file(err, &file_contents)
    })?;

    Ok(Rules {
        rules,
        file: file_contents,
    })
}

fn parse_str(source: &str) -> Result<Vec<Rule>, RuleError> {
    file(tokenize(source)?)
}
