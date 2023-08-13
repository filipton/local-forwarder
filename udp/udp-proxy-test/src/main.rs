use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use udpflow::{UdpListener, UdpSocket, UdpStreamRemote};

const UDP_BUFFER_SIZE: usize = 0x10000;
const UDP_TIMEOUT: u64 = 10 * 1000;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:8080";
    let remote_addr = "127.0.0.1:8888";
    let timeout = Duration::from_millis(UDP_TIMEOUT);
    //let listener = UdpListener::bind(addr.parse()?).await?;

    let socket = UdpSocket::bind(addr).await?;
    let listener = UdpListener::new(socket);

    let buf = &mut [0; UDP_BUFFER_SIZE];
    loop {
        let (mut stream, _) = listener.accept(&mut buf[..]).await?;
        println!("Accepted connection from: {}", stream.peer_addr());

        tokio::spawn(async move {
            let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
            let mut stream2 = UdpStreamRemote::new(socket, remote_addr.parse().unwrap());

            //tokio::io::copy(&mut stream, &mut stream2).await.unwrap();

            let mut local_buf = vec![0u8; UDP_BUFFER_SIZE];
            let mut remote_buf = vec![0u8; UDP_BUFFER_SIZE];

            loop {
                tokio::select! {
                    res = tokio::time::timeout(timeout, stream.read(&mut local_buf))=> {
                        if res.is_err() {
                            _ = stream2.shutdown().await;
                            _ = stream.shutdown().await;

                            println!("Connection closed");
                            break;
                        }
                        let n = res.unwrap().unwrap();

                        stream2.write(&local_buf[..n]).await.unwrap();
                    }
                    res = tokio::time::timeout(timeout, stream2.read(&mut remote_buf)) => {
                        if res.is_err() {
                            _ = stream2.shutdown().await;
                            _ = stream.shutdown().await;

                            println!("Connection closed");
                            break;
                        }
                        let n = res.unwrap().unwrap();

                        stream.write(&remote_buf[..n]).await.unwrap();
                    }
                }
            }
        });
    }
}
