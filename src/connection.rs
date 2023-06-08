use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

pub struct Connection<'a> {
    stream: &'a mut TcpStream,
    https_config: Option<Arc<rustls::ServerConfig>>,
    tls_connection: Option<rustls::ServerConnection>,
}

impl<'a> Connection<'a> {
    pub fn new(stream: &'a mut TcpStream, https_config: Option<Arc<rustls::ServerConfig>>) -> Self {
        Connection {
            stream,
            https_config,
            tls_connection: None,
        }
    }

    pub fn read(&mut self) -> std::io::Result<Option<Vec<u8>>> {
        let mut request_bytes: Vec<u8> = Vec::new();

        if let Some(https_config) = self.https_config.clone() {
            self.tls_connection =
                Some(rustls::ServerConnection::new(https_config).unwrap());
        }

        if let Some(tls_connection) = &mut self.tls_connection {
            while tls_connection.is_handshaking() {
                tls_connection.read_tls(self.stream)?;
                match tls_connection.process_new_packets() {
                    Err(err) => {
                        println!("Hanshake error: {err:?}");
                        tls_connection.write_tls(self.stream).unwrap();
                        return Ok(None);
                    }
                    Ok(state) => {
                        println!("Handshaking state: {state:?}");
                    }
                }
                tls_connection.write_tls(self.stream)?;
            }

            tls_connection.read_tls(self.stream)?;
            match tls_connection.process_new_packets() {
                Err(err) => {
                    println!("Plaintext read error: {err:?}");
                    tls_connection.write_tls(self.stream).unwrap();
                    return Ok(None);
                }
                Ok(state) => {
                    let mut buf = vec![];
                    buf.resize(state.plaintext_bytes_to_read(), 0u8);
                    match tls_connection.reader().read(&mut buf) {
                        Ok(n) => println!("ok bytes {n}"),
                        Err(err) => println!("{err:?}"),
                    }
                    request_bytes.append(&mut buf);
                }
            }
        } else {
            loop {
                let mut stream_buf: [u8; 255] = [0; 255];
                let read_result = self.stream.read(stream_buf.as_mut_slice());
                match read_result {
                    Ok(n) => {
                        request_bytes.extend_from_slice(stream_buf.take(n as u64).into_inner());
                        // raw_request = raw_request.add(std::str::from_utf8(&stream_buf).unwrap());
                        if n < stream_buf.len() {
                            break;
                        }
                        stream_buf.fill(0);
                    }
                    // todo: change this panic
                    Err(e) => panic!("{}", e),
                }
            }
        }

        Ok(Some(request_bytes))
    }

    pub fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(conn) = &mut self.tls_connection {
            conn.writer().write_all(bytes)?;
            conn.write_tls(self.stream)?;
            conn.send_close_notify();
        } else {
            self.stream.write_all(bytes)?;
        }

        Ok(())
    }
}