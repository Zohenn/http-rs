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
        matches!(self, RequestMethod::Get | RequestMethod::Head | RequestMethod:: Options)
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