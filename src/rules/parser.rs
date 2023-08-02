use crate::response_status_code::ResponseStatusCode;
use crate::rules::grammar::file;
use crate::rules::lexer::{tokenize, RuleTokenKind};
use crate::rules::{Rule, RuleAction};
use std::fs::File;
use std::io::Read;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

enum RuleParseState {
    None,
    HasPattern,
}

pub fn parse_file(path: &str) -> Result<Vec<Rule>> {
    let mut file = File::open(path).unwrap();

    let mut file_contents = String::new();

    file.read_to_string(&mut file_contents).unwrap();

    parse_str(&file_contents)
}

fn parse_str(source: &str) -> Result<Vec<Rule>> {
    // parse_tokens(tokenize(source)?)
    file(tokenize(source)?)
}

fn parse_tokens(tokens: Vec<RuleTokenKind>) -> Result<Vec<Rule>> {
    let mut rules: Vec<Rule> = vec![];

    if tokens.is_empty() {
        return Ok(rules);
    }

    let mut state = RuleParseState::None;

    let mut token_iter = tokens.iter().peekable();
    let mut rule_builder = Rule::builder();

    while let Some(next_token) = token_iter.peek() {
        match state {
            RuleParseState::None => {
                let tokens_until_lbrace = token_iter
                    .by_ref()
                    .take_while(|token| !matches!(token, RuleTokenKind::LBrace))
                    .collect::<Vec<&RuleTokenKind>>();

                match tokens_until_lbrace[..] {
                    [RuleTokenKind::Matches, RuleTokenKind::LitStr(pattern)] => {
                        rule_builder = rule_builder.pattern(pattern.into());
                        state = RuleParseState::HasPattern;
                    }
                    _ => {
                        return Err(format!(
                            "Rule must start with 'matches *pattern*' {:?}",
                            tokens_until_lbrace
                        )
                        .into())
                    }
                }
            }
            RuleParseState::HasPattern => match next_token {
                RuleTokenKind::RBrace => {
                    token_iter.next();

                    rules.push(rule_builder.get());
                    rule_builder = Rule::builder();
                    state = RuleParseState::None;
                }
                _ => {
                    let tokens_until_semicolon = token_iter
                        .by_ref()
                        .take_while(|token| !matches!(token, RuleTokenKind::Semicolon))
                        .collect::<Vec<&RuleTokenKind>>();

                    let action = match tokens_until_semicolon[..] {
                        [RuleTokenKind::Return, RuleTokenKind::LitInt(response_code), RuleTokenKind::LitStr(location)] => {
                            parse_return(response_code, Some(location))?
                        }
                        [RuleTokenKind::Return, RuleTokenKind::LitInt(response_code)] => {
                            parse_return(response_code, None)?
                        }
                        [RuleTokenKind::Ident(function), RuleTokenKind::LitStr(arg1), RuleTokenKind::LitStr(arg2)] => {
                            parse_2_arg_function(function, arg1, arg2)?
                        }
                        _ => {
                            return Err(
                                format!("Unexpected tokens {tokens_until_semicolon:?}").into()
                            )
                        }
                    };

                    rule_builder = rule_builder.add_action(action);
                }
            },
        }
    }

    Ok(rules)
}

fn parse_return(response_code: &str, additional_data: Option<&str>) -> Result<RuleAction> {
    let response_code = response_code
        .parse::<u16>()
        .map_err(|_| "Incorrect response code")?;

    let response_code = ResponseStatusCode::try_from(response_code).unwrap();

    let action = if response_code.is_redirect() {
        let location = additional_data.ok_or::<Box<dyn std::error::Error>>(
            "Return with redirect must be followed with location url".into(),
        )?;
        RuleAction::RedirectReturn(response_code, location.into())
    } else {
        RuleAction::CustomReturn(response_code, additional_data.map(|v| v.into()))
    };

    Ok(action)
}

fn parse_2_arg_function(function: &str, arg1: &str, arg2: &str) -> Result<RuleAction> {
    let action = match function {
        "set_header" => RuleAction::SetHeader(arg1.into(), arg2.into()),
        _ => return Err(format!("Unexpected identifier: {function}").into()),
    };

    Ok(action)
}

#[cfg(test)]
mod test {
    use crate::response_status_code::ResponseStatusCode;
    use crate::rules::parser::parse_str;
    use crate::rules::RuleAction;

    #[test]
    fn base_test() {
        let rules = parse_str(
            r#"
            matches / {
                set_header "Server" "http-rs";
            }

            matches /index.html {
                return 301 "/index2.html";
            }
        "#,
        )
        .unwrap();

        let rule = rules.first().unwrap();

        assert_eq!(rule.pattern, "/");
        assert_eq!(
            rule.actions[0],
            RuleAction::SetHeader("Server".into(), "http-rs".into())
        );

        let rule = rules.get(1).unwrap();

        assert_eq!(rule.pattern, "/index.html");
        assert_eq!(
            rule.actions[0],
            RuleAction::CustomReturn(
                ResponseStatusCode::MovedPermanently,
                Some("/index2.html".into())
            )
        );
    }
}
