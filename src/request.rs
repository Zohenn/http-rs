use crate::request_method::RequestMethod;
use std::collections::HashMap;
use std::fmt;

type Result<T> = std::result::Result<T, RequestParseError>;
type MutableIterator<'a, T> = dyn Iterator<Item = &'a T>;
type MutableByteIterator<'a> = MutableIterator<'a, u8>;

// #[derive(Debug)]
pub struct Request {
    pub method: RequestMethod,
    pub url: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
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

fn take_while_clone<'a, T>(
    iterator: &mut dyn Iterator<Item = &'a T>,
    predicate: impl FnMut(&&T) -> bool,
) -> Vec<T>
where
    T: Copy,
{
    iterator.take_while(predicate).map(|value| *value).collect()
}

fn take_until_crlf<'a>(iterator: &mut dyn Iterator<Item = &'a u8>) -> Result<Vec<u8>>
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
            break;
        }
    }

    Err(RequestParseError)
}

fn parse_request_line<'a>(
    iterator: &mut impl Iterator<Item = &'a u8>,
) -> Result<(RequestMethod, String, String)> {
    let method_bytes = take_while_clone(iterator, |byte| **byte != b' ');
    let method = RequestMethod::from_str(std::str::from_utf8(method_bytes.as_slice()).unwrap());

    let url_bytes = take_while_clone(iterator, |byte| **byte != b' ');
    let url = String::from(std::str::from_utf8(url_bytes.as_slice()).unwrap());

    let version_bytes = take_until_crlf(iterator)?;
    let version = String::from(std::str::from_utf8(version_bytes.as_slice()).unwrap());

    if method.is_some() && !url.is_empty() {
        Ok((method.unwrap(), url, version))
    } else {
        Err(RequestParseError)
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

            if *last_byte.unwrap_or(&&0u8) == b'\n' {
                return Ok(headers);
            }

            return Err(RequestParseError)
        }

        let header = take_while_clone(&mut peekable_iterator, |byte| **byte != b':');
        peekable_iterator.next();
        let header_value = take_until_crlf(&mut peekable_iterator)?;

        headers.insert(
            String::from(std::str::from_utf8(header.as_slice()).unwrap()),
            String::from(std::str::from_utf8(header_value.as_slice()).unwrap()),
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
        body: bytes_iter.map(|byte| *byte).collect(),
    })
}
