use crate::response_status_code::ResponseStatusCode;
use crate::utils::StringUtils;
use std::collections::HashMap;

const SPACE: u8 = b' ';
static CRLF: [u8; 2] = [b'\r', b'\n'];

#[derive(Debug)]
pub struct Response {
    version: String,
    status_code: ResponseStatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Response {
    pub fn version(&self) -> &str {
        &self.version.as_str()
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

    pub fn as_bytes(&mut self) -> Vec<u8> {
        let mut bytes: Vec<u8> = vec![];

        bytes.append(&mut self.version.as_bytes_vec());
        bytes.push(SPACE);
        bytes.append(&mut self.status_code.as_bytes());
        bytes.extend_from_slice(&CRLF);

        for (header_name, header_value) in self.headers.iter() {
            bytes.append(&mut header_name.as_bytes_vec());
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

    pub fn error_response(status_code: ResponseStatusCode) -> Response {
        ResponseBuilder::new()
            .status_code(status_code)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(
                format!(
                    "<html><body><h1 style='text-align: center'>{} {}</h1></body></html>",
                    status_code as u16,
                    status_code
                )
                .as_bytes_vec(),
            )
            .get()
    }
}

#[derive(Debug)]
pub struct ResponseBuilder {
    response: Response,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        ResponseBuilder {
            response: Response {
                version: String::from("HTTP/1.1"),
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

    pub fn get(self) -> Response {
        self.response
    }
}
