use log::{debug, error};
use rustls::IoState;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;

type IoResult<S> = std::io::Result<S>;

#[derive(Debug)]
pub enum ReadStrategy {
    UntilDoubleCrlf,
    UntilNoBytesRead(usize),
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

pub struct Connection<'stream> {
    stream: &'stream mut dyn ReadWrite,
    tls_connection: Option<rustls::ServerConnection>,
    persistent: bool,
}

impl<'stream> Connection<'stream> {
    pub fn new(
        stream: &'stream mut TcpStream,
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

    pub fn read(&mut self, read_strategy: ReadStrategy) -> std::io::Result<Vec<u8>> {
        let mut read_state_machine = ReadStateMachine::new(self, read_strategy);

        loop {
            read_state_machine = read_state_machine.next();

            match read_state_machine.state {
                ReadState::Done => return Ok(read_state_machine.read_bytes),
                ReadState::Error(kind) => return Err(kind.into()),
                _ => {}
            }
        }
    }

    pub fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(conn) = self.tls_connection.as_mut() {
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

#[derive(Copy, Clone)]
enum ReadState {
    Before,
    Read,
    TlsHandshake,
    TlsRead,
    After(usize),
    Done,
    Error(ErrorKind),
}

struct ReadStateMachine<'connection, 'stream> {
    connection: &'connection mut Connection<'stream>,
    read_strategy: ReadStrategy,
    read_bytes: Vec<u8>,
    state: ReadState,
}

impl<'connection, 'stream> ReadStateMachine<'connection, 'stream> {
    fn new(connection: &'connection mut Connection<'stream>, read_strategy: ReadStrategy) -> Self {
        ReadStateMachine {
            connection,
            read_strategy,
            read_bytes: vec![],
            state: ReadState::Before,
        }
    }

    fn next(mut self) -> Self {
        let next_state = match self.state {
            ReadState::Before if self.connection.tls_connection.is_none() => ReadState::Read,
            ReadState::Before => ReadState::TlsHandshake,
            ReadState::Read => Self::map_error(self.read()),
            ReadState::TlsHandshake => Self::map_error(self.tls_handshake()),
            ReadState::TlsRead => Self::map_error(self.tls_read()),
            ReadState::After(read_bytes) => self.check_if_finished(read_bytes),
            ReadState::Done | ReadState::Error(_) => self.state,
        };

        self.set_state(next_state)
    }

    fn set_state(mut self, new_state: ReadState) -> Self {
        self.state = new_state;

        self
    }

    fn map_error(result: IoResult<ReadState>) -> ReadState {
        match result {
            Ok(state) => state,
            Err(err) => ReadState::Error(err.kind()),
        }
    }

    fn read(&mut self) -> IoResult<ReadState> {
        let stream = &mut self.connection.stream;

        let mut read_bytes: usize = 0;

        loop {
            let mut stream_buf = [0u8; 1024];
            let read_length = stream.as_read_mut().read(stream_buf.as_mut_slice())?;
            self.read_bytes.append(
                &mut stream_buf
                    .into_iter()
                    .take(read_length)
                    .collect::<Vec<u8>>(),
            );

            read_bytes += read_length;

            if read_length < stream_buf.len() {
                break;
            }

            stream_buf.fill(0);
        }

        Ok(ReadState::After(read_bytes))
    }

    fn tls_handshake(&mut self) -> IoResult<ReadState> {
        let tls_connection = self.connection.tls_connection.as_mut().unwrap();
        let stream = &mut self.connection.stream;

        while tls_connection.is_handshaking() {
            tls_connection.read_tls(stream.as_read_mut())?;
            match &mut tls_connection.process_new_packets() {
                Err(err) => {
                    error!("Handshake error: {err:?}");
                    tls_connection.write_tls(stream.as_write_mut())?;
                    return Err(ErrorKind::Other.into());
                }
                Ok(state) => {
                    debug!(
                        "Handshaking state: {state:?}, {}",
                        tls_connection.is_handshaking()
                    );
                    let bytes_to_read = state.plaintext_bytes_to_read();
                    if bytes_to_read > 0 {
                        self.read_bytes
                            .append(&mut read_tls_plaintext_bytes(tls_connection, state)?);
                        return Ok(ReadState::After(bytes_to_read));
                    }
                }
            }
            tls_connection.write_tls(stream.as_write_mut())?;
        }

        Ok(ReadState::TlsRead)
    }

    fn tls_read(&mut self) -> IoResult<ReadState> {
        let tls_connection = self.connection.tls_connection.as_mut().unwrap();
        let stream = &mut self.connection.stream;

        tls_connection.read_tls(stream.as_read_mut())?;
        match &mut tls_connection.process_new_packets() {
            Err(err) => {
                error!("Plaintext read error: {err:?}");
                tls_connection.write_tls(stream.as_write_mut())?;
                Err(ErrorKind::Other.into())
            }
            Ok(state) => {
                let read_bytes = state.plaintext_bytes_to_read();
                self.read_bytes
                    .append(&mut read_tls_plaintext_bytes(tls_connection, state)?);
                Ok(ReadState::After(read_bytes))
            }
        }
    }

    fn check_if_finished(&mut self, read_bytes: usize) -> ReadState {
        if read_bytes == 0 {
            return ReadState::Done;
        }

        match self.read_strategy {
            ReadStrategy::UntilDoubleCrlf => {
                for bytes in self.read_bytes.windows(4) {
                    if let [.., b'\r', b'\n', b'\r', b'\n'] = bytes {
                        return ReadState::Done;
                    }
                }
            }
            ReadStrategy::UntilNoBytesRead(length) => {
                if self.read_bytes.len() >= length {
                    return ReadState::Done;
                }
            }
        }

        ReadState::Read
    }
}

#[cfg(test)]
mod test {
    use crate::connection::{Connection, ReadStrategy};
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

        let read_bytes = connection.read(ReadStrategy::UntilDoubleCrlf).unwrap();
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

        let read_bytes = connection.read(ReadStrategy::UntilDoubleCrlf).unwrap();
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

        let read_bytes = connection
            .read(ReadStrategy::UntilNoBytesRead(501))
            .unwrap();
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

        let read_bytes = connection.read(ReadStrategy::UntilDoubleCrlf).unwrap();
        assert_eq!(read_bytes.len(), 0);
    }
}
