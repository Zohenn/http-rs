use std::iter::Peekable;

#[derive(Clone, Debug, PartialEq)]
pub enum RuleToken {
    Ident(String),

    LBrace,
    RBrace,

    // literals
    LitStr(String),
    LitInt(String),

    // keywords
    Matches,
    Return,

    Eof,
}

fn tokenize(input: &str) -> Vec<RuleToken> {
    let input_bytes = input.to_string().into_bytes();
    let mut iter = input_bytes.into_iter().peekable();

    let mut tokens: Vec<RuleToken> = vec![];

    skip_whitespace(&mut iter);

    while iter.peek().is_some() {
        let character = iter.next().unwrap();
        let token = match character {
            b'{' => RuleToken::LBrace,
            b'}' => RuleToken::RBrace,
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let ident = String::from(character as char);
                let ident = ident + &read_ident(&mut iter);

                match &*ident {
                    "matches" => RuleToken::Matches,
                    "return" => RuleToken::Return,
                    _ => RuleToken::Ident(ident),
                }
            }
            b'"' => {
                let lit = read_string(&mut iter);

                // swallow ending "
                iter.next();

                RuleToken::LitStr(lit)
            }
            b'0'..=b'9' => {
                let lit = String::from(character as char) + &read_int(&mut iter);

                RuleToken::LitInt(lit)
            }
            _ => panic!("Unexpected token: {}", character as char),
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

fn skip_whitespace(iter: &mut Peekable<impl Iterator<Item = u8>>) {
    while iter.peek().unwrap_or(&0u8).is_ascii_whitespace() {
        iter.next();
    }
}

macro_rules! read_until_helper {
    ($name:ident, $condition:expr) => {
        fn $name(iter: &mut Peekable<impl Iterator<Item = u8>>) -> String {
            let mut output = String::new();

            loop {
                let next = iter.peek().unwrap_or(&0u8);

                if $condition(next) {
                    output.push(*next as char);
                    iter.next();
                } else {
                    break;
                }
            }

            output
        }
    };
}

read_until_helper!(read_ident, |next: &u8| {
    next.is_ascii_alphabetic() || next == &b'_'
});

read_until_helper!(read_string, |next: &u8| { next != &b'"' });

read_until_helper!(read_int, |next: &u8| { next.is_ascii_digit() });

read_until_helper!(read_until_whitespace, |next: &u8| {
    !next.is_ascii_whitespace()
});

#[cfg(test)]
mod test {
    use crate::rules::lexer::{tokenize, RuleToken};

    #[test]
    fn base_test() {
        let tokens = tokenize(
            r#"
            matches /index.html {
                set_header "Server" "http-rs"
                return 301 "/index2.html"
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
            RuleToken::Return,
            RuleToken::LitInt("301".into()),
            RuleToken::LitStr("/index2.html".into()),
            RuleToken::RBrace,
        ];

        for (index, token) in tokens.iter().enumerate() {
            let expected = expected_tokens.get(index).unwrap_or(&RuleToken::Eof);
            println!("expected: {expected:?}, got: {token:?}");
            assert_eq!(token, expected);
        }
    }
}
