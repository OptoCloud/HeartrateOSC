#[derive(Clone, serde::Serialize)]
pub struct HeartRateMeasurement {
    pub heart_rate: u16,
    pub sensor_contact_detected: bool,
    pub sensor_contact_supported: bool,
    pub energy_expended_present: bool,
    pub energy_expended: u16,
    pub rr_intervals: Vec<u16>
}