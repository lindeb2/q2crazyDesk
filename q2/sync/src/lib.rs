//! Last-writer-wins documents for layout, profiles, and settings (R-LAYOUT-7).

use q2_layout::DeviceId;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Versioned<T> {
    pub version: u64,
    pub writer: DeviceId,
    pub value: T,
}

impl<T> Versioned<T> {
    pub fn new(writer: DeviceId, value: T) -> Self {
        Self {
            version: 1,
            writer,
            value,
        }
    }

    pub fn bump(&mut self, writer: DeviceId, value: T) {
        self.version = self.version.saturating_add(1);
        self.writer = writer;
        self.value = value;
    }

    /// Keep `incoming` when its version is higher, or when versions tie and
    /// the incoming writer id is greater. Returns true if we replaced ours.
    pub fn merge_incoming(&mut self, incoming: Self) -> bool
    where
        T: Clone,
    {
        let take = incoming.version > self.version
            || (incoming.version == self.version && incoming.writer > self.writer);
        if take {
            *self = incoming;
            true
        } else {
            false
        }
    }
}

pub fn encode<T: Serialize>(doc: &Versioned<T>) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(doc)
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<Versioned<T>, serde_json::Error> {
    serde_json::from_slice(bytes)
}

#[cfg(test)]
mod tests {
    use q2_layout::Canvas;

    use super::*;

    fn id(s: &str) -> DeviceId {
        DeviceId(s.into())
    }

    #[test]
    fn higher_version_wins() {
        let mut local = Versioned::new(id("win1"), 1u32);
        let mut remote = Versioned::new(id("mac"), 1u32);
        remote.version = 3;
        remote.value = 9;
        assert!(local.merge_incoming(remote));
        assert_eq!(local.value, 9);
        assert_eq!(local.version, 3);
    }

    #[test]
    fn stale_incoming_dropped() {
        let mut local = Versioned::new(id("win1"), Canvas::new());
        local.version = 5;
        let incoming = Versioned::new(id("mac"), Canvas::new());
        assert!(!local.merge_incoming(incoming));
        assert_eq!(local.version, 5);
    }

    #[test]
    fn tie_breaks_on_writer_id() {
        let mut a = Versioned {
            version: 2,
            writer: id("mac"),
            value: 1u32,
        };
        let b = Versioned {
            version: 2,
            writer: id("win1"),
            value: 7u32,
        };
        assert!(a.merge_incoming(b));
        assert_eq!(a.value, 7);
    }
}
