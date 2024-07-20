// TODO: reconsider using channeling for request queue
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use common::data::dto::public_request::PublicRequest;
use common::data::dto::public_response::PublicResponse;

pub struct ClientConnection {
    tx: mpsc::Sender<PublicRequest>,
    rx: mpsc::Sender<PublicResponse>
}

pub type ClientMap = Arc<Mutex<HashMap<String, ClientConnection>>>;
