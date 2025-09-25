mod test_media;
use crate::utils::spawn_task;
use crate::Result;
use futures_util::Future;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

pub(crate) const TEST_ENDPOINT: &str = "https://chat.ruzhila.cn";

#[allow(unused)]
pub(crate) fn open_port() -> String {
    // random port
    use rand::Rng;
    let mut rng = rand::thread_rng();

    for port in 1..10 {
        let port = rng.gen_range(30000..30100);
        let addr = format!("127.0.0.1:{}", port);
        if let Ok(_) = std::net::TcpListener::bind(addr.clone()) {
            return addr;
        }
    }
    panic!("no port available");
}

#[allow(unused)]
pub(crate) async fn serve_test_server<F, Fut>(addr: &str, func: F) -> Result<()>
where
    F: Fn(Request<IncomingBody>) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = Result<Response<Full<Bytes>>>> + Send + 'static,
{
    let addr = addr.to_string();
    let (is_started_tx, is_started) = oneshot::channel();
    spawn_task(async move {
        let listener = TcpListener::bind(addr.clone()).await.unwrap();
        println!("Listening on http://{}", addr);
        is_started_tx.send(()).unwrap();

        loop {
            let (tcp, _) = listener.accept().await.unwrap();
            let io = TokioIo::new(tcp);

            let func = func.clone();
            let service = service_fn(move |req| {
                let fut = func(req);
                async move { fut.await }
            });
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                println!("Error serving connection: {:?}", err);
                return;
            }
        }
    });
    is_started.await.unwrap();
    Ok(())
}
