// I use this file to check if server correctly responds in certain scenarios (usually it doesn't).

use std::io::{Result, Write};
use std::net::TcpStream;

fn main() -> Result<()> {
    let mut tcp = TcpStream::connect("127.0.0.1:80")?;

    let message = "GET /index.html HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n";
    let message2 = "\r\n";

    // Should wait for the second write before processing request.
    tcp.write_all(message.as_bytes())?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    tcp.write_all(message2.as_bytes())?;

    std::thread::sleep(std::time::Duration::from_secs(2));

    // Should return 408 if whole request was not read before socket timeout.
    tcp.write_all(message.as_bytes())?;
    std::thread::sleep(std::time::Duration::from_secs(60));

    Ok(())
}
