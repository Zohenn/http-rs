mod request;
mod request_method;
mod response;
mod response_status_code;
mod utils;
mod content;

use crate::request::parse_request;
use crate::response::{Response};
use crate::response_status_code::ResponseStatusCode;
use std::io::{Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::ops::Add;
use crate::content::{serve_content};
use crate::utils::StringUtils;

fn handle_connection(stream: &mut TcpStream) -> Result<()> {
    // let mut raw_request = String::new();
    let mut stream_buf: [u8; 255] = [0; 255];
    let mut request_bytes: Vec<u8> = Vec::new();

    loop {
        let read_result = stream.read(stream_buf.as_mut_slice());
        match read_result {
            Ok(n) => {
                request_bytes.extend_from_slice(stream_buf.take(n as u64).into_inner());
                // raw_request = raw_request.add(std::str::from_utf8(&stream_buf).unwrap());
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

    // println!("{request:#?}\n");

    let mut response = if let Ok(request) = request {
        serve_content(&request)
    } else {
        Response::error_response(ResponseStatusCode::BadRequest)
    };

    stream.write_all(&response.as_bytes())?;

    Ok(())
}

pub fn run() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:80")?;

    for stream in listener.incoming() {
        handle_connection(&mut stream?)?;
    }

    Ok(())
}
