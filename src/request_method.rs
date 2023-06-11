use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum RequestMethod {
    Get,
    Head,
    Options,
    Post,
    Put,
    Patch,
    Delete,
}

impl RequestMethod {
    pub fn is_safe(&self) -> bool {
        matches!(
            self,
            RequestMethod::Get | RequestMethod::Head | RequestMethod::Options
        )
    }

    pub fn safe_methods_str() -> String {
        vec![
            RequestMethod::Get,
            RequestMethod::Head,
            RequestMethod::Options,
        ]
            .iter()
            .map(|m| m.to_string().to_uppercase())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

impl Display for RequestMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for RequestMethod {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, ()> {
        match value {
            "GET" => Ok(RequestMethod::Get),
            "HEAD" => Ok(RequestMethod::Head),
            "OPTIONS" => Ok(RequestMethod::Options),
            "POST" => Ok(RequestMethod::Post),
            "PUT" => Ok(RequestMethod::Put),
            "PATCH" => Ok(RequestMethod::Patch),
            "DELETE" => Ok(RequestMethod::Delete),
            _ => Err(()),
        }
    }
}
