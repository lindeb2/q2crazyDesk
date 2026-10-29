use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::types::{Canvas, DeviceId};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub devices: BTreeSet<DeviceId>,
    pub canvas: Canvas,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileStore {
    pub profiles: Vec<Profile>,
}

impl ProfileStore {
    pub fn new() -> Self {
        Self {
            profiles: Vec::new(),
        }
    }

    pub fn save_or_update(&mut self, profile: Profile) {
        if let Some(pos) = self.profiles.iter().position(|p| p.devices == profile.devices) {
            self.profiles[pos] = profile;
        } else {
            self.profiles.push(profile);
        }
    }

    pub fn select_for_online(&self, online: &BTreeSet<DeviceId>) -> Option<&Profile> {
        select_profile(&self.profiles, online)
    }
}

/// Exact set wins. Otherwise the profile with the largest intersection,
/// then the smallest extra membership (R-LAYOUT-8).
pub fn select_profile<'a>(
    profiles: &'a [Profile],
    online: &BTreeSet<DeviceId>,
) -> Option<&'a Profile> {
    if profiles.is_empty() {
        return None;
    }
    if let Some(exact) = profiles.iter().find(|p| p.devices == *online) {
        return Some(exact);
    }
    profiles.iter().max_by(|a, b| {
        let ia = a.devices.intersection(online).count();
        let ib = b.devices.intersection(online).count();
        ia.cmp(&ib).then_with(|| {
            let extra_a = a.devices.symmetric_difference(online).count();
            let extra_b = b.devices.symmetric_difference(online).count();
            extra_b.cmp(&extra_a).then_with(|| a.devices.cmp(&b.devices))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> DeviceId {
        DeviceId(s.into())
    }

    fn prof(names: &[&str]) -> Profile {
        Profile {
            devices: names.iter().map(|n| id(n)).collect(),
            canvas: Canvas::new(),
        }
    }

    #[test]
    fn exact_match() {
        let profiles = vec![prof(&["win1", "win2"]), prof(&["win1", "win2", "mac"])];
        let online = ["win1", "win2", "mac"].iter().map(|n| id(n)).collect();
        let got = select_profile(&profiles, &online).unwrap();
        assert_eq!(got.devices.len(), 3);
    }

    #[test]
    fn closest_when_unseen() {
        let profiles = vec![prof(&["win1", "win2"]), prof(&["win1", "mac"])];
        let online = ["win1", "win2", "mac"].iter().map(|n| id(n)).collect();
        let got = select_profile(&profiles, &online).unwrap();
        // both intersect in 2; the three-way set is closer to either; tie broken
        // by extra count (both extra=1) then device order. Either is acceptable
        // as long as we pick one of the two-device profiles.
        assert_eq!(got.devices.len(), 2);
    }

    #[test]
    fn store_save_update_and_select() {
        let mut store = ProfileStore::new();
        store.save_or_update(prof(&["win1", "win2"]));
        store.save_or_update(prof(&["win1", "win2", "mac"]));
        let online: BTreeSet<DeviceId> = ["win1", "win2"].iter().map(|n| id(n)).collect();
        let selected = store.select_for_online(&online).unwrap();
        assert_eq!(selected.devices.len(), 2);
    }
}
