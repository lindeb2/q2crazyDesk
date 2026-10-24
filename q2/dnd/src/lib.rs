//! Cross-machine file drag protocol (R-FILES-2).
//!
//! The transfer itself reuses RustDesk's file channel. This crate is the
//! handshake: a drag left through an edge, the file list, progress, and a
//! request to start a synthetic drop under the cursor on the target.

use q2_layout::{DeviceId, MonitorId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DragFile {
    pub relative_path: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DragMessage {
    Begin {
        from: DeviceId,
        to: DeviceId,
        origin_monitor: MonitorId,
        files: Vec<DragFile>,
    },
    Chunk {
        offset: u64,
        data: Vec<u8>,
    },
    Progress {
        bytes_done: u64,
        bytes_total: u64,
    },
    ReadyToDrop {
        dest_monitor: MonitorId,
        dest_px: (f64, f64),
    },
    Cancel,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragPhase {
    Idle,
    Collecting,
    Streaming,
    Dropping,
}

#[derive(Clone, Debug)]
pub struct DragSession {
    pub phase: DragPhase,
    pub files: Vec<DragFile>,
    pub bytes_done: u64,
}

impl DragSession {
    pub fn new() -> Self {
        Self {
            phase: DragPhase::Idle,
            files: Vec::new(),
            bytes_done: 0,
        }
    }

    pub fn apply(&mut self, msg: &DragMessage) {
        match msg {
            DragMessage::Begin { files, .. } => {
                self.phase = DragPhase::Streaming;
                self.files = files.clone();
                self.bytes_done = 0;
            }
            DragMessage::Progress { bytes_done, .. } => {
                self.bytes_done = *bytes_done;
            }
            DragMessage::ReadyToDrop { .. } => {
                self.phase = DragPhase::Dropping;
            }
            DragMessage::Cancel => {
                *self = Self::new();
            }
            DragMessage::Done => {
                self.phase = DragPhase::Idle;
            }
            DragMessage::Chunk { .. } => {
                self.phase = DragPhase::Streaming;
            }
        }
    }

    pub fn bytes_total(&self) -> u64 {
        self.files.iter().map(|f| f.size).sum()
    }
}

impl Default for DragSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use q2_layout::{DeviceId, MonitorId};

    use super::*;

    #[test]
    fn begin_then_done() {
        let mut s = DragSession::new();
        s.apply(&DragMessage::Begin {
            from: DeviceId("win1".into()),
            to: DeviceId("mac".into()),
            origin_monitor: MonitorId {
                device: DeviceId("win1".into()),
                local_index: 0,
            },
            files: vec![DragFile {
                relative_path: "a.txt".into(),
                size: 10,
            }],
        });
        assert_eq!(s.phase, DragPhase::Streaming);
        assert_eq!(s.bytes_total(), 10);
        s.apply(&DragMessage::Progress {
            bytes_done: 10,
            bytes_total: 10,
        });
        s.apply(&DragMessage::Done);
        assert_eq!(s.phase, DragPhase::Idle);
    }
}
