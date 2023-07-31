use std::iter::Peekable;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Debug, PartialEq)]
pub enum RuleToken {
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

pub(crate) fn tokenize(input: &str) -> Result<Vec<RuleToken>> {
    let mut iter = input.chars().peekable();

    let mut tokens: Vec<RuleToken> = vec![];

    skip_whitespace(&mut iter);

    while iter.peek().is_some() {
        let character = iter.next().unwrap();
        let token = match character {
            '{' => RuleToken::LBrace,
            '}' => RuleToken::RBrace,
            '(' => RuleToken::LParen,
            ')' => RuleToken::RParen,
            ',' => RuleToken::Comma,
            '"' => {
                let lit = read_string(&mut iter)?;

                RuleToken::LitStr(lit)
            }
            ';' => RuleToken::Semicolon,
            'a'..='z' | 'A'..='Z' | '_' => {
                let ident = String::from(character);
                let ident = ident + &read_ident(&mut iter);

                match &*ident {
                    "matches" => RuleToken::Matches,
                    "redirect" => RuleToken::Redirect,
                    "return" => RuleToken::Return,
                    _ => RuleToken::Ident(ident),
                }
            }
            '0'..='9' => {
                let lit = String::from(character) + &read_int(&mut iter)?;

                RuleToken::LitInt(lit)
            }
            _ => return Err(format!("Unexpected token: {}", { character }).into()),
        };

        tokens.push(token);

        // todo: change this in some way, lexer is not the best place for this
        // either change the grammar and make pattern be a normal " delimited string
        // or store info on whether next character after token is whitespace
        // and take all grouped (not separated by whitespace) tokens when parsing a rule
        if let Some(RuleToken::Matches) = tokens.last() {
            skip_whitespace(&mut iter);
            tokens.push(RuleToken::LitStr(read_until_whitespace(&mut iter)));
        };

        skip_whitespace(&mut iter);
    }

    Ok(tokens)
}

fn skip_whitespace(iter: &mut Peekable<impl Iterator<Item = char>>) {
    while iter.peek().unwrap_or(&'\0').is_ascii_whitespace() {
        iter.next();
    }
}

#[rustfmt::skip]
fn read_ident(iter: &mut Peekable<impl Iterator<Item = char>>) -> String {
    read_until_inner(iter, |next: &char| next.is_ascii_alphabetic() || next == &'_').0
}

fn read_string(iter: &mut Peekable<impl Iterator<Item = char>>) -> Result<String> {
    let (lit, next) = read_until_inner(iter, |next: &char| next != &'"');

    match next {
        Some(c) if c == '"' => {
            // swallow ending "
            iter.next();

            Ok(lit)
        }
        _ => Err("Unterminated string".into()),
    }
}

fn read_int(iter: &mut Peekable<impl Iterator<Item = char>>) -> Result<String> {
    let (lit, next) = read_until_inner(iter, |next: &char| next.is_ascii_digit());

    match next {
        Some(c) if c.is_ascii_whitespace() => Ok(lit),
        None => Ok(lit),
        _ => Err("Unexpected token".into()),
    }
}

fn read_until_whitespace(iter: &mut Peekable<impl Iterator<Item = char>>) -> String {
    read_until_inner(iter, |next: &char| !next.is_ascii_whitespace()).0
}

fn read_until_inner(
    iter: &mut Peekable<impl Iterator<Item = char>>,
    condition: impl Fn(&char) -> bool,
) -> (String, Option<char>) {
    let mut output = String::new();

    loop {
        let next = iter.peek();

        match next {
            Some(next) if condition(next) => {
                output.push(*next);
                iter.next();
            }
            _ => return (output, next.copied()),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::rules::lexer::{tokenize, RuleToken};

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
            RuleToken::Matches,
            RuleToken::LitStr("/index.html".into()),
            RuleToken::LBrace,
            RuleToken::Ident("set_header".into()),
            RuleToken::LParen,
            RuleToken::LitStr("Server".into()),
            RuleToken::Comma,
            RuleToken::LitStr("http-rs".into()),
            RuleToken::RParen,
            RuleToken::Semicolon,
            RuleToken::Return,
            RuleToken::LitInt("301".into()),
            RuleToken::LitStr("/index2.html".into()),
            RuleToken::Semicolon,
            RuleToken::RBrace,
        ];

        for (index, token) in tokens.iter().enumerate() {
            let expected = expected_tokens.get(index).unwrap_or(&RuleToken::Eof);
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
