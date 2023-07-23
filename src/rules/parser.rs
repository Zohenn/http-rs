use crate::rules::Rule;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, LitStr};

mod kw {
    syn::custom_keyword!(matches);
}

struct RuleMetadata {
    pub pattern: LitStr,
}

impl Parse for RuleMetadata {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();

        input.parse::<kw::matches>()?;

        let pattern = parse_pattern_token(input)?;

        input.parse::<proc_macro2::Group>()?;

        // while !input.is_empty() {
        //     let a: proc_macro2::TokenTree = input.parse()?;
        // }

        let mut metadata = RuleMetadata {
            pattern: LitStr::new(&pattern, Span::call_site()),
        };

        Ok(metadata)
    }
}

fn parse_pattern_token(input: ParseStream) -> syn::Result<String> {
    let mut pattern_tokens = TokenStream::new();
    while !input.peek(syn::token::Brace) {
        let next: proc_macro2::TokenTree = input.parse()?;
        pattern_tokens.extend(Some(next));
    }

    Ok(pattern_tokens
        .to_token_stream()
        .into_iter()
        .map(|token| token.to_string())
        .collect::<String>())
}

fn parse_tokens(tokens: TokenStream) -> Rule {
    Rule {
        pattern: "123".to_string(),
    }
}

fn parse_str(source: &str) -> Rule {
    Rule {
        pattern: "123".to_string(),
    }
}

#[cfg(test)]
mod test {
    use crate::rules::parser::parse_str;

    #[test]
    fn test() {
        let rule = parse_str(
            r#"
            matches /index.html {
                return 301 /index2.html
            }
        "#,
        );

        assert_eq!(rule.pattern, "/index.html");
    }
}
