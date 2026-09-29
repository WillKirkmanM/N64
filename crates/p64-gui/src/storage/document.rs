use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
#[serde(untagged)]
enum RomSaveDocument {
    Map(Map<String, Value>),
    LegacyTuple((Map<String, Value>, bool)),
}

pub(super) fn decode_romsave(bytes: &[u8]) -> serde_json::Result<Map<String, Value>> {
    serde_json::from_slice(bytes).map(|document| match document {
        RomSaveDocument::Map(data) | RomSaveDocument::LegacyTuple((data, _)) => data,
    })
}
