use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use udp_stream::{UdpListener, UdpStream};

const UDP_BUFFER_SIZE: usize = 17480; // 17kb
const UDP_TIMEOUT: u64 = 10 * 1000; // 10sec

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:8080";
    let remote_addr = "127.0.0.1:8888";
    let timeout = Duration::from_millis(UDP_TIMEOUT);
    let listener = UdpListener::bind(addr.parse()?).await?;
    loop {
        let (mut stream, _) = listener.accept().await?;
        println!("Accepted connection from: {}", stream.peer_addr()?,);

        tokio::spawn(async move {
            let mut remote = UdpStream::connect(remote_addr.parse().unwrap())
                .await
                .unwrap();

            let mut local_buf = vec![0u8; UDP_BUFFER_SIZE];
            let mut remote_buf = vec![0u8; UDP_BUFFER_SIZE];

            loop {
                tokio::select! {
                    res = tokio::time::timeout(timeout, stream.read(&mut local_buf))=> {
                        if res.is_err() {
                            remote.shutdown();
                            stream.shutdown();

                            println!("Connection closed");
                            break;
                        }
                        let n = res.unwrap().unwrap();

                        remote.write(&local_buf[..n]).await.unwrap();
                    }
                    res = tokio::time::timeout(timeout, remote.read(&mut remote_buf)) => {
                        if res.is_err() {
                            remote.shutdown();
                            stream.shutdown();

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
