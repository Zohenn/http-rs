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
pub enum RuleErrorKind {
    Syntax(SyntaxErrorKind),
    Semantic(SemanticErrorKind),
}

impl Display for RuleErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleErrorKind::Syntax(kind) => write!(f, "Syntax error: {kind}"),
            RuleErrorKind::Semantic(kind) => write!(f, "Semantic error: {kind}"),
        }
    }
}

#[derive(Debug)]
pub struct RuleError {
    kind: RuleErrorKind,
    position: Position,
}

impl RuleError {
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
