use crate::header::is_header_valid;
use crate::http_version::HttpVersion;
use crate::request_method::RequestMethod;
use crate::utils::{skip_whitespace, IteratorUtils, StringUtils};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct Request {
    pub method: RequestMethod,
    pub url: String,
    pub version: HttpVersion,
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

    pub fn content_length(&self) -> Option<usize> {
        self.headers
            .get("Content-Length")
            .map(|content_length_value| content_length_value.parse::<usize>().unwrap())
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
                    Err("Could not find CRLF".into())
                };
            }

            values.push(*value);
        } else {
            return Err("Could not find CRLF".into());
        }
    }
}

fn parse_request_line<'a>(
    iterator: &mut (impl Iterator<Item = &'a u8> + IteratorUtils<'a, u8>),
) -> Result<(RequestMethod, String, HttpVersion)> {
    let method_bytes = iterator.take_while_copy(|byte| **byte != b' ');
    let method = RequestMethod::from_str(std::str::from_utf8(&method_bytes).unwrap());

    let url_bytes = iterator.take_while_copy(|byte| **byte != b' ');
    let url = String::from_vec(url_bytes);

    let version_bytes = take_until_crlf(iterator)?;
    let version = HttpVersion::from_str(std::str::from_utf8(&version_bytes).unwrap());

    match (method, version) {
        (Ok(method), Ok(version)) if !url.is_empty() && version == HttpVersion::Http1_1 => {
            Ok((method, url, version))
        }
        _ => Err("Request line parsing error".into()),
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

            return Err("Found CR without LF in header line".into());
        }

        let header = peekable_iterator.take_while_copy(|byte| **byte != b':');
        skip_whitespace(&mut peekable_iterator);
        let header_value = take_until_crlf(&mut peekable_iterator)?;

        let header_name = String::from_vec(header);
        let header_value = String::from_vec(header_value);

        if !is_header_valid(&header_name, &header_value) {
            return Err("Invalid header".into());
        }

        headers.insert(header_name, header_value);
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
