use std::error;
use std::fmt;
use std::io::{self, Read, Write};

#[derive(Debug)]
pub enum Error {
    Transport(io::Error),
    Upstream(String),
    RequestTooLong,
    Proto,
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Self::Transport(ref err) => Some(err),
            Self::Upstream(_) | Self::RequestTooLong | Self::Proto => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upstream(message) => write!(f, "Error from upstream: {}", message),
            Self::RequestTooLong => write!(f, "Request is too long"),
            Self::Transport(_) => write!(f, "Failed to write to stream"),
            Self::Proto => write!(f, "Protocol error"),
        }
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct RconClient<T>
where
    T: Read + Write,
{
    transport: T,
    cur_req_id: i32,
}

#[derive(Debug)]
struct RconPacket {
    req_id: i32,
    packet_type: i32,
    payload: Vec<u8>,
}

impl<T> RconClient<T>
where
    T: Read + Write,
{
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            cur_req_id: 0,
        }
    }

    fn recv_packet(&mut self) -> Result<RconPacket> {
        let mut buf = [0; 12];
        match self.transport.read_exact(&mut buf) {
            Ok(_) => (),
            Err(err) => return Err(Error::Transport(err)),
        }

        let len = i32::from_le_bytes(buf[0..4].try_into().unwrap());
        if len < 10 {
            return Err(Error::Proto);
        }

        let req_id = i32::from_le_bytes(buf[4..8].try_into().unwrap());
        let packet_type = i32::from_le_bytes(buf[8..12].try_into().unwrap());

        let mut payload = vec![0; len as usize - 4 - 4];
        match self.transport.read_exact(payload.as_mut_slice()) {
            Ok(_) => (),
            Err(err) => return Err(Error::Transport(err)),
        }
        let payload = buf[0..buf.len() - 2].to_vec();

        Ok(RconPacket {
            req_id,
            packet_type,
            payload,
        })
    }

    fn send_packet(&mut self, packet_type: i32, payload: &str) -> Result<()> {
        let len = 4 + 4 + 2 + payload.len();
        if len > 1456 {
            return Err(Error::RequestTooLong);
        }

        let mut buf = Vec::<u8>::new();

        buf.write_all(&(len as i32).to_le_bytes()).unwrap();
        buf.write_all(&self.cur_req_id.to_le_bytes()).unwrap();
        buf.write_all(&packet_type.to_le_bytes()).unwrap();
        buf.write_all(payload.as_bytes()).unwrap();
        buf.write_all(&[0; 2]).unwrap();

        if let Err(err) = self.transport.write_all(buf.as_slice()) {
            return Err(Error::Transport(err));
        };
        self.cur_req_id += 1;

        Ok(())
    }

    pub fn authenticate(&mut self, passwd: &str) -> Result<()> {
        match self.send_packet(3, passwd) {
            Ok(_) => (),
            Err(err) => return Err(err),
        }

        let packet = match self.recv_packet() {
            Ok(packet) => packet,
            Err(err) => return Err(err),
        };

        if packet.req_id == -1 || packet.packet_type != 2 {
            Err(Error::Upstream(
                String::from_utf8_lossy(&packet.payload).into_owned(),
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream;

    #[test]
    fn test() {
        // Remove this test in the future because it depends on a real Minecraft server.

        let mut client = RconClient::new(TcpStream::connect(("localhost", 25575)).unwrap());
        client.authenticate("x").unwrap();
    }
}
