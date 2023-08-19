use crate::rules::error::RuleError;
use crate::rules::grammar::file;
use crate::rules::lexer::tokenize;
use crate::rules::Rule;
use std::fs::File;
use std::io::Read;

pub fn parse_file(path: &str) -> Result<Vec<Rule>, String> {
    let mut file = File::open(path).unwrap();

    let mut file_contents = String::new();

    file.read_to_string(&mut file_contents).unwrap();

    parse_str(&file_contents).map_err(|err| {
        let base_err = err.to_string();

        let lines = file_contents.lines().collect::<Vec<&str>>();

        let pos = err.position();
        let line_indent = format!("{} | ", pos.line);
        let line = lines.get(pos.line as usize - 1).unwrap_or(&"");
        let caret_indent = " ".repeat(line_indent.len() + pos.column as usize - 1);

        format!("{base_err}\n{line_indent}{line}\n{caret_indent}^")
    })
}

fn parse_str(source: &str) -> Result<Vec<Rule>, RuleError> {
    file(tokenize(source)?)
}
