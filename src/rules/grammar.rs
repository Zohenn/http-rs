use crate::response_status_code::ResponseStatusCode;
use crate::rules::error::{RuleError, SemanticErrorKind, SyntaxErrorKind};
use crate::rules::lexer::{Position, RuleToken, RuleTokenKind};
use crate::rules::Rule;
use std::error::Error;
use std::fmt::{format, Display, Formatter};
use std::iter::Peekable;
use std::vec::IntoIter;

type Result<T> = std::result::Result<T, RuleError>;
type TokenIter = Peekable<IntoIter<RuleToken>>;

#[derive(Debug)]
pub enum Lit {
    String(String),
    Int(String),
}

pub enum Expr {
    Lit(Lit),
}

#[derive(Debug)]
pub enum StatementKind {
    Func(String, Vec<Lit>),
    Redirect(ResponseStatusCode, String),
    Return(ResponseStatusCode, Option<String>),
}

impl Display for StatementKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str_value = match self {
            StatementKind::Func(_, _) => "function call",
            StatementKind::Redirect(_, _) => "redirect",
            StatementKind::Return(_, _) => "return",
        };

        write!(f, "{str_value}")
    }
}

#[derive(Debug)]
pub struct Statement {
    pub kind: StatementKind,
}

pub fn file(tokens: Vec<RuleToken>) -> Result<Vec<Rule>> {
    let mut rules: Vec<Rule> = vec![];

    let mut iter = tokens.into_iter().peekable();

    while iter.peek().is_some() {
        rules.push(rule(&mut iter)?);
    }

    Ok(rules)
}

pub fn rule(iter: &mut TokenIter) -> Result<Rule> {
    swallow(iter, RuleTokenKind::Matches)?;

    let RuleTokenKind::LitStr(pattern) = pattern(iter)?.kind else { unreachable!() };

    swallow(iter, RuleTokenKind::LBrace)?;

    let statements = rule_statements(iter)?;

    swallow(iter, RuleTokenKind::RBrace)?;

    let rule = Rule {
        pattern,
        actions: vec![],
        statements,
    };

    Ok(rule)
}

pub fn rule_statements(iter: &mut TokenIter) -> Result<Vec<Statement>> {
    let mut statements: Vec<Statement> = vec![];

    while let Some(token) = iter.peek() {
        let position = token.position;

        let statement = match token.kind {
            RuleTokenKind::Ident(_) => base_statement(iter)?,
            RuleTokenKind::Redirect => redirect_statement(iter)?,
            RuleTokenKind::Return => return_statement(iter)?,
            RuleTokenKind::RBrace => break,
            _ => {
                return Err(RuleError::syntax(
                    SyntaxErrorKind::UnexpectedToken(token.kind.to_string()),
                    position,
                ))
            }
        };

        match statements.last() {
            // todo: move this check to semantic analyzer
            Some(last_statement) if matches!(last_statement.kind, StatementKind::Return(_, _)) => {
                return Err(RuleError::semantic(
                    SemanticErrorKind::UnexpectedStatement(statement.kind.to_string()),
                    position,
                ));
            }
            _ => statements.push(statement),
        }
    }

    Ok(statements)
}

pub fn base_statement(iter: &mut TokenIter) -> Result<Statement> {
    let statement = match iter.next() {
        Some(RuleToken {
            kind: RuleTokenKind::Ident(name),
            ..
        }) => {
            let mut args: Vec<Lit> = vec![];

            swallow(iter, RuleTokenKind::LParen)?;

            while let Some(token) = iter.peek() {
                match &token.kind {
                    RuleTokenKind::LitStr(str_val) => {
                        args.push(Lit::String(str_val.clone()));
                        iter.next();
                    }
                    RuleTokenKind::LitInt(int_val) => {
                        args.push(Lit::Int(int_val.clone()));
                        iter.next();
                    }
                    RuleTokenKind::Comma => {
                        swallow(iter, RuleTokenKind::Comma)?;
                    }
                    RuleTokenKind::RParen => break,
                    _ => {
                        return Err(RuleError::syntax(
                            SyntaxErrorKind::UnexpectedToken(token.kind.to_string()),
                            token.position,
                        ))
                    }
                }
            }

            swallow(iter, RuleTokenKind::RParen)?;
            swallow(iter, RuleTokenKind::Semicolon)?;

            Statement {
                kind: StatementKind::Func(name, args),
            }
        }
        Some(_) => todo!(),
        _ => unreachable!(),
    };

    Ok(statement)
}

pub fn redirect_statement(iter: &mut TokenIter) -> Result<Statement> {
    let statement = match iter.next() {
        Some(RuleToken {
            kind: RuleTokenKind::Redirect,
            ..
        }) => {
            let response_code = status_code(iter)?;

            let location = match string(iter)?.kind {
                RuleTokenKind::LitStr(str_val) => str_val,
                _ => unreachable!(),
            };

            let statement = Statement {
                kind: StatementKind::Redirect(response_code, location),
            };

            swallow(iter, RuleTokenKind::Semicolon)?;

            statement
        }
        _ => unreachable!(),
    };

    Ok(statement)
}

pub fn return_statement(iter: &mut TokenIter) -> Result<Statement> {
    let statement = match iter.next() {
        Some(RuleToken {
            kind: RuleTokenKind::Return,
            ..
        }) => {
            let response_code = status_code(iter)?;

            let location_or_body = string(iter).ok().map(|token| match token.kind {
                RuleTokenKind::LitStr(str_val) => str_val,
                _ => unreachable!(),
            });

            let statement = Statement {
                kind: StatementKind::Return(response_code, location_or_body),
            };

            swallow(iter, RuleTokenKind::Semicolon)?;

            statement
        }
        _ => unreachable!(),
    };

    Ok(statement)
}

fn status_code(iter: &mut TokenIter) -> Result<ResponseStatusCode> {
    let (response_code, position) = match int(iter)? {
        RuleToken {
            kind: RuleTokenKind::LitInt(int_val),
            position,
        } => {
            let code = int_val.parse::<u16>().map_err(|_| {
                RuleError::syntax(SyntaxErrorKind::IncorrectResponseCode(int_val), position)
            })?;

            (code, position)
        }
        _ => unreachable!(),
    };

    let response_code = ResponseStatusCode::try_from(response_code).map_err(|_| {
        RuleError::syntax(
            SyntaxErrorKind::IncorrectResponseCode(response_code.to_string()),
            position,
        )
    })?;

    Ok(response_code)
}

macro_rules! rule_helper {
    ($name:ident, $variant:pat, $expected:literal) => {
        pub fn $name(iter: &mut TokenIter) -> Result<RuleToken> {
            match iter.peek() {
                Some(token) if matches!(token.kind, $variant) => Ok(iter.next().unwrap()),
                Some(token) => Err(RuleError::syntax(
                    SyntaxErrorKind::ExpectedOther($expected.into(), token.kind.to_string()),
                    token.position,
                )),
                _ => Err(RuleError::syntax(
                    SyntaxErrorKind::ExpectedOther(
                        $expected.into(),
                        RuleTokenKind::Eof.to_string(),
                    ),
                    Position::zero(),
                )),
            }
        }
    };
}

rule_helper!(pattern, RuleTokenKind::LitStr(_), "string");
rule_helper!(ident, RuleTokenKind::Ident(_), "string");
rule_helper!(int, RuleTokenKind::LitInt(_), "integer");
rule_helper!(string, RuleTokenKind::LitStr(_), "string");

fn swallow(iter: &mut TokenIter, to_swallow: RuleTokenKind) -> Result<RuleToken> {
    match iter.peek() {
        Some(token) => {
            if matches!(&token.kind, to_swallow) {
                Ok(iter.next().unwrap())
            } else {
                Err(RuleError::syntax(
                    SyntaxErrorKind::ExpectedOther(to_swallow.to_string(), token.kind.to_string()),
                    token.position,
                ))
            }
        }
        _ => Err(RuleError::syntax(
            SyntaxErrorKind::ExpectedOther(to_swallow.to_string(), RuleTokenKind::Eof.to_string()),
            Position::zero(),
        )),
    }
}
