use anyhow::Result;
use fastwebsockets::{FragmentCollector, OpCode};
use http_body_util::Empty;
use hyper::body::Bytes;
use hyper::Request;
use std::future::Future;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::ClientConfig;
use tokio_rustls::TlsConnector;

const DOMAIN: &str = "stream.binance.com";
const WS_URI: &str = "/ws/suiusdt@trade";

struct SpawnExecutor;

impl<Fut> hyper::rt::Executor<Fut> for SpawnExecutor
where
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    fn execute(&self, fut: Fut) {
        tokio::spawn(fut);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut addr = String::from(DOMAIN);
    addr.push_str(":9443");
    let tcp_stream = TcpStream::connect(&addr).await?;
    let root_store = tokio_rustls::rustls::RootCertStore::from_iter(
        webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
    );
    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let tls_stream = TlsConnector::from(Arc::new(config));
    let domain = ServerName::try_from(DOMAIN)?;
    let tls_stream = tls_stream.connect(domain, tcp_stream).await?;

    let req = Request::builder()
        .method("GET")
        .uri(format!("wss://{}{}", &addr, WS_URI))
        .header("Host", &addr)
        .header("Upgrade", "websocket")
        .header("Connection", "Upgrade")
        .header(
            "Sec-WebSocket-Key",
            fastwebsockets::handshake::generate_key(),
        )
        .header("Sec-WebSocket-Version", "13")
        .body(Empty::<Bytes>::new())?;

    let (ws, _) = fastwebsockets::handshake::client(&SpawnExecutor, req, tls_stream).await?;

    let mut ws = FragmentCollector::new(ws);
    loop {
        let msg = match ws.read_frame().await {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Error reading frame: {:?}", e);
                break;
            }
        };

        match msg.opcode {
            OpCode::Text => {
                let text = String::from_utf8_lossy(&msg.payload);
                println!("Text: {}", text);
            }
            OpCode::Binary => {
                println!("Binary: {:?}", msg.payload);
            }
            OpCode::Close => {
                println!("Close: {:?}", msg.payload);
                break;
            }
            OpCode::Ping => {
                println!("Ping: {:?}", msg.payload);
            }
            OpCode::Pong => {
                println!("Pong: {:?}", msg.payload);
            }
            OpCode::Continuation => {
                println!("Continuation: {:?}", msg.payload);
            }
        }
    }
    Ok(())
}
