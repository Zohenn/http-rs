use crate::response_status_code::ResponseStatusCode;
use crate::rules::lexer::RuleToken;
use crate::rules::Rule;
use std::error::Error;
use std::iter::Peekable;
use std::vec::IntoIter;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
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
    swallow(iter, RuleToken::Matches)?;

    let pattern = match pattern(iter)? {
        RuleToken::LitStr(pattern) => pattern,
        _ => unreachable!(),
    };

    swallow(iter, RuleToken::LBrace)?;

    let statements = rule_statements(iter)?;

    swallow(iter, RuleToken::RBrace)?;

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
            RuleToken::Ident(_) => base_statement(iter)?,
            RuleToken::Redirect => redirect_statement(iter)?,
            RuleToken::Return => return_statement(iter)?,
            RuleToken::RBrace => break,
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
        Some(RuleToken::Ident(name)) => {
            let mut args: Vec<Lit> = vec![];

            swallow(iter, RuleToken::LParen)?;

            while let Some(token) = iter.peek() {
                match token {
                    RuleToken::LitStr(str_val) => {
                        args.push(Lit::String(str_val.clone()));
                        iter.next();
                    }
                    RuleToken::LitInt(int_val) => {
                        args.push(Lit::Int(int_val.clone()));
                        iter.next();
                    }
                    RuleToken::Comma => {
                        swallow(iter, RuleToken::Comma)?;
                    }
                    RuleToken::RParen => break,
                    _ => return Err(format!("Unexpected token {token:?}").into()),
                }
            }

            swallow(iter, RuleToken::RParen)?;
            swallow(iter, RuleToken::Semicolon)?;

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
        Some(RuleToken::Redirect) => {
            let response_code = match int(iter)? {
                RuleToken::LitInt(int_val) => int_val
                    .parse::<u16>()
                    .map_err(|_| "Incorrect response code")?,
                _ => unreachable!(),
            };
            let response_code = ResponseStatusCode::try_from(response_code)?;

            let location = match string(iter)? {
                RuleToken::LitStr(str_val) => str_val,
                _ => unreachable!(),
            };

            let statement = Statement {
                kind: StatementKind::Redirect(response_code, location),
            };

            swallow(iter, RuleToken::Semicolon)?;

            statement
        }
        _ => unreachable!(),
    };

    Ok(statement)
}

pub fn return_statement(iter: &mut TokenIter) -> Result<Statement> {
    let statement = match iter.next() {
        Some(RuleToken::Return) => {
            let response_code = match int(iter)? {
                RuleToken::LitInt(int_val) => int_val
                    .parse::<u16>()
                    .map_err(|_| "Incorrect response code")?,
                _ => unreachable!(),
            };
            let response_code = ResponseStatusCode::try_from(response_code)?;

            let location_or_body = string(iter).ok().map(|token| match token {
                RuleToken::LitStr(str_val) => str_val,
                _ => unreachable!(),
            });

            let statement = Statement {
                kind: StatementKind::Return(response_code, location_or_body),
            };

            swallow(iter, RuleToken::Semicolon)?;

            statement
        }
        _ => unreachable!(),
    };

    Ok(statement)
}

macro_rules! rule_helper {
    ($name:ident, $variant:pat) => {
        pub fn $name(iter: &mut TokenIter) -> Result<RuleToken> {
            match iter.peek() {
                Some($variant) => Ok(iter.next().unwrap()),
                Some(token) => Err(format!(concat!("Expected ", stringify!($name), ", got {:?}"), token).into()),
                _ => Err(format!(stringify!(Expected $name, got EOF)).into()),
            }
        }
    }
}

rule_helper!(pattern, RuleToken::LitStr(_));
rule_helper!(ident, RuleToken::Ident(_));
rule_helper!(int, RuleToken::LitInt(_));
rule_helper!(string, RuleToken::LitStr(_));

fn swallow(iter: &mut TokenIter, to_swallow: RuleToken) -> Result<RuleToken> {
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
