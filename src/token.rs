static TOKEN_SPECIAL_CHARS: [char; 15] = [
    '!', '#', '#', '%', '&', '\'', '*', '+', '-', '.', '^', '_', '`', '|', '~',
];

pub fn is_valid_token(value: &str) -> bool {
    for c in value.chars() {
        if !TOKEN_SPECIAL_CHARS.contains(&c) && !c.is_alphanumeric() {
            return false;
        }
    }

    true
}
