use crate::http_version::HttpVersion;
use crate::response_status_code::ResponseStatusCode;
use crate::utils::StringUtils;
use std::collections::HashMap;

const SPACE: u8 = b' ';
static CRLF: [u8; 2] = [b'\r', b'\n'];

#[derive(Debug)]
pub struct Response {
    version: HttpVersion,
    status_code: ResponseStatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[allow(dead_code)]
impl Response {
    pub fn version(&self) -> &HttpVersion {
        &self.version
    }

    pub fn status_code(&self) -> &ResponseStatusCode {
        &self.status_code
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn body(&self) -> &Vec<u8> {
        &self.body
    }

    pub fn set_status_code(&mut self, status_code: ResponseStatusCode) {
        self.status_code = status_code;
    }

    pub fn set_header(&mut self, header_name: &str, header_value: &str) {
        self.headers.insert(header_name.into(), header_value.into());
    }

    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
    }

    pub(crate) fn as_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = vec![];

        bytes.append(&mut self.version.as_bytes());
        bytes.push(SPACE);
        bytes.append(&mut self.status_code.as_bytes());
        bytes.extend_from_slice(&CRLF);

        for (header_name, header_value) in self.headers.iter() {
            bytes.append(&mut header_name.as_bytes_vec());
            bytes.push(b':');
            bytes.push(SPACE);
            bytes.append(&mut header_value.as_bytes_vec());
            bytes.extend_from_slice(&CRLF);
        }

        bytes.extend_from_slice(&CRLF);
        bytes.extend_from_slice(&self.body);

        bytes
    }

    pub fn builder() -> ResponseBuilder {
        ResponseBuilder::new()
    }
}

#[derive(Debug)]
pub struct ResponseBuilder {
    response: Response,
}

#[allow(clippy::new_without_default)]
impl ResponseBuilder {
    pub fn new() -> Self {
        ResponseBuilder {
            response: Response {
                version: HttpVersion::Http1_1,
                status_code: ResponseStatusCode::Ok,
                headers: HashMap::new(),
                body: vec![],
            },
        }
    }

    pub fn status_code(mut self, status_code: ResponseStatusCode) -> Self {
        self.response.status_code = status_code;

        self
    }

    pub fn header(mut self, header_name: &str, header_value: &str) -> Self {
        self.response
            .headers
            .insert(String::from(header_name), String::from(header_value));

        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.response.body = body;

        self
    }

    pub fn text_body(mut self, body: &str) -> Self {
        self.response.body = body.as_bytes().to_vec();

        self
    }

    pub fn get(self) -> Response {
        if !self.response.body.is_empty() && !self.response.headers.contains_key("Content-Length") {
            let len = self.response.body.len();
            return self.header("Content-Length", &len.to_string()).response;
        }

        self.response
    }
}

#[cfg(test)]
mod test {
    mod response {
        use crate::response::Response;
        use crate::response_status_code::ResponseStatusCode;

        #[test]
        fn correct_as_bytes_representation() {
            let response = Response::builder()
                .status_code(ResponseStatusCode::Ok)
                .header("Content-Type", "text/plain")
                .body(vec![b'1', b'2', b'3'])
                .get();
            let bytes = response.as_bytes();
            let response_str = std::str::from_utf8(&bytes).unwrap();

            let mut found_body = false;
            for (index, line) in response_str.split("\r\n").enumerate() {
                if index == 0 {
                    assert_eq!(line, "HTTP/1.1 200 OK");
                } else if !found_body {
                    if line.is_empty() {
                        found_body = true;
                        continue;
                    }
                    assert!(matches!(
                        line,
                        "Content-Type: text/plain" | "Content-Length: 3"
                    ));
                } else {
                    assert_eq!(line, "123");
                }
            }
        }
    }
}
