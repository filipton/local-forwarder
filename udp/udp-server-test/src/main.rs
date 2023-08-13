use anyhow::Result;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::UdpSocket};
use udpflow::{UdpListener, UdpStreamLocal};

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "0.0.0.0:8888";
    let sock = UdpSocket::bind(addr).await?;
    let listener = UdpListener::new(sock);
    let buf = &mut [0u8; 0x10000];

    while let Ok((stream, _)) = listener.accept(&mut buf[..]).await {
        tokio::spawn(handle(stream));
    }

    Ok(())
}

async fn handle(mut stream: UdpStreamLocal) {
    println!("new stream: {}", stream.peer_addr());

    let buf = &mut [0u8; 0x10000];
    loop {
        let n = stream.read(&mut buf[..]).await.unwrap();
        if n == 0 {
            break;
        }

        println!("recv {} bytes", n);
        println!("Message: {}", String::from_utf8_lossy(&buf[..n]));

        stream.write_all(&buf[..n]).await.unwrap();
    }
}
