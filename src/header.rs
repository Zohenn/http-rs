use crate::token::is_valid_token;

pub static HEADERS_WITH_NUMBER_VALUES: [&str; 1] = ["Content-Length"];

pub fn is_header_valid(header_name: &str, header_value: &str) -> bool {
    if !is_valid_token(header_name) {
        return false;
    }

    if HEADERS_WITH_NUMBER_VALUES.contains(&header_name) {
        return header_value.parse::<usize>().is_ok();
    }

    true
}
