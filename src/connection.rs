use log::{debug, error};
use rustls::IoState;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;

#[derive(Debug)]
pub enum ReadUntil {
    DoubleCrLf,
    NoBytes(usize),
}

pub trait ReadWrite: Read + Write {
    fn as_read_mut(&mut self) -> &mut dyn Read;

    fn as_write_mut(&mut self) -> &mut dyn Write;
}

impl ReadWrite for TcpStream {
    fn as_read_mut(&mut self) -> &mut dyn Read {
        self
    }

    fn as_write_mut(&mut self) -> &mut dyn Write {
        self
    }
}

pub struct Connection<'a> {
    stream: &'a mut dyn ReadWrite,
    tls_connection: Option<rustls::ServerConnection>,
    persistent: bool,
}

impl<'a> Connection<'a> {
    pub fn new(
        stream: &'a mut TcpStream,
        https_config: Option<Arc<rustls::ServerConfig>>,
        persistent: bool,
    ) -> Self {
        let port = match stream.local_addr().unwrap() {
            SocketAddr::V4(addr) => addr.port(),
            SocketAddr::V6(_) => unimplemented!(),
        };
        let tls_connection = match port {
            443 => https_config
                .map(|https_config| rustls::ServerConnection::new(https_config).unwrap()),
            _ => None,
        };

        Connection {
            stream,
            tls_connection,
            persistent,
        }
    }

    pub fn read(&mut self, read_until: ReadUntil) -> std::io::Result<Option<Vec<u8>>> {
        let mut request_bytes: Vec<u8> = Vec::new();

        loop {
            let prev_iter_len = request_bytes.len();

            if let Some(tls_connection) = &mut self.tls_connection {
                let mut read_plaintext_bytes = false;
                while tls_connection.is_handshaking() {
                    tls_connection.read_tls(self.stream.as_read_mut())?;
                    match &mut tls_connection.process_new_packets() {
                        Err(err) => {
                            error!("Handshake error: {err:?}");
                            tls_connection
                                .write_tls(self.stream.as_write_mut())
                                .unwrap();
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
                    tls_connection.write_tls(self.stream.as_write_mut())?;
                }

                if !read_plaintext_bytes {
                    tls_connection.read_tls(self.stream.as_read_mut())?;
                    match &mut tls_connection.process_new_packets() {
                        Err(err) => {
                            error!("Plaintext read error: {err:?}");
                            tls_connection
                                .write_tls(self.stream.as_write_mut())
                                .unwrap();
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
                    let read_length = self.stream.as_read_mut().read(stream_buf.as_mut_slice())?;
                    request_bytes.append(
                        &mut stream_buf
                            .into_iter()
                            .take(read_length)
                            .collect::<Vec<u8>>(),
                    );

                    if read_length < stream_buf.len() {
                        break;
                    }

                    stream_buf.fill(0);
                }
            }

            // Fixes infinite loop when peer closes connection before whole HTTP message
            // has been received. In such case read() returns nothing and this is the easier
            // way to check if that's the case.
            if request_bytes.len() == prev_iter_len {
                break;
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
                conn.write_tls(self.stream.as_write_mut())?;
            }
        } else {
            self.stream.as_write_mut().write_all(bytes)?;
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

#[cfg(test)]
mod test {
    use crate::connection::{Connection, ReadUntil};
    use crate::test::mocks::MockReadWrite;
    use rand::RngCore;

    fn get_rand_vec(len: usize) -> Vec<u8> {
        let mut read_buf: Vec<u8> = Vec::new();
        read_buf.resize(len, 0);
        rand::thread_rng().fill_bytes(&mut read_buf);

        read_buf
    }

    fn prepare_mock(read_buf_len: usize) -> MockReadWrite {
        let mut read_buf: Vec<u8> = get_rand_vec(read_buf_len);
        read_buf[read_buf_len - 4] = b'\r';
        read_buf[read_buf_len - 3] = b'\n';
        read_buf[read_buf_len - 2] = b'\r';
        read_buf[read_buf_len - 1] = b'\n';
        MockReadWrite {
            read_buf,
            write_buf: vec![],
        }
    }

    #[test]
    fn reads_all_bytes_until_double_crlf_at_end() {
        let mut mock = prepare_mock(734);
        let mut connection = Connection {
            stream: &mut mock,
            tls_connection: None,
            persistent: false,
        };

        let read_bytes = connection.read(ReadUntil::DoubleCrLf).unwrap().unwrap();
        assert_eq!(read_bytes.len(), 734);
    }

    #[test]
    fn reads_all_bytes_until_double_crlf_mid_way() {
        let mut mock = {
            let mut read_buf: Vec<u8> = get_rand_vec(395);
            read_buf[237] = b'\r';
            read_buf[238] = b'\n';
            read_buf[239] = b'\r';
            read_buf[240] = b'\n';
            MockReadWrite {
                read_buf,
                write_buf: vec![],
            }
        };
        let mut connection = Connection {
            stream: &mut mock,
            tls_connection: None,
            persistent: false,
        };

        let read_bytes = connection.read(ReadUntil::DoubleCrLf).unwrap().unwrap();
        assert_eq!(read_bytes.len(), 395);
    }

    #[test]
    fn reads_all_bytes_until_no_bytes() {
        let mut mock = MockReadWrite {
            read_buf: get_rand_vec(501),
            write_buf: vec![],
        };
        let mut connection = Connection {
            stream: &mut mock,
            tls_connection: None,
            persistent: false,
        };

        let read_bytes = connection.read(ReadUntil::NoBytes(501)).unwrap().unwrap();
        assert_eq!(read_bytes.len(), 501);
    }

    #[test]
    fn returns_empty_vec_if_read_nothing() {
        let mut mock = MockReadWrite {
            read_buf: vec![],
            write_buf: vec![],
        };
        let mut connection = Connection {
            stream: &mut mock,
            tls_connection: None,
            persistent: false,
        };

        let read_bytes = connection.read(ReadUntil::DoubleCrLf).unwrap().unwrap();
        assert_eq!(read_bytes.len(), 0);
    }
}
