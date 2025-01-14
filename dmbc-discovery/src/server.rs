use std::error::Error;
use std::net::SocketAddr;

use futures::{future, Future, Stream};
use hyper;
use hyper::{Body, Method, Request, Response, StatusCode};
use serde_json as json;

use nodes;
use nodes::{NodeInfo, NodeKeys};
use log;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct ValidatorInfo(NodeKeys, NodeInfo);

pub type ResponseFuture = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send + 'static>;

pub fn serve(req: Request<Body>, addr: SocketAddr) -> ResponseFuture {
    info!(log::SERVER,
          "Processing request";
          "remote" => %addr,
          "method" => %req.method(),
          "uri" => %req.uri());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/nodes") => get_nodes(),
        (&Method::POST, "/nodes") => post_node(req.into_body(), addr),
        _ => Box::new(future::ok(
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap(),
        )),
    }
}

fn get_nodes() -> ResponseFuture {
    let nodes = json::to_string_pretty(&nodes::list()).expect("Unable to deserialize nodes list.");
    Box::new(future::ok(Response::new(nodes.into())))
}

fn update_peer(vi: ValidatorInfo) -> ResponseFuture {
    nodes::update(vi.0, vi.1);
    Box::new(future::ok(Response::new(Body::empty())))
}

fn post_node(body: Body, addr: SocketAddr) -> ResponseFuture {
    let post = body
        .concat2()
        .and_then(move |v| match json::from_slice::<ValidatorInfo>(&v) {
            Ok(mut info) => {
                let fix_addr = |reported: &mut String| {
                    let offset = reported.find(':').unwrap_or(reported.len());
                    reported.replace_range(..offset, &format!("{}", addr.ip()));
                };
                fix_addr(&mut info.1.public);
                fix_addr(&mut info.1.private);
                fix_addr(&mut info.1.peer);
                update_peer(info)
            }
            Err(e) => Box::new(future::ok(
                Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(e.description().to_string().into())
                    .unwrap(),
            )),
        });
    Box::new(post)
}

