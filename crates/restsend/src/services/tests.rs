use anyhow::Result;
use futures_util::Future;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

pub(crate) const TEST_ENDPOINT: &str = "https://chat.ruzhila.cn";

#[allow(unused)]
pub(crate) fn open_port() -> String {
    for port in 30000..30100 {
        let addr = format!("127.0.0.1:{}", port);
        if let Ok(_) = std::net::TcpListener::bind(addr.clone()) {
            return addr;
        }
    }
    panic!("no port available");
}

#[allow(unused)]
async fn response_examples(req: Request<IncomingBody>) -> Result<Response<Full<Bytes>>> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

#[allow(unused)]
pub(crate) async fn serve<F, Fut>(addr: &str, func: F) -> Result<()>
where
    F: Fn(Request<IncomingBody>) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = Result<Response<Full<Bytes>>>> + Send + 'static,
{
    // Bind to the port and listen for incoming TCP connections
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        let (tcp, _) = listener.accept().await?;
        let io = TokioIo::new(tcp);

        let func = func.clone();
        tokio::task::spawn(async move {
            let service = service_fn(move |req| func(req));

            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
