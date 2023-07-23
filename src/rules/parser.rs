use crate::rules::lexer::{tokenize, RuleToken};
use crate::rules::{Rule, RuleAction};

enum RuleParseState {
    None,
    HasPattern,
}

fn parse_str(source: &str) -> Vec<Rule> {
    parse_tokens(tokenize(source))
}

fn parse_tokens(tokens: Vec<RuleToken>) -> Vec<Rule> {
    let mut rules: Vec<Rule> = vec![];

    if tokens.is_empty() {
        return rules;
    }

    let mut state = RuleParseState::None;

    let mut token_iter = tokens.iter().peekable();
    let mut rule_builder = Rule::builder();

    while let Some(next_token) = token_iter.peek() {
        match state {
            RuleParseState::None => {
                let tokens_until_lbrace = token_iter
                    .by_ref()
                    .take_while(|token| !matches!(token, RuleToken::LBrace))
                    .collect::<Vec<&RuleToken>>();

                match tokens_until_lbrace[..] {
                    [RuleToken::Matches, RuleToken::LitStr(pattern)] => {
                        rule_builder = rule_builder.pattern(pattern.into());
                        state = RuleParseState::HasPattern;
                    }
                    _ => panic!(
                        "Rule must start with 'matches *pattern*' {:?}",
                        tokens_until_lbrace
                    ),
                }
            }
            RuleParseState::HasPattern => match next_token {
                RuleToken::RBrace => {
                    token_iter.next();

                    rules.push(rule_builder.get());
                    rule_builder = Rule::builder();
                    state = RuleParseState::None;
                }
                _ => {
                    let tokens_until_semicolon = token_iter
                        .by_ref()
                        .take_while(|token| !matches!(token, RuleToken::Semicolon))
                        .collect::<Vec<&RuleToken>>();

                    let action = match tokens_until_semicolon[..] {
                        [RuleToken::Return, RuleToken::LitInt(response_code), RuleToken::LitStr(location)] => {
                            parse_return(response_code, location)
                        }
                        [RuleToken::Ident(function), RuleToken::LitStr(arg1), RuleToken::LitStr(arg2)] => {
                            parse_2_arg_function(function, arg1, arg2)
                        }
                        _ => panic!("Unexpected tokens {tokens_until_semicolon:?}"),
                    };

                    rule_builder = rule_builder.add_action(action);
                }
            },
        }
    }

    rules
}

fn parse_return(response_code: &str, location: &str) -> RuleAction {
    let response_code = response_code
        .parse::<u16>()
        .expect("Incorrect response_code");

    RuleAction::CustomReturn(response_code, location.into())
}

fn parse_2_arg_function(function: &str, arg1: &str, arg2: &str) -> RuleAction {
    match function {
        "set_header" => RuleAction::SetHeader(arg1.into(), arg2.into()),
        _ => panic!("Unexpected identifier: {function}"),
    }
}

#[cfg(test)]
mod test {
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
        );

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
            RuleAction::CustomReturn(301, "/index2.html".into())
        );
    }
}
