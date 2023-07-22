use proc_macro2::TokenTree::Punct;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Ident, LitStr};

mod rule;

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

#[proc_macro]
pub fn rules(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let rules: RuleMetadata = parse_macro_input!(input);

    let pattern = rules.pattern.clone();

    let tokens = quote! {
        http_rs::rule::Rule {
            pattern: #pattern.to_string(),
        }
    };

    tokens.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test() {
        let input: proc_macro2::TokenStream = quote!(
            matches /index.html {
                return 301 /index2.html
            }
        );

        println!("{input:?}");
    }
}
