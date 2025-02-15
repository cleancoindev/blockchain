// Copyright 2017 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;
use std::net::SocketAddr;

use iron::prelude::*;
use params::{Params, Value as ParamsValue};
use router::Router;

use api::{Api, ApiError};
use blockchain::{Blockchain, Service, SharedNodeState};
use crypto::PublicKey;
use messages::{PROTOCOL_MAJOR_VERSION, TEST_NETWORK_ID};
use node::ApiSender;

#[derive(Serialize, Clone, Debug)]
struct ServiceInfo {
    name: String,
    id: u16,
}

/// `DTO` is used to transfer information about node.
#[derive(Serialize, Clone, Debug)]
pub struct NodeInfo {
    network_id: u8,
    protocol_version: u8,
    services: Vec<ServiceInfo>,
}

impl NodeInfo {
    /// Creates new `NodeInfo`, from services list.
    pub fn new<'a, I>(services: I) -> NodeInfo
    where
        I: IntoIterator<Item = &'a Box<Service>>,
    {
        NodeInfo {
            network_id: TEST_NETWORK_ID,
            protocol_version: PROTOCOL_MAJOR_VERSION,
            services: services
                .into_iter()
                .map(|s| ServiceInfo {
                    name: s.service_name().to_owned(),
                    id: s.service_id(),
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Default)]
struct ReconnectInfo {
    delay: u64,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum IncomingConnectionState {
    Active,
    Reconnect(ReconnectInfo),
}

impl Default for IncomingConnectionState {
    fn default() -> IncomingConnectionState {
        IncomingConnectionState::Active
    }
}

#[derive(Serialize, Default)]
struct IncomingConnection {
    public_key: Option<PublicKey>,
    state: IncomingConnectionState,
}

#[derive(Serialize)]
struct PeersInfo {
    incoming_connections: Vec<SocketAddr>,
    outgoing_connections: HashMap<SocketAddr, IncomingConnection>,
}

/// Private system API.
#[derive(Clone, Debug)]
pub struct SystemApi {
    blockchain: Blockchain,
    info: NodeInfo,
    shared_api_state: SharedNodeState,
    node_channel: ApiSender,
}

impl SystemApi {
    /// Creates a new `public::SystemApi` instance.
    pub fn new(
        info: NodeInfo,
        blockchain: Blockchain,
        shared_api_state: SharedNodeState,
        node_channel: ApiSender,
    ) -> SystemApi {
        SystemApi {
            info,
            blockchain,
            node_channel,
            shared_api_state,
        }
    }

    fn get_peers_info(&self) -> PeersInfo {
        let mut outgoing_connections: HashMap<SocketAddr, IncomingConnection> = HashMap::new();

        for socket in self.shared_api_state.outgoing_connections() {
            outgoing_connections.insert(socket, Default::default());
        }

        for (s, delay) in self.shared_api_state.reconnects_timeout() {
            outgoing_connections
                .entry(s)
                .or_insert_with(Default::default)
                .state = IncomingConnectionState::Reconnect(ReconnectInfo { delay });
        }

        for (s, p) in self.shared_api_state.peers_info() {
            outgoing_connections
                .entry(s)
                .or_insert_with(Default::default)
                .public_key = Some(p);
        }

        PeersInfo {
            incoming_connections: self.shared_api_state.incoming_connections(),
            outgoing_connections,
        }
    }

    fn get_network_info(&self) -> NodeInfo {
        self.info.clone()
    }

    fn peer_add(&self, ip_str: &str) -> Result<(), ApiError> {
        let addr: SocketAddr = ip_str.parse()?;
        self.node_channel.peer_add(addr)?;
        Ok(())
    }
}

impl Api for SystemApi {
    fn wire(&self, router: &mut Router) {
        let self_ = self.clone();
        let peer_add = move |req: &mut Request| -> IronResult<Response> {
            let map = req.get_ref::<Params>().unwrap();
            match map.find(&["ip"]) {
                Some(&ParamsValue::String(ref ip_str)) => {
                    self_.peer_add(ip_str)?;
                    self_.ok_response(&::serde_json::to_value("Ok").unwrap())
                }
                _ => Err(ApiError::IncorrectRequest(
                    "Required parameter of peer 'ip' is missing".into(),
                ))?,
            }
        };

        let self_ = self.clone();
        let peers_info = move |_: &mut Request| -> IronResult<Response> {
            let info = self_.get_peers_info();
            self_.ok_response(&::serde_json::to_value(info).unwrap())
        };

        let self_ = self.clone();
        let network = move |_: &mut Request| -> IronResult<Response> {
            let info = self_.get_network_info();
            self_.ok_response(&::serde_json::to_value(info).unwrap())
        };

        router.get("/v1/peers", peers_info, "peers_info");
        router.post("/v1/peers", peer_add, "peer_add");
        router.get("/v1/network", network, "network_info");
    }
}
