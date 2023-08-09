use anyhow::Result;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "0.0.0.0:8888";
    let sock = UdpSocket::bind(addr).await?;
    let mut buf = [0; 1024];
    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        println!("{:?} bytes received from {:?}", len, addr);

        let len = sock.send_to(&buf[..len], addr).await?;
        println!("{:?} bytes sent to {:?}", len, addr);
    }
}
