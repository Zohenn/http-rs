use crate::response_status_code::ResponseStatusCode;
use crate::rules::lexer::RuleTokenKind;
use crate::rules::Rule;
use std::error::Error;
use std::iter::Peekable;
use std::vec::IntoIter;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type TokenIter = Peekable<IntoIter<RuleTokenKind>>;

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

#[derive(Debug)]
pub struct Statement {
    pub kind: StatementKind,
}

pub fn file(tokens: Vec<RuleTokenKind>) -> Result<Vec<Rule>> {
    let mut rules: Vec<Rule> = vec![];

    let mut iter = tokens.into_iter().peekable();

    while iter.peek().is_some() {
        rules.push(rule(&mut iter)?);
    }

    Ok(rules)
}

pub fn rule(iter: &mut TokenIter) -> Result<Rule> {
    swallow(iter, RuleTokenKind::Matches)?;

    let pattern = match pattern(iter)? {
        RuleTokenKind::LitStr(pattern) => pattern,
        _ => unreachable!(),
    };

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
        let statement = match token {
            RuleTokenKind::Ident(_) => base_statement(iter)?,
            RuleTokenKind::Redirect => redirect_statement(iter)?,
            RuleTokenKind::Return => return_statement(iter)?,
            RuleTokenKind::RBrace => break,
            _ => return Err(format!("Unexpected token {token:?}").into()),
        };

        match statements.last() {
            Some(last_statement) if matches!(last_statement.kind, StatementKind::Return(_, _)) => {
                return Err("Unexpected statement after return".into());
            }
            _ => statements.push(statement),
        }
    }

    Ok(statements)
}

pub fn base_statement(iter: &mut TokenIter) -> Result<Statement> {
    let statement = match iter.next() {
        Some(RuleTokenKind::Ident(name)) => {
            let mut args: Vec<Lit> = vec![];

            swallow(iter, RuleTokenKind::LParen)?;

            while let Some(token) = iter.peek() {
                match token {
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
                    _ => return Err(format!("Unexpected token {token:?}").into()),
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
        Some(RuleTokenKind::Redirect) => {
            let response_code = match int(iter)? {
                RuleTokenKind::LitInt(int_val) => int_val
                    .parse::<u16>()
                    .map_err(|_| "Incorrect response code")?,
                _ => unreachable!(),
            };
            let response_code = ResponseStatusCode::try_from(response_code)?;

            let location = match string(iter)? {
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
        Some(RuleTokenKind::Return) => {
            let response_code = match int(iter)? {
                RuleTokenKind::LitInt(int_val) => int_val
                    .parse::<u16>()
                    .map_err(|_| "Incorrect response code")?,
                _ => unreachable!(),
            };
            let response_code = ResponseStatusCode::try_from(response_code)?;

            let location_or_body = string(iter).ok().map(|token| match token {
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

macro_rules! rule_helper {
    ($name:ident, $variant:pat) => {
        pub fn $name(iter: &mut TokenIter) -> Result<RuleTokenKind> {
            match iter.peek() {
                Some($variant) => Ok(iter.next().unwrap()),
                Some(token) => Err(format!(concat!("Expected ", stringify!($name), ", got {:?}"), token).into()),
                _ => Err(format!(stringify!(Expected $name, got EOF)).into()),
            }
        }
    }
}

rule_helper!(pattern, RuleTokenKind::LitStr(_));
rule_helper!(ident, RuleTokenKind::Ident(_));
rule_helper!(int, RuleTokenKind::LitInt(_));
rule_helper!(string, RuleTokenKind::LitStr(_));

fn swallow(iter: &mut TokenIter, to_swallow: RuleTokenKind) -> Result<RuleTokenKind> {
    match iter.peek() {
        Some(token) => {
            if matches!(token, to_swallow) {
                Ok(iter.next().unwrap())
            } else {
                Err(format!("Expected {to_swallow:?}, got {token:?}").into())
            }
        }
        _ => Err(format!("Expected {to_swallow:?}").into()),
    }
}
