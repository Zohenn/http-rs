use crate::header::is_header_valid;
use crate::http_version::HttpVersion;
use crate::request_method::RequestMethod;
use crate::utils::{skip_whitespace, IteratorUtils, StringUtils};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub enum RequestBodyType {
    None,
    ContentLength,
    TransferEncodingChunked,
}

pub struct Request {
    pub method: RequestMethod,
    pub url: String,
    pub version: HttpVersion,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    // todo: this should be case insensitive, at least for header names
    // Probably requires HashMap<String, String> to be changed to Vec<(String, String)>,
    // and then using string.eq_ignore_ascii_case() when iterating the vector to find given header.
    // Vec can be used as there will be at most 30-40 headers per request, usually much less,
    // so it will probably be even faster than HashMap that has constant lookup times
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

    pub fn body_type(&self) -> RequestBodyType {
        if let Some(_length) = self.content_length() {
            RequestBodyType::ContentLength
        } else if self.has_header("Transfer-Encoding", Some("chunked")) {
            RequestBodyType::TransferEncodingChunked
        } else {
            RequestBodyType::None
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut str = format!("{} {} {}\r\n", self.method, self.url, self.version);

        for (name, value) in &self.headers {
            str += format!("{}: {}\r\n", name, value).as_str();
        }

        str += "\r\n";

        let mut bytes = Vec::from(str);

        if !self.body.is_empty() {
            bytes.append(&mut self.body.clone());
        }

        bytes
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

pub fn parse_chunked_body(body: Vec<u8>) -> Result<Vec<u8>> {
    let mut parsed: Vec<u8> = vec![];
    let mut iterator = body.iter();

    loop {
        let mut peekable_iterator = iterator.by_ref().peekable();
        let chunk_len_bytes = take_until_crlf(&mut peekable_iterator)?;
        let chunk_len_str = std::str::from_utf8(&chunk_len_bytes)?;
        let chunk_len = chunk_len_str.parse::<usize>()?;

        if peekable_iterator.peek().is_none() {
            return Err("Incorrect chunked body".into());
        }

        let mut chunk_bytes = take_until_crlf(&mut peekable_iterator)?;

        if chunk_bytes.len() != chunk_len {
            return Err("Incorrect chunk length".into());
        }

        if chunk_len == 0 {
            return Ok(parsed);
        } else {
            parsed.append(&mut chunk_bytes);
        }
    }
}

pub fn parse_request(bytes: &[u8]) -> Result<Request> {
    let mut bytes_iter = bytes.iter();
    let (method, url, version) = parse_request_line(bytes_iter.by_ref())?;
    let headers = parse_headers(bytes_iter.by_ref())?;

    let mut request = Request {
        method,
        url,
        version,
        headers,
        body: vec![],
    };

    match request.body_type() {
        RequestBodyType::ContentLength => {
            request.body = bytes_iter.copied().collect();
        }
        RequestBodyType::TransferEncodingChunked => {
            request.body = parse_chunked_body(bytes_iter.copied().collect())?;
        }
        RequestBodyType::None => {}
    }

    Ok(request)
}

#[cfg(test)]
mod tests {
    mod parse_request_line {
        use crate::http_version::HttpVersion;
        use crate::request::parse_request_line;
        use crate::request_method::RequestMethod;
        use std::error::Error;

        fn msg_result(msg: &str) -> Result<(RequestMethod, String, HttpVersion), Box<dyn Error>> {
            parse_request_line(&mut format!("{}\r\n\r\n", msg).as_bytes().iter())
        }

        #[test]
        fn err_with_invalid_method() {
            let result = msg_result("GET123 /index.html HTTP/1.1");
            assert!(result.is_err());
        }

        #[test]
        fn err_with_empty_url() {
            let result = msg_result("GET  HTTP/1.1");
            assert!(result.is_err());
        }

        #[test]
        fn err_with_invalid_http_version() {
            let result = msg_result("GET /index.html HTTP/12.34");
            assert!(result.is_err());
        }

        #[test]
        fn err_with_whitespace_after_http_version() {
            let result = msg_result("GET /index.html HTTP/1.1 ");
            assert!(result.is_err());
        }

        #[test]
        fn err_with_malformed_msg() {
            let result = msg_result("GET/index.htmlHTTP/1.1");
            assert!(result.is_err());
        }
    }

    mod parse_headers {
        use crate::request::parse_headers;
        use std::collections::HashMap;
        use std::error::Error;

        fn msg_result(msg: &str) -> Result<HashMap<String, String>, Box<dyn Error>> {
            parse_headers(&mut format!("{}\r\n\r\n", msg).as_bytes().iter())
        }

        #[test]
        fn err_with_whitespace_before_header_name() {
            let result = msg_result("Content-Type : text/html\r\n  Content-Length: 123");
            assert!(result.is_err());
        }

        #[test]
        fn err_with_whitespace_before_colon() {
            let result = msg_result("Content-Type : text/html");
            assert!(result.is_err());
        }

        #[test]
        fn err_with_non_numeric_value_when_numeric_expected() {
            let result = msg_result("Content-Length: text/html");
            assert!(result.is_err());
        }
    }

    mod parse_request {
        use crate::http_version::HttpVersion;
        use crate::request::{parse_request, Request};
        use crate::request_method::RequestMethod;
        use std::collections::HashMap;
        use std::error::Error;

        static TEST_MESSAGE: &str =
            "POST /index.html HTTP/1.1\r\nContent-Type: text/plain\r\nContent-Length: 3\r\n\r\n123";

        fn msg_result(msg: &str) -> Result<Request, Box<dyn Error>> {
            parse_request(msg.as_bytes())
        }

        #[test]
        fn result_contains_request_line_info() {
            let result = msg_result(TEST_MESSAGE).unwrap();
            assert_eq!(result.method, RequestMethod::Post);
            assert_eq!(result.url, "/index.html");
            assert_eq!(result.version, HttpVersion::Http1_1);
        }

        #[test]
        fn result_contains_headers() {
            let result = msg_result(TEST_MESSAGE).unwrap();
            let headers = HashMap::from([
                ("Content-Type".to_string(), "text/plain".to_string()),
                ("Content-Length".to_string(), "3".to_string()),
            ]);
            assert_eq!(result.headers, headers);
        }

        #[test]
        fn leftover_bytes_copied_to_body() {
            let result = msg_result(TEST_MESSAGE);
            assert_eq!(result.unwrap().body, vec![b'1', b'2', b'3']);
        }
    }
}
