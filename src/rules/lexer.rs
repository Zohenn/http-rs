use std::iter::Peekable;

#[derive(Clone, Debug, PartialEq)]
pub enum RuleToken {
    Ident(String),

    LBrace,
    RBrace,
    Semicolon,

    // literals
    LitStr(String),
    LitInt(String),

    // keywords
    Matches,
    Return,

    Eof,
}

pub(crate) fn tokenize(input: &str) -> Vec<RuleToken> {
    let mut iter = input.chars().peekable();

    let mut tokens: Vec<RuleToken> = vec![];

    skip_whitespace(&mut iter);

    while iter.peek().is_some() {
        let character = iter.next().unwrap();
        let token = match character {
            '{' => RuleToken::LBrace,
            '}' => RuleToken::RBrace,
            '"' => {
                let lit = read_string(&mut iter);

                // swallow ending "
                iter.next();

                RuleToken::LitStr(lit)
            }
            ';' => RuleToken::Semicolon,
            'a'..='z' | 'A'..='Z' | '_' => {
                let ident = String::from(character);
                let ident = ident + &read_ident(&mut iter);

                match &*ident {
                    "matches" => RuleToken::Matches,
                    "return" => RuleToken::Return,
                    _ => RuleToken::Ident(ident),
                }
            }
            '0'..='9' => {
                let lit = String::from(character) + &read_int(&mut iter);

                RuleToken::LitInt(lit)
            }
            _ => panic!("Unexpected token: {}", { character }),
        };

        tokens.push(token);

        if let Some(RuleToken::Matches) = tokens.last() {
            skip_whitespace(&mut iter);
            tokens.push(RuleToken::LitStr(read_until_whitespace(&mut iter)));
        };

        skip_whitespace(&mut iter);
    }

    tokens
}

fn skip_whitespace(iter: &mut Peekable<impl Iterator<Item = char>>) {
    while iter.peek().unwrap_or(&'\0').is_ascii_whitespace() {
        iter.next();
    }
}

fn read_ident(iter: &mut Peekable<impl Iterator<Item = char>>) -> String {
    read_until_inner(iter, |next: &char| {
        next.is_ascii_alphabetic() || next == &'_'
    })
}

fn read_string(iter: &mut Peekable<impl Iterator<Item = char>>) -> String {
    read_until_inner(iter, |next: &char| next != &'"')
}

fn read_int(iter: &mut Peekable<impl Iterator<Item = char>>) -> String {
    read_until_inner(iter, |next: &char| next.is_ascii_digit())
}

fn read_until_whitespace(iter: &mut Peekable<impl Iterator<Item = char>>) -> String {
    read_until_inner(iter, |next: &char| !next.is_ascii_whitespace())
}

fn read_until_inner(
    iter: &mut Peekable<impl Iterator<Item = char>>,
    condition: impl Fn(&char) -> bool,
) -> String {
    let mut output = String::new();

    loop {
        let next = iter.peek().unwrap_or(&'\0');

        if condition(next) {
            output.push(*next);
            iter.next();
        } else {
            break;
        }
    }

    output
}

#[cfg(test)]
mod test {
    use crate::rules::lexer::{tokenize, RuleToken};

    #[test]
    fn base_test() {
        let tokens = tokenize(
            r#"
            matches /index.html {
                set_header "Server" "http-rs";
                return 301 "/index2.html";
            }
        "#,
        );

        let expected_tokens = vec![
            RuleToken::Matches,
            RuleToken::LitStr("/index.html".into()),
            RuleToken::LBrace,
            RuleToken::Ident("set_header".into()),
            RuleToken::LitStr("Server".into()),
            RuleToken::LitStr("http-rs".into()),
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
}
