use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper_util::rt::TokioIo;
use hyper::{body::Buf, Request};
use hyper::Response;
use hyper::body::Incoming;
use tokio::net::TcpStream;

pub async fn http_fetch_raw(uri: &str) -> Result<Response<Incoming>, Box<dyn std::error::Error + Send + Sync>> {
    let uri = uri.parse::<hyper::Uri>()?;
    let host = uri.host().expect("uri has no host");
    let port = uri.port_u16().unwrap_or(80);
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake::<_, Empty<Bytes>>(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    let authority = uri.authority().unwrap().clone();

    let path = uri.path();
    let req = Request::builder()
        .uri(path)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

    let res = sender.send_request(req).await?;

    Ok(res)
}

pub async fn http_fetch_bytes(uri: &str) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    let res = http_fetch_raw(uri).await?;
    let mut body = res.collect().await?.aggregate();
    Ok(body.copy_to_bytes(body.remaining()))
}

pub async fn http_fetch_string(uri: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let bytes = http_fetch_bytes(uri).await?;
    Ok(String::from_utf8(bytes.to_vec())?)
}

pub async fn http_fetch_json(uri: &str) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let res = http_fetch_raw(uri).await?;
    let body = res.collect().await?.aggregate();
    let json = serde_json::from_reader(body.reader())?;
    Ok(json)
}

#[cfg(test)]

mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_fetch_raw() {
        let res = http_fetch_raw("http://httpbin.org/get").await.unwrap();
        assert_eq!(res.status(), 200);
    }

    #[tokio::test]
    async fn test_http_fetch_bytes() {
        let bytes = http_fetch_bytes("http://httpbin.org/get").await.unwrap();
        assert!(bytes.len() > 0);
    }

    #[tokio::test]
    async fn test_http_fetch_string() {
        let string = http_fetch_string("http://httpbin.org/get").await.unwrap();
        assert!(string.len() > 0);
    }

    #[tokio::test]
    async fn test_http_fetch_json() {
        let json = http_fetch_json("http://httpbin.org/get").await.unwrap();
        assert_eq!(json["url"], "http://httpbin.org/get");
    }
}