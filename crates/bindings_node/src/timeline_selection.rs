use draft_model::{SegmentId, TrackId};

pub fn timeline_track_selection_handle(track_id: &TrackId) -> String {
    format!(
        "timeline-track:{}",
        percent_encode_timeline_handle_component(track_id.as_str())
    )
}

pub fn timeline_segment_selection_handle(track_id: &TrackId, segment_id: &SegmentId) -> String {
    format!(
        "timeline-segment:{}:{}",
        percent_encode_timeline_handle_component(track_id.as_str()),
        percent_encode_timeline_handle_component(segment_id.as_str())
    )
}

pub fn percent_decode_timeline_handle_component(encoded: &str) -> Result<String, ()> {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }

        if index + 2 >= bytes.len() {
            return Err(());
        }

        let high = percent_hex_nibble(bytes[index + 1]).ok_or(())?;
        let low = percent_hex_nibble(bytes[index + 2]).ok_or(())?;
        decoded.push((high << 4) | low);
        index += 3;
    }

    String::from_utf8(decoded).map_err(|_| ())
}

fn percent_encode_timeline_handle_component(raw: &str) -> String {
    let mut encoded = String::with_capacity(raw.len());
    for byte in raw.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn percent_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
