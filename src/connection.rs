use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

pub struct Connection<'a> {
    stream: &'a mut TcpStream,
    tls_connection: Option<rustls::ServerConnection>,
    persistent: bool,
}

impl<'a> Connection<'a> {
    pub fn new(
        stream: &'a mut TcpStream,
        https_config: Option<Arc<rustls::ServerConfig>>,
        persistent: bool,
    ) -> Self {
        let tls_connection =
            https_config.map(|https_config| rustls::ServerConnection::new(https_config).unwrap());

        Connection {
            stream,
            tls_connection,
            persistent,
        }
    }

    // todo: this should read until at least CRLFCRLF,
    // then the result should be parsed to check if request might have a body
    pub fn read(&mut self) -> std::io::Result<Option<Vec<u8>>> {
        let mut request_bytes: Vec<u8> = Vec::new();

        if let Some(tls_connection) = &mut self.tls_connection {
            while tls_connection.is_handshaking() {
                tls_connection.read_tls(self.stream)?;
                match tls_connection.process_new_packets() {
                    Err(err) => {
                        println!("Handshake error: {err:?}");
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
                    tls_connection.reader().read_exact(&mut buf)?;
                    request_bytes.append(&mut buf);
                }
            }
        } else {
            loop {
                let mut stream_buf: [u8; 255] = [0; 255];
                let read_result = self.stream.read(stream_buf.as_mut_slice());
                match read_result {
                    Ok(n) => {
                        request_bytes.extend_from_slice(
                            stream_buf
                                .into_iter()
                                .take(n)
                                .collect::<Vec<u8>>()
                                .as_slice(),
                        );
                        // raw_request = raw_request.add(std::str::from_utf8(&stream_buf).unwrap());
                        if n < stream_buf.len() {
                            break;
                        }
                        stream_buf.fill(0);
                    }
                    Err(err) => {
                        return match err.kind() {
                            ErrorKind::ConnectionReset
                            | ErrorKind::ConnectionAborted
                            // todo: timeout error should not be swallowed, return 408 instead
                            | ErrorKind::TimedOut => Ok(None),
                            _ => Err(err),
                        }
                    }
                }
            }
        }

        Ok(Some(request_bytes))
    }

    pub fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(conn) = &mut self.tls_connection {
            conn.writer().write_all(bytes)?;
            if !self.persistent {
                conn.send_close_notify();
            }
            conn.write_tls(self.stream)?;
        } else {
            self.stream.write_all(bytes)?;
        }

        Ok(())
    }
}
