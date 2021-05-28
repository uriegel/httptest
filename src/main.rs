use std::{collections::HashMap, convert::Infallible, sync::{Arc, Mutex}, thread};
use chrono::Utc;
use tokio::{sync::mpsc};
use warp::{Filter, Rejection, Reply, fs::{
        File, dir
    }, http::HeaderValue, hyper::{Body, HeaderMap, Response}, ws::{
        Message, WebSocket
    }};
use futures::{FutureExt, StreamExt};

fn add_headers(reply: File)->Response<Body> {
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

#[derive(Debug, Clone)]
struct Session {
    sender: mpsc::UnboundedSender<std::result::Result<Message, warp::Error>>
}

type Result<T> = std::result::Result<T, Rejection>;

type Sessions = Arc<Mutex<HashMap<String, Session>>>;

async fn client_connection(ws: WebSocket, id: String, sessions: Sessions) {
    let (client_ws_sender, _) = ws.split();
    let (client_sender, client_rcv) = mpsc::unbounded_channel();
    let client_rcv = tokio_stream::wrappers::UnboundedReceiverStream::new(client_rcv);
    tokio::task::spawn(client_rcv.forward(client_ws_sender).map(|result| {
        if let Err(e) = result {
            eprintln!("error sending websocket msg: {}", e);
        }
    }));

    sessions.lock().unwrap().insert(
        id.clone(),
        Session {
            sender: client_sender,
        },
    );

    println!("{} connected", id);
}

async fn on_connect(ws: warp::ws::Ws, id: String, sessions: Sessions) -> Result<impl Reply> {
    println!("on_connect: {}", id);

    Ok(ws.on_upgrade(move |socket| client_connection(socket, id, sessions)))
    //    None => Err(warp::reject::not_found()),
    //}
}

#[tokio::main]
async fn main() {
    let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));
    //let (tx, rx) = mpsc::unbounded_channel::<String>();
    
    fn with_sessions(sessions: Sessions) -> impl Filter<Extract = (Sessions,), Error = Infallible> + Clone {
        warp::any().map(move || sessions.clone())
    }

    let route_static = dir(".")
        .map(add_headers);

    async fn bm_test(sessions: Sessions)->std::result::Result<impl warp::Reply, warp::Rejection> {
        thread::spawn( move|| {
            std::thread::sleep(core::time::Duration::from_millis(5000));
            println!("Sending to left ws");
            if let Some(session) = sessions.lock().unwrap().get("left").cloned() {
                //tokio::time::sleep(core::time::Duration::from_millis(5000)).await;
                let _ = session.sender.send(Ok(Message::text("Guten Abend")));
            }
        });

        Ok("passed".to_string())
    }
    let route_test = 
        warp::path("test")
        .and(warp::path::end())
        .and(with_sessions(sessions.clone()))
        .and_then(bm_test);

    let route_ws = warp::path("ws")
        .and(warp::ws())
        .and(warp::path::param())
        .and(with_sessions(sessions.clone()))
        .and_then(on_connect);

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
