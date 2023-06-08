use std::str::FromStr;

#[derive(Debug)]
pub enum RequestMethod {
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
}

impl FromStr for RequestMethod {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, ()> {
        match value {
            "GET" => Ok(RequestMethod::Get),
            "HEAD" => Ok(RequestMethod::Head),
            "POST" => Ok(RequestMethod::Post),
            "PUT" => Ok(RequestMethod::Put),
            "PATCH" => Ok(RequestMethod::Patch),
            "DELETE" => Ok(RequestMethod::Delete),
            _ => Err(()),
        }
    }
}