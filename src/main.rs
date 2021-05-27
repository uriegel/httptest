use chrono::Utc;
use warp::{Filter, Reply, fs::{File, dir}, http::HeaderValue, hyper::{Body, HeaderMap, Response}};
use futures::{FutureExt, StreamExt};

//type Sessions = Arc<Mutex<HashMap<String, Session>>>;

pub fn add_headers(reply: File)->Response<Body> {
    let mut res = reply.into_response();
    let headers = res.headers_mut();
    let header_map = create_headers();
    headers.extend(header_map);
    res
}

fn create_headers() -> HeaderMap {
    let mut header_map = HeaderMap::new();
    let now = Utc::now();
    let now_str = now.format("%a, %d %h %Y %T GMT").to_string();
    header_map.insert("Expires", HeaderValue::from_str(now_str.as_str()).unwrap());
    header_map.insert("Server", HeaderValue::from_str("webview-app").unwrap());
    header_map
}

#[tokio::main]
async fn main() {
    let route_static = dir(".")
    .map(add_headers);

    async fn bm_test()->Result<impl warp::Reply, warp::Rejection> {
        std::thread::sleep(core::time::Duration::from_millis(25000));
        //tokio::time::sleep(core::time::Duration::from_millis(5000)).await;
        Ok("passed".to_string())
    }
    let route_test = 
        warp::path("test")
        .and(warp::path::end())
        .and_then(bm_test);

    let route_ws = warp::path("echo")
        // The `ws()` filter will prepare the Websocket handshake.
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(|websocket| {
                // Just echo all messages back...
                let (tx, rx) = websocket.split();
                rx.forward(tx).map(|result| {
                    if let Err(e) = result {
                        eprintln!("websocket error: {:?}", e);
                    }
                })
            })
        });        

    let routes = route_test
        .or(route_ws)
        .or(route_static);

    let port = 8080;
    println!("Warp started: https://localhost:{}", 8080);

    warp::serve(routes)
        .tls()
        .cert_path("src/cert.pem")
        .key_path("src/key.rsa")    
        .run(([127,0,0,1], port))
        .await;     
}