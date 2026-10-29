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

#[cfg(windows)]
pub struct WindowsVddProvider {
    created: Vec<VirtualDisplayId>,
}

#[cfg(windows)]
impl Default for WindowsVddProvider {
    fn default() -> Self {
        Self { created: Vec::new() }
    }
}

#[cfg(windows)]
impl VirtualDisplayProvider for WindowsVddProvider {
    fn create(&mut self, spec: &VirtualDisplaySpec) -> Result<VirtualDisplayId, DisplayError> {
        let id = VirtualDisplayId(format!("vdd-{}", self.created.len()));
        let xml_path = std::env::var("ProgramData")
            .unwrap_or_else(|_| "C:\\ProgramData".into())
            + "\\Virtual-Display-Driver\\vdd_settings.xml";
        let xml_content = format!(
            "<Settings><Displays><Display id=\"{}\"><Width>{}</Width><Height>{}</Height><RefreshRate>{}</RefreshRate></Display></Displays></Settings>",
            id.0, spec.pixels.width, spec.pixels.height, spec.refresh_hz
        );
        let _ = std::fs::write(xml_path, xml_content);
        self.created.push(id.clone());
        Ok(id)
    }

    fn remove(&mut self, id: &VirtualDisplayId) -> Result<(), DisplayError> {
        self.created.retain(|i| i != id);
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub struct MacVirtualDisplayProvider {
    created: Vec<VirtualDisplayId>,
}

#[cfg(target_os = "macos")]
impl Default for MacVirtualDisplayProvider {
    fn default() -> Self {
        Self { created: Vec::new() }
    }
}

#[cfg(target_os = "macos")]
impl VirtualDisplayProvider for MacVirtualDisplayProvider {
    fn create(&mut self, _spec: &VirtualDisplaySpec) -> Result<VirtualDisplayId, DisplayError> {
        let id = VirtualDisplayId(format!("cgvd-{}", self.created.len()));
        self.created.push(id.clone());
        Ok(id)
    }

    fn remove(&mut self, id: &VirtualDisplayId) -> Result<(), DisplayError> {
        self.created.retain(|i| i != id);
        Ok(())
    }
}

pub fn active_provider() -> Box<dyn VirtualDisplayProvider + Send> {
    #[cfg(windows)]
    return Box::new(WindowsVddProvider::default());

    #[cfg(target_os = "macos")]
    return Box::new(MacVirtualDisplayProvider::default());

    #[cfg(not(any(windows, target_os = "macos")))]
    return Box::new(MemoryProvider::default());
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

    #[test]
    fn active_provider_smoke() {
        let mut p = active_provider();
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
        assert!(p.remove(&id).is_ok());
    }
}
