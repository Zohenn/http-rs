use crate::rules::error::{RuleError, SyntaxErrorKind};
use std::fmt::{Display, Formatter};
use std::iter::Peekable;
use std::ops::Add;
use std::str::Chars;

type Result<T> = std::result::Result<T, RuleError>;

#[derive(Clone, Debug, PartialEq)]
pub enum RuleTokenKind {
    Ident(String),

    LBrace,
    RBrace,
    LParen,
    RParen,
    Comma,
    Semicolon,
    Dot,
    Eq,
    NotEq,
    And,
    Or,

    // literals
    LitStr(String),
    LitInt(String),

    // keywords
    Matches,
    Redirect,
    Return,
    If,

    Eof,
}

impl RuleTokenKind {
    pub fn len(&self) -> u16 {
        match self {
            RuleTokenKind::Ident(val) => val.len() as u16,
            RuleTokenKind::LBrace => 1,
            RuleTokenKind::RBrace => 1,
            RuleTokenKind::LParen => 1,
            RuleTokenKind::RParen => 1,
            RuleTokenKind::Comma => 1,
            RuleTokenKind::Semicolon => 1,
            RuleTokenKind::Dot => 1,
            RuleTokenKind::Eq => 2,
            RuleTokenKind::NotEq => 2,
            RuleTokenKind::And => 2,
            RuleTokenKind::Or => 2,
            RuleTokenKind::LitStr(val) => val.len() as u16 + 2,
            RuleTokenKind::LitInt(val) => val.len() as u16,
            RuleTokenKind::Matches => 7,
            RuleTokenKind::Redirect => 8,
            RuleTokenKind::Return => 6,
            RuleTokenKind::If => 2,
            RuleTokenKind::Eof => 1,
        }
    }

    pub fn is_lit(&self) -> bool {
        matches!(self, RuleTokenKind::LitInt(_) | RuleTokenKind::LitStr(_))
    }
}

impl Display for RuleTokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str_value = match self {
            RuleTokenKind::Ident(s) => s,
            RuleTokenKind::LBrace => "{",
            RuleTokenKind::RBrace => "}",
            RuleTokenKind::LParen => "(",
            RuleTokenKind::RParen => ")",
            RuleTokenKind::Comma => ",",
            RuleTokenKind::Semicolon => ";",
            RuleTokenKind::Dot => ".",
            RuleTokenKind::Eq => "==",
            RuleTokenKind::NotEq => "!=",
            RuleTokenKind::And => "&&",
            RuleTokenKind::Or => "||",
            RuleTokenKind::LitStr(s) => s,
            RuleTokenKind::LitInt(s) => s,
            RuleTokenKind::Matches => "matches",
            RuleTokenKind::Redirect => "redirect",
            RuleTokenKind::Return => "return",
            RuleTokenKind::If => "if",
            RuleTokenKind::Eof => "EOF",
        };

        write!(f, "{}", str_value)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Position {
    pub line: u32,
    pub column: u32,
    pub len: u16,
}

impl Position {
    pub const fn zero() -> Self {
        Position {
            line: 0,
            column: 0,
            len: 0,
        }
    }

    // todo: Might produce funny results if lines differ
    pub fn sum(positions: &[Position]) -> Self {
        if positions.is_empty() {
            return Position::zero();
        }

        let (first, last) = (positions.first().unwrap(), positions.last().unwrap());

        Position {
            line: first.line,
            column: first.column,
            len: (last.line - first.line) as u16 + last.len,
        }
    }

    pub fn with_len(mut self, len: u16) -> Self {
        self.len = len;

        self
    }
}

impl Add for &Position {
    type Output = Position;

    // todo: Might produce funny results if lines differ
    fn add(self, rhs: Self) -> Self::Output {
        Position {
            line: self.line,
            column: self.column,
            len: (rhs.column - self.column) as u16 + rhs.len,
        }
    }
}

#[derive(Debug)]
pub struct RuleToken {
    pub kind: RuleTokenKind,
    pub position: Position,
}

impl RuleToken {
    pub const fn eof() -> Self {
        RuleToken {
            kind: RuleTokenKind::Eof,
            position: Position::zero(),
        }
    }
}

struct LexerIter<'a> {
    input: &'a str,
    iter: Peekable<Chars<'a>>,
    // Position of the token that will be returned on next Self::next() call
    position: Position,
}

impl<'a> LexerIter<'a> {
    fn new(input: &'a str) -> Self {
        LexerIter {
            input,
            iter: input.chars().peekable(),
            position: Position {
                line: 1,
                column: 1,
                len: 0,
            },
        }
    }

    fn peek(&mut self) -> Option<&char> {
        self.iter.peek()
    }

    fn next(&mut self) -> Option<char> {
        let next = self.iter.next();

        if let Some(c) = next {
            match c {
                '\n' => {
                    self.position.line += 1;
                    self.position.column = 1;
                }
                _ => self.position.column += 1,
            }
        }

        next
    }

    fn skip_whitespace(&mut self) {
        while self.peek().unwrap_or(&'\0').is_ascii_whitespace() {
            self.next();
        }
    }

    #[rustfmt::skip]
    fn read_ident(&mut self) -> String {
        self.read_until_inner(|next: &char| next.is_ascii_alphabetic() || next == &'_').0
    }

    fn read_string(&mut self) -> Result<String> {
        let (lit, next) = self.read_until_inner(|next: &char| next != &'"');

        match next {
            Some(c) if c == '"' => {
                // swallow ending "
                self.next();

                Ok(lit)
            }
            _ => Err(RuleError::syntax(
                SyntaxErrorKind::UnterminatedString,
                self.position,
            )),
        }
    }

    fn read_int(&mut self) -> Result<String> {
        let (lit, next) = self.read_until_inner(|next: &char| next.is_ascii_digit());

        match next {
            Some(c) if !c.is_ascii_alphabetic() => Ok(lit),
            None => Ok(lit),
            Some(c) => Err(RuleError::syntax(
                SyntaxErrorKind::UnexpectedToken(c.into()),
                self.position,
            )),
        }
    }

    fn read_until_whitespace(&mut self) -> String {
        self.read_until_inner(|next: &char| !next.is_ascii_whitespace())
            .0
    }

    fn read_until_lf(&mut self) -> String {
        self.read_until_inner(|next: &char| next != &'\n').0
    }

    fn read_until_inner(&mut self, condition: impl Fn(&char) -> bool) -> (String, Option<char>) {
        let mut output = String::new();

        loop {
            let next = self.peek();

            match next {
                Some(next) if condition(next) => {
                    output.push(*next);
                    self.next();
                }
                _ => return (output, next.copied()),
            }
        }
    }
}

pub(crate) fn tokenize(input: &str) -> Result<Vec<RuleToken>> {
    let mut iter = LexerIter::new(input);

    let mut tokens: Vec<RuleToken> = vec![];

    iter.skip_whitespace();

    while iter.peek().is_some() {
        let position = iter.position;
        let character = iter.next().unwrap();
        let token = match character {
            '{' => RuleTokenKind::LBrace,
            '}' => RuleTokenKind::RBrace,
            '(' => RuleTokenKind::LParen,
            ')' => RuleTokenKind::RParen,
            ',' => RuleTokenKind::Comma,
            '"' => {
                let lit = iter.read_string()?;

                RuleTokenKind::LitStr(lit)
            }
            ';' => RuleTokenKind::Semicolon,
            '.' => RuleTokenKind::Dot,
            '=' => match iter.peek() {
                Some(c) if c == &'=' => {
                    iter.next();
                    RuleTokenKind::Eq
                }
                _ => {
                    return Err(RuleError::syntax(
                        SyntaxErrorKind::UnexpectedToken(character.into()),
                        position,
                    ))
                }
            },
            '!' => match iter.peek() {
                Some(c) if c == &'=' => {
                    iter.next();
                    RuleTokenKind::NotEq
                }
                _ => {
                    return Err(RuleError::syntax(
                        SyntaxErrorKind::UnexpectedToken(character.into()),
                        position,
                    ))
                }
            },
            '&' => match iter.peek() {
                Some(c) if c == &'&' => {
                    iter.next();
                    RuleTokenKind::And
                }
                _ => {
                    return Err(RuleError::syntax(
                        SyntaxErrorKind::UnexpectedToken(character.into()),
                        position,
                    ))
                }
            },
            '|' => match iter.peek() {
                Some(c) if c == &'|' => {
                    iter.next();
                    RuleTokenKind::Or
                }
                _ => {
                    return Err(RuleError::syntax(
                        SyntaxErrorKind::UnexpectedToken(character.into()),
                        position,
                    ))
                }
            },
            '#' => {
                // This is a comment
                iter.read_until_lf();
                iter.skip_whitespace();
                continue;
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let ident = String::from(character);
                let ident = ident + &iter.read_ident();

                match &*ident {
                    "matches" => RuleTokenKind::Matches,
                    "redirect" => RuleTokenKind::Redirect,
                    "return" => RuleTokenKind::Return,
                    "if" => RuleTokenKind::If,
                    _ => RuleTokenKind::Ident(ident),
                }
            }
            '0'..='9' => {
                let lit = String::from(character) + &iter.read_int()?;

                RuleTokenKind::LitInt(lit)
            }
            _ => {
                return Err(RuleError::syntax(
                    SyntaxErrorKind::UnexpectedToken(character.into()),
                    position,
                ))
            }
        };

        tokens.push(RuleToken {
            position: position.with_len(token.len()),
            kind: token,
        });

        // todo: change this in some way, lexer is not the best place for this
        // either change the grammar and make pattern be a normal " delimited string
        // or store info on whether next character after token is whitespace
        // and take all grouped (not separated by whitespace) tokens when parsing a rule
        match tokens.last() {
            Some(token) if matches!(token.kind, RuleTokenKind::Matches) => {
                iter.skip_whitespace();
                let position = iter.position;

                tokens.push(RuleToken {
                    kind: RuleTokenKind::LitStr(iter.read_until_whitespace()),
                    position,
                });
            }
            _ => {}
        }

        iter.skip_whitespace();
    }

    Ok(tokens)
}

#[cfg(test)]
mod test {
    use crate::rules::lexer::{tokenize, RuleTokenKind};

    #[test]
    fn base_test() {
        let tokens = tokenize(
            r#"
            matches /index.html {
                set_header("Server", "http-rs");
                abc == 123;

                if method == "POST" {
                    return 400;
                }

                request.method;
                response.set_header();

                return 301 "/index2.html";
            }
        "#,
        )
        .unwrap();

        let expected_tokens = vec![
            RuleTokenKind::Matches,
            RuleTokenKind::LitStr("/index.html".into()),
            RuleTokenKind::LBrace,
            RuleTokenKind::Ident("set_header".into()),
            RuleTokenKind::LParen,
            RuleTokenKind::LitStr("Server".into()),
            RuleTokenKind::Comma,
            RuleTokenKind::LitStr("http-rs".into()),
            RuleTokenKind::RParen,
            RuleTokenKind::Semicolon,
            RuleTokenKind::Ident("abc".into()),
            RuleTokenKind::Eq,
            RuleTokenKind::LitInt("123".into()),
            RuleTokenKind::Semicolon,
            RuleTokenKind::If,
            RuleTokenKind::Ident("method".into()),
            RuleTokenKind::Eq,
            RuleTokenKind::LitStr("POST".into()),
            RuleTokenKind::LBrace,
            RuleTokenKind::Return,
            RuleTokenKind::LitInt("400".into()),
            RuleTokenKind::Semicolon,
            RuleTokenKind::RBrace,
            RuleTokenKind::Ident("request".into()),
            RuleTokenKind::Dot,
            RuleTokenKind::Ident("method".into()),
            RuleTokenKind::Semicolon,
            RuleTokenKind::Ident("response".into()),
            RuleTokenKind::Dot,
            RuleTokenKind::Ident("set_header".into()),
            RuleTokenKind::LParen,
            RuleTokenKind::RParen,
            RuleTokenKind::Semicolon,
            RuleTokenKind::Return,
            RuleTokenKind::LitInt("301".into()),
            RuleTokenKind::LitStr("/index2.html".into()),
            RuleTokenKind::Semicolon,
            RuleTokenKind::RBrace,
        ];

        for (index, token) in tokens.iter().enumerate() {
            let expected = expected_tokens.get(index).unwrap_or(&RuleTokenKind::Eof);
            println!("expected: {expected:?}, got: {token:?}");
            assert_eq!(&token.kind, expected);
        }
    }

    #[test]
    fn err_on_invalid_int() {
        let tokens = tokenize("34rioewj");

        assert!(tokens.is_err())
    }

    #[test]
    fn err_on_unterminated_string() {
        let tokens = tokenize("return 301 \"/index.html");

        assert!(tokens.is_err());
    }
}
