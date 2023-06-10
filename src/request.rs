use crate::request_method::RequestMethod;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use crate::utils::{StringUtils, IteratorUtils};

type Result<T> = std::result::Result<T, RequestParseError>;

pub struct Request {
    pub method: RequestMethod,
    pub url: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn has_header(&self, header_name: &str, header_value: Option<&str>) -> bool {
        match (self.headers.get(header_name), header_value) {
            (Some(value), Some(header_value)) => header_value == value,
            (Some(_), None) => true,
            (None, _) => false,
        }
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("version", &self.version)
            .field("headers", &self.headers)
            .field("body", &format!("{} bytes", self.body.len()))
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct RequestParseError;

fn take_until_crlf<'a>(iterator: &mut impl Iterator<Item = &'a u8>) -> Result<Vec<u8>>
where
    u8: Copy,
{
    let mut values: Vec<u8> = vec![];
    let mut next_value: Option<&u8>;

    loop {
        next_value = iterator.next();

        if let Some(value) = next_value {
            if *value == b'\n' {
                return if *values.last().unwrap_or(&0u8) == b'\r' {
                    values.pop();

                    Ok(values)
                } else {
                    Err(RequestParseError)
                };
            }

            values.push(*value);
        } else {
            return Err(RequestParseError)
        }
    }
}

fn parse_request_line<'a>(
    iterator: &mut (impl Iterator<Item = &'a u8> + IteratorUtils<'a, u8>),
) -> Result<(RequestMethod, String, String)> {
    let method_bytes = iterator.take_while_copy(|byte| **byte != b' ');
    let method = RequestMethod::from_str(std::str::from_utf8(&method_bytes).unwrap());

    let url_bytes = iterator.take_while_copy(|byte| **byte != b' ');
    let url = String::from_vec(url_bytes);

    let version_bytes = take_until_crlf(iterator)?;
    let version = String::from_vec(version_bytes);

    match method {
        Ok(method) if !url.is_empty() => Ok((method, url, version)),
        _ => Err(RequestParseError),
    }
}

fn parse_headers<'a>(
    iterator: &mut impl Iterator<Item = &'a u8>,
) -> Result<HashMap<String, String>> {
    let mut headers: HashMap<String, String> = HashMap::new();

    loop {
        let mut peekable_iterator = iterator.peekable();
        // check if the first value of current line is CRLF
        if **peekable_iterator.peek().unwrap_or(&&0u8) == b'\r' {
            peekable_iterator.next();
            let last_byte = peekable_iterator.next();

            if *last_byte.unwrap_or(&0u8) == b'\n' {
                return Ok(headers);
            }

            return Err(RequestParseError)
        }

        let header = peekable_iterator.take_while_copy(|byte| **byte != b':');
        peekable_iterator.next();
        let header_value = take_until_crlf(&mut peekable_iterator)?;

        headers.insert(
            String::from_vec(header),
            String::from_vec(header_value),
        );
    }
}

pub fn parse_request(bytes: &[u8]) -> Result<Request> {
    let mut bytes_iter = bytes.iter();
    let (method, url, version) = parse_request_line(bytes_iter.by_ref())?;
    let headers = parse_headers(bytes_iter.by_ref())?;

    Ok(Request {
        method,
        url,
        version,
        headers,
        body: bytes_iter.copied().collect(),
    })
}
