//! Geographic and network location types

use serde::{Deserialize, Serialize};

/// Geographic location with optional precision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// Latitude in degrees (-90 to 90)
    pub latitude: f64,
    /// Longitude in degrees (-180 to 180)
    pub longitude: f64,
    /// Optional altitude in meters
    pub altitude: Option<f64>,
    /// Precision radius in meters
    pub precision: Option<f64>,
}

impl Location {
    /// Create a new location
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            altitude: None,
            precision: None,
        }
    }

    /// Calculate distance to another location in meters (Haversine formula)
    pub fn distance_to(&self, other: &Location) -> f64 {
        const EARTH_RADIUS: f64 = 6_371_000.0; // meters

        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let dlat = (other.latitude - self.latitude).to_radians();
        let dlon = (other.longitude - self.longitude).to_radians();

        let a = (dlat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS * c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_calculation() {
        let sf = Location::new(37.7749, -122.4194);
        let la = Location::new(34.0522, -118.2437);

        let distance = sf.distance_to(&la);
        // Approximately 559 km
        assert!((distance - 559_000.0).abs() < 10_000.0);
    }
}
