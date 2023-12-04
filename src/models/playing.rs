use songbird::{input::AuxMetadata, tracks::TrackHandle};
use std::sync::Arc;

pub struct Playing {
    pub track_handle: TrackHandle,
    pub metadata: Arc<AuxMetadata>, // Metadata here is only for read purpose and not write behavior is supposed to happen
}
