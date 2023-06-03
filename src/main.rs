mod request;
mod request_method;
mod response;
mod response_status_code;
mod utils;

use crate::request::parse_request;
use crate::response::{Response};
use crate::response_status_code::ResponseStatusCode;
use std::io::{Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::ops::Add;
use crate::utils::StringUtils;

fn handle_connection(stream: &mut TcpStream) -> Result<()> {
    let mut raw_request = String::new();
    let mut stream_buf: [u8; 255] = [0; 255];
    let mut request_bytes: Vec<u8> = Vec::new();

    loop {
        let read_result = stream.read(stream_buf.as_mut_slice());
        match read_result {
            Ok(n) => {
                request_bytes.extend_from_slice(stream_buf.take(n as u64).into_inner());
                raw_request = raw_request.add(std::str::from_utf8(&stream_buf).unwrap());
                if n < stream_buf.len() {
                    break;
                }
                stream_buf.fill(0);
            }
            Err(e) => panic!("{}", e),
        }
    }

    // println!("Received message:\n{raw_request}\n");

    let request = parse_request(request_bytes.as_slice());
    // todo: 400 if request is malformed
    println!("{request:#?}\n");

    let mut response = Response::builder()
        .status_code(ResponseStatusCode::Ok)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(String::from("<html><body><h1>Hello</h2></body></html>").as_bytes_vec())
        .get();

    stream.write_all(&response.as_bytes())?;

    Ok(())
}

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:80")?;

    for stream in listener.incoming() {
        handle_connection(&mut stream?)?;
    }

    Ok(())
}
