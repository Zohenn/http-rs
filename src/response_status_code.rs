use crate::utils::StringUtils;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
// not every status code defined by the spec is here, but I don't care
pub enum ResponseStatusCode {
    // Informational responses (100 - 199)
    Continue = 100,
    SwitchingProtocols = 101,

    // Successful responses (200 - 299)
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,

    // Redirection messages (300 - 399)
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    TemporaryRedirect = 307,
    PermanentRedirect = 308,

    // Client error responses (400 - 499)
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    ImATeapot = 418,
    TooManyRequests = 429,

    // Server error responses (500 - 599)
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    HttpVersionNotSupported = 505,
}

impl ResponseStatusCode {
    pub fn is_error(&self) -> bool {
        *self as u16 >= 400
    }
}

impl Display for ResponseStatusCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string_value = match self {
            ResponseStatusCode::Continue => "Continue",
            ResponseStatusCode::SwitchingProtocols => "Switching Protocols",
            ResponseStatusCode::Ok => "OK",
            ResponseStatusCode::Created => "Created",
            ResponseStatusCode::Accepted => "Accepted",
            ResponseStatusCode::NoContent => "No Content",
            ResponseStatusCode::MovedPermanently => "Moved Permanently",
            ResponseStatusCode::Found => "Found",
            ResponseStatusCode::SeeOther => "See Other",
            ResponseStatusCode::NotModified => "Not Modified",
            ResponseStatusCode::TemporaryRedirect => "Temporary Redirect",
            ResponseStatusCode::PermanentRedirect => "Permanent Redirect",
            ResponseStatusCode::BadRequest => "Bad Request",
            ResponseStatusCode::Unauthorized => "Unauthorized",
            ResponseStatusCode::Forbidden => "Forbidden",
            ResponseStatusCode::NotFound => "Not Found",
            ResponseStatusCode::MethodNotAllowed => "Method Not Allowed",
            ResponseStatusCode::ImATeapot => "I'm a teapot",
            ResponseStatusCode::TooManyRequests => "Too Many Requests",
            ResponseStatusCode::InternalServerError => "Internal Server Error",
            ResponseStatusCode::NotImplemented => "Not Implemented",
            ResponseStatusCode::BadGateway => "Bad Gateway",
            ResponseStatusCode::ServiceUnavailable => "Service Unavailable",
            ResponseStatusCode::GatewayTimeout => "Gateway Timeout",
            ResponseStatusCode::HttpVersionNotSupported => "Http Version Not Supported",
        };

        write!(f, "{}", string_value)
    }
}

impl ResponseStatusCode {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = vec![];

        let mut code_string: Vec<u8> = (*self as u16).to_string().as_bytes_vec();
        let mut status_string: Vec<u8> = self.to_string().as_bytes_vec();

        bytes.append(&mut code_string);
        bytes.push(b' ');
        bytes.append(&mut status_string);

        bytes
    }
}
