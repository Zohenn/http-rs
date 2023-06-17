use crate::utils::StringUtils;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum HttpVersion {
    Http0_9,
    Http1_0,
    Http1_1,
    Http2,
}

impl HttpVersion {
    pub fn as_bytes(&self) -> Vec<u8> {
        self.to_string().as_bytes_vec()
    }
}

impl FromStr for HttpVersion {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, ()> {
        match value {
            "HTTP/0.9" => Ok(HttpVersion::Http0_9),
            "HTTP/1.0" => Ok(HttpVersion::Http1_0),
            "HTTP/1.1" => Ok(HttpVersion::Http1_1),
            "HTTP/2" => Ok(HttpVersion::Http2),
            _ => Err(()),
        }
    }
}

impl Display for HttpVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string_value = match self {
            HttpVersion::Http0_9 => "HTTP/0.9",
            HttpVersion::Http1_0 => "HTTP/1.0",
            HttpVersion::Http1_1 => "HTTP/1.1",
            HttpVersion::Http2 => "HTTP/2",
        };

        write!(f, "{}", string_value)
    }
}
