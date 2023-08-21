use crate::rules::lexer::Position;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum SyntaxErrorKind {
    UnexpectedToken(String),
    ExpectedOther(String, String),
    UnterminatedString,
    IncorrectResponseCode(String),
}

impl Display for SyntaxErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SyntaxErrorKind::UnexpectedToken(s) => write!(f, "Unexpected token \"{s}\""),
            SyntaxErrorKind::ExpectedOther(expected, got) => {
                write!(f, "Expected \"{expected}\", got \"{got}\"")
            }
            SyntaxErrorKind::UnterminatedString => write!(f, "Unterminated string literal"),
            SyntaxErrorKind::IncorrectResponseCode(s) => {
                write!(f, "Incorrect response code \"{s}\"")
            }
        }
    }
}

#[derive(Debug)]
pub enum SemanticErrorKind {
    UnexpectedStatement(String),
}

impl Display for SemanticErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SemanticErrorKind::UnexpectedStatement(s) => write!(f, "Unexpected \"{s}\" statement"),
        }
    }
}

#[derive(Debug)]
pub enum RuntimeErrorKind {
    IncorrectType(String, String),
    UnresolvedReference(String),
    MemberNotDefined(String, String),
    TooFewArguments(usize, usize),
}

impl Display for RuntimeErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeErrorKind::IncorrectType(expected, got) => {
                write!(f, "Incorrect type, expected {expected}, got {got}")
            }
            RuntimeErrorKind::UnresolvedReference(s) => write!(f, "Unresolved reference \"{s}\""),
            RuntimeErrorKind::MemberNotDefined(member, object) => {
                write!(f, "Member \"{member}\" is not defined on \"{object}\"")
            }
            RuntimeErrorKind::TooFewArguments(expected, got) => {
                write!(
                    f,
                    "Function takes {expected} arguments, but {got} arguments were passed"
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum RuleErrorKind {
    Syntax(SyntaxErrorKind),
    Semantic(SemanticErrorKind),
    Runtime(RuntimeErrorKind),
}

impl Display for RuleErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleErrorKind::Syntax(kind) => write!(f, "Syntax error: {kind}"),
            RuleErrorKind::Semantic(kind) => write!(f, "Semantic error: {kind}"),
            RuleErrorKind::Runtime(kind) => write!(f, "Runtime error: {kind}"),
        }
    }
}

#[derive(Debug)]
pub struct RuleError {
    kind: RuleErrorKind,
    position: Position,
}

impl RuleError {
    pub fn new(kind: RuleErrorKind, position: Position) -> Self {
        RuleError { kind, position }
    }

    pub fn syntax(kind: SyntaxErrorKind, position: Position) -> Self {
        RuleError {
            kind: RuleErrorKind::Syntax(kind),
            position,
        }
    }

    pub fn semantic(kind: SemanticErrorKind, position: Position) -> Self {
        RuleError {
            kind: RuleErrorKind::Semantic(kind),
            position,
        }
    }

    pub fn runtime(kind: RuntimeErrorKind, position: Position) -> Self {
        RuleError {
            kind: RuleErrorKind::Runtime(kind),
            position,
        }
    }

    pub fn kind_owned(self) -> RuleErrorKind {
        self.kind
    }

    pub fn position(&self) -> &Position {
        &self.position
    }
}

impl Display for RuleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {}:{}",
            self.kind, self.position.line, self.position.column
        )
    }
}

impl Error for RuleError {}

pub fn format_error_in_file(err: RuleError, file_contents: &str) -> String {
    let base_err = err.to_string();

    let lines = file_contents.lines().collect::<Vec<&str>>();

    let pos = err.position();
    let line_indent = format!("{} | ", pos.line);
    let line = lines.get(pos.line as usize - 1).unwrap_or(&"");
    let caret_indent = " ".repeat(line_indent.len() + pos.column as usize - 1);

    format!("{base_err}\n{line_indent}{line}\n{caret_indent}^")
}
