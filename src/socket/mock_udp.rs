use super::*;

pub struct NetworkConfig {}

pub fn create_mock_socket(_: NetworkConfig) -> (MockUdpSocket, MockUdpSocket) {
    (MockUdpSocket {}, MockUdpSocket {})
}

pub struct MockUdpSocket {}

impl DatagramSocket for MockUdpSocket {
    fn set_read_timeout(&self, _buf: Option<Duration>) -> io::Result<()> {
        todo!()
    }

    fn read_timeout(&self) -> io::Result<Option<Duration>> {
        todo!()
    }

    fn send(&self, _buf: &[u8]) -> io::Result<()> {
        todo!()
    }

    fn recv(&self, _buf: &mut [u8]) -> io::Result<usize> {
        todo!()
    }
}
