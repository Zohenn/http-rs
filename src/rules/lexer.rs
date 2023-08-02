use std::iter::Peekable;
use std::str::Chars;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Debug, PartialEq)]
pub enum RuleTokenKind {
    Ident(String),

    LBrace,
    RBrace,
    LParen,
    RParen,
    Comma,
    Semicolon,

    // literals
    LitStr(String),
    LitInt(String),

    // keywords
    Matches,
    Redirect,
    Return,

    Eof,
}

#[derive(Debug, Copy, Clone)]
pub struct Position {
    line: u32,
    column: u32,
    len: u16,
}

#[derive(Debug)]
pub struct RuleToken {
    kind: RuleTokenKind,
    position: Position,
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
            _ => Err(format!("Unterminated string {:?}", self.position).into()),
        }
    }

    fn read_int(&mut self) -> Result<String> {
        let (lit, next) = self.read_until_inner(|next: &char| next.is_ascii_digit());

        match next {
            Some(c) if c.is_ascii_whitespace() => Ok(lit),
            None => Ok(lit),
            _ => Err(format!("Unexpected token {:?}", self.position).into()),
        }
    }

    fn read_until_whitespace(&mut self) -> String {
        self.read_until_inner(|next: &char| !next.is_ascii_whitespace())
            .0
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

pub(crate) fn tokenize(input: &str) -> Result<Vec<RuleTokenKind>> {
    let mut iter = LexerIter::new(input);

    let mut tokens: Vec<RuleTokenKind> = vec![];

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
            'a'..='z' | 'A'..='Z' | '_' => {
                let ident = String::from(character);
                let ident = ident + &iter.read_ident();

                match &*ident {
                    "matches" => RuleTokenKind::Matches,
                    "redirect" => RuleTokenKind::Redirect,
                    "return" => RuleTokenKind::Return,
                    _ => RuleTokenKind::Ident(ident),
                }
            }
            '0'..='9' => {
                let lit = String::from(character) + &iter.read_int()?;

                RuleTokenKind::LitInt(lit)
            }
            _ => return Err(format!("Unexpected token: {} {:?}", character, position).into()),
        };

        tokens.push(token);

        // todo: change this in some way, lexer is not the best place for this
        // either change the grammar and make pattern be a normal " delimited string
        // or store info on whether next character after token is whitespace
        // and take all grouped (not separated by whitespace) tokens when parsing a rule
        if let Some(RuleTokenKind::Matches) = tokens.last() {
            iter.skip_whitespace();
            tokens.push(RuleTokenKind::LitStr(iter.read_until_whitespace()));
        };

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
            RuleTokenKind::Return,
            RuleTokenKind::LitInt("301".into()),
            RuleTokenKind::LitStr("/index2.html".into()),
            RuleTokenKind::Semicolon,
            RuleTokenKind::RBrace,
        ];

        for (index, token) in tokens.iter().enumerate() {
            let expected = expected_tokens.get(index).unwrap_or(&RuleTokenKind::Eof);
            println!("expected: {expected:?}, got: {token:?}");
            assert_eq!(token, expected);
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
