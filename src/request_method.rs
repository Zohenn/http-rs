#[derive(Debug)]
pub enum RequestMethod {
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
}

impl RequestMethod {
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "GET" => Some(RequestMethod::Get),
            "HEAD" => Some(RequestMethod::Head),
            "POST" => Some(RequestMethod::Post),
            "PUT" => Some(RequestMethod::Put),
            "PATCH" => Some(RequestMethod::Patch),
            "DELETE" => Some(RequestMethod::Delete),
            _ => None,
        }
    }
}