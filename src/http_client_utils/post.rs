use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use hyper_util::rt::TokioIo;
use hyper::{body::Buf, Request};
use hyper::Response;
use hyper::body::Incoming;
use tokio::net::TcpStream;

pub async fn http_post_raw(uri: &str, body: &str) -> Result<Response<Incoming>, Box<dyn std::error::Error + Send + Sync>> {
  let uri = uri.parse::<hyper::Uri>()?;
  let host = uri.host().expect("uri has no host");
  let port = uri.port_u16().unwrap_or(80);
  let addr = format!("{}:{}", host, port);
  let stream = TcpStream::connect(addr).await?;
  let io = TokioIo::new(stream);

  let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
  tokio::task::spawn(async move {
      if let Err(err) = conn.await {
          println!("Connection failed: {:?}", err);
      }
  });

  let authority = uri.authority().unwrap().clone();

  let path = uri.path();

  let req = Request::builder()
  .method("POST")
  .uri(path)
  .header(hyper::header::HOST, authority.as_str())
  .header(hyper::header::CONTENT_TYPE, "application/json")
  .body(Full::new(Bytes::from(body.to_string())))?;

  let res = sender.send_request(req).await?;

  Ok(res)  
}

pub async fn http_post_bytes(uri: &str, body: &str) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
  let res = http_post_raw(uri, body).await?;
  let body = res.collect().await?.aggregate();
  Ok(Bytes::copy_from_slice(body.chunk()))
}

pub async fn http_post_string(uri: &str, body: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
  let bytes = http_post_bytes(uri, body).await?;
  Ok(String::from_utf8(bytes.to_vec())?)
}

pub async fn http_post_json(uri: &str, body: &str) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
  let res = http_post_raw(uri, body).await?;
  let body = res.collect().await?.aggregate();
  let json = serde_json::from_reader(body.reader())?;
  Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_post_raw() {
        let res = http_post_raw("http://httpbin.org/post", r#"{"key": "value"}"#).await.unwrap();
        assert_eq!(res.status(), 200);
    }

    #[tokio::test]
    async fn test_http_post_bytes() {
        let bytes = http_post_bytes("http://httpbin.org/post", r#"{"key": "value"}"#).await.unwrap();
        let response = String::from_utf8_lossy(&bytes);
        assert!(response.contains(r#""url": "http://httpbin.org/post""#));
        assert!(response.contains(r#""key": "value""#));
    }

    #[tokio::test]
    async fn test_http_post_string() {
        let string = http_post_string("http://httpbin.org/post", r#"{"key": "value"}"#).await.unwrap();
        assert!(string.contains(r#""url": "http://httpbin.org/post""#));
        assert!(string.contains(r#""key": "value""#));
    }

    #[tokio::test]
    async fn test_http_post_json() {
        let json = http_post_json("http://httpbin.org/post", r#"{"key": "value"}"#).await.unwrap();
        assert_eq!(json["url"], "http://httpbin.org/post");
    }
}
