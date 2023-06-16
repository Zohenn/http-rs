use log::{debug, error};
use rustls::IoState;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

#[derive(Debug)]
pub enum ReadUntil {
    DoubleCrLf,
    NoBytes(usize),
}

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
    pub fn read(&mut self, read_until: ReadUntil) -> std::io::Result<Option<Vec<u8>>> {
        let mut request_bytes: Vec<u8> = Vec::new();

        loop {
            if let Some(tls_connection) = &mut self.tls_connection {
                let mut read_plaintext_bytes = false;
                while tls_connection.is_handshaking() {
                    tls_connection.read_tls(self.stream)?;
                    match &mut tls_connection.process_new_packets() {
                        Err(err) => {
                            error!("Handshake error: {err:?}");
                            tls_connection.write_tls(self.stream).unwrap();
                            return Ok(None);
                        }
                        Ok(state) => {
                            debug!(
                                "Handshaking state: {state:?}, {}",
                                tls_connection.is_handshaking()
                            );
                            if state.plaintext_bytes_to_read() > 0 {
                                request_bytes
                                    .append(&mut read_tls_plaintext_bytes(tls_connection, state)?);
                                read_plaintext_bytes = true;
                            }
                        }
                    }
                    tls_connection.write_tls(self.stream)?;
                }

                if !read_plaintext_bytes {
                    tls_connection.read_tls(self.stream)?;
                    match &mut tls_connection.process_new_packets() {
                        Err(err) => {
                            error!("Plaintext read error: {err:?}");
                            tls_connection.write_tls(self.stream).unwrap();
                            return Ok(None);
                        }
                        Ok(state) => {
                            request_bytes
                                .append(&mut read_tls_plaintext_bytes(tls_connection, state)?);
                        }
                    }
                }
            } else {
                loop {
                    let mut stream_buf: [u8; 255] = [0; 255];
                    let read_length = self.stream.read(stream_buf.as_mut_slice())?;
                    request_bytes.extend_from_slice(
                        stream_buf
                            .into_iter()
                            .take(read_length)
                            .collect::<Vec<u8>>()
                            .as_slice(),
                    );

                    if read_length < stream_buf.len() {
                        break;
                    }

                    stream_buf.fill(0);
                }
            }

            match read_until {
                ReadUntil::DoubleCrLf => {
                    let mut crlf_found = false;
                    for bytes in request_bytes.windows(4) {
                        if let [.., b'\r', b'\n', b'\r', b'\n'] = bytes {
                            crlf_found = true;
                            break;
                        }
                    }

                    if crlf_found {
                        break;
                    }
                }
                ReadUntil::NoBytes(length) => {
                    if request_bytes.len() >= length {
                        break;
                    }
                }
            }
        }

        Ok(Some(request_bytes))
    }

    pub fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(conn) = &mut self.tls_connection {
            // todo: try not to set unlimited buffer size
            conn.set_buffer_limit(None);
            conn.writer().write_all(bytes)?;
            if !self.persistent {
                conn.send_close_notify();
            }
            while conn.wants_write() {
                conn.write_tls(self.stream)?;
            }
        } else {
            self.stream.write_all(bytes)?;
        }

        Ok(())
    }
}

fn read_tls_plaintext_bytes(
    tls_connection: &mut rustls::ServerConnection,
    state: &IoState,
) -> std::io::Result<Vec<u8>> {
    let mut buf = vec![];
    buf.resize(state.plaintext_bytes_to_read(), 0u8);
    tls_connection.reader().read_exact(&mut buf)?;

    Ok(buf)
}
