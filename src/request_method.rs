#[derive(Debug)]
pub enum RequestMethod {
    GET,
    HEAD,
    POST,
    PUT,
    PATCH,
    DELETE,
}

impl RequestMethod {
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "GET" => Some(RequestMethod::GET),
            "HEAD" => Some(RequestMethod::HEAD),
            "POST" => Some(RequestMethod::POST),
            "PUT" => Some(RequestMethod::PUT),
            "PATCH" => Some(RequestMethod::PATCH),
            "DELETE" => Some(RequestMethod::DELETE),
            _ => None,
        }
    }
}