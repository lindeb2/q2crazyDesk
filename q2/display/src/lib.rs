//! Virtual display provider interface for Lent (R-PLACE-3).
//!
//! Windows talks to the signed Virtual-Display-Driver release. macOS talks to
//! a Swift `CGVirtualDisplay` shim that must set size at create time only
//! (see `docs/DECISIONS.md` D-008).

use q2_layout::{MonitorId, SizeMm, SizePx};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VirtualDisplaySpec {
    pub pixels: SizePx,
    pub physical_mm: SizeMm,
    pub refresh_hz: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VirtualDisplayId(pub String);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LentSession {
    pub id: VirtualDisplayId,
    pub lender_monitor: MonitorId,
    pub borrower: q2_layout::DeviceId,
    pub spec: VirtualDisplaySpec,
}

#[derive(Debug)]
pub enum DisplayError {
    Unsupported,
    CreateFailed(String),
    RemoveFailed(String),
}

pub trait VirtualDisplayProvider {
    fn create(&mut self, spec: &VirtualDisplaySpec) -> Result<VirtualDisplayId, DisplayError>;
    fn remove(&mut self, id: &VirtualDisplayId) -> Result<(), DisplayError>;
}

/// In-memory provider used by tests and by the headless integration harness.
#[derive(Default)]
pub struct MemoryProvider {
    pub created: Vec<(VirtualDisplayId, VirtualDisplaySpec)>,
}

impl VirtualDisplayProvider for MemoryProvider {
    fn create(&mut self, spec: &VirtualDisplaySpec) -> Result<VirtualDisplayId, DisplayError> {
        let id = VirtualDisplayId(format!("mem-{}", self.created.len()));
        self.created.push((id.clone(), spec.clone()));
        Ok(id)
    }

    fn remove(&mut self, id: &VirtualDisplayId) -> Result<(), DisplayError> {
        let before = self.created.len();
        self.created.retain(|(i, _)| i != id);
        if self.created.len() == before {
            Err(DisplayError::RemoveFailed(id.0.clone()))
        } else {
            Ok(())
        }
    }
}

pub fn spec_from_lender(pixels: SizePx, physical_mm: SizeMm) -> VirtualDisplaySpec {
    VirtualDisplaySpec {
        pixels,
        physical_mm,
        refresh_hz: 60,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_create_and_remove() {
        let mut p = MemoryProvider::default();
        let spec = spec_from_lender(
            SizePx {
                width: 1920,
                height: 1080,
            },
            SizeMm {
                width: 530.0,
                height: 300.0,
            },
        );
        let id = p.create(&spec).unwrap();
        assert_eq!(p.created.len(), 1);
        p.remove(&id).unwrap();
        assert!(p.created.is_empty());
    }
}
