#[cfg(test)]
mod tests {
    use super::*;
    use rules::rules;

    #[test]
    fn test2() {
        let rule = rules! {
            matches /index.html {
                return 301 /index2.html
            }
        };
    }
}
