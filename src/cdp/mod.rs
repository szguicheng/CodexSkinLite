mod discovery;

pub use discovery::{
    CdpTarget, endpoint_available, list_targets, pick_primary_target, validate_websocket_url,
};
