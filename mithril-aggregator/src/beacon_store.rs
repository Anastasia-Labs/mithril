use async_trait::async_trait;
use mithril_common::entities::Beacon;

#[cfg(test)]
use mockall::automock;

/// BeaconStore represents a beacon store interactor
#[cfg_attr(test, automock)]
#[async_trait]
pub trait BeaconStore: Sync + Send {
    /// Get the current beacon
    async fn get_current_beacon(&self) -> Result<Option<Beacon>, String>;

    /// Set the current beacon
    async fn set_current_beacon(&mut self, beacon: Beacon) -> Result<(), String>;

    /// Reset the current beacon
    async fn reset_current_beacon(&mut self) -> Result<(), String>;
}

/// MemoryBeaconStore is in memory [`BeaconStore`]
pub struct MemoryBeaconStore {
    current_beacon: Option<Beacon>,
}

impl Default for MemoryBeaconStore {
    /// MemoryBeaconStore factory
    fn default() -> Self {
        Self {
            current_beacon: None,
        }
    }
}

#[async_trait]
impl BeaconStore for MemoryBeaconStore {
    async fn get_current_beacon(&self) -> Result<Option<Beacon>, String> {
        Ok(self.current_beacon.clone())
    }

    async fn set_current_beacon(&mut self, beacon: Beacon) -> Result<(), String> {
        self.current_beacon = Some(beacon);
        Ok(())
    }

    async fn reset_current_beacon(&mut self) -> Result<(), String> {
        self.current_beacon = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{BeaconStore, MemoryBeaconStore};
    use mithril_common::fake_data;

    #[tokio::test]
    async fn test_can_store_beacon() {
        let mut sut = MemoryBeaconStore::default();
        let beacon = fake_data::beacon();
        sut.set_current_beacon(beacon.clone())
            .await
            .expect("unexpected error in set_current_beacon");
        let stored_beacon = sut.get_current_beacon().await;

        assert_eq!(Some(beacon), stored_beacon.unwrap());
    }

    #[tokio::test]
    async fn test_reset_current_beacon_ok() {
        let mut sut = MemoryBeaconStore::default();
        sut.set_current_beacon(fake_data::beacon())
            .await
            .expect("unexpected error in set_current_beacon");
        sut.reset_current_beacon()
            .await
            .expect("unexpected error in set_current_beacon");
        let stored_beacon = sut.get_current_beacon().await;

        assert_eq!(None, stored_beacon.unwrap());
    }
}