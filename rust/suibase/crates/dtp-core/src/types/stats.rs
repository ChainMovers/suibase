#[derive(Default)]
pub struct PingStats {
    pub ping_count_attempted: u8,
    pub success_request: u8,
    pub success_reply: u8,
    pub conn_creation_time: u64,  // milliseconds (can be zero)
    pub min_round_trip_time: u64, // milliseconds
    pub max_round_trip_time: u64, // milliseconds
    pub avg_round_trip_time: u64, // milliseconds
    pub min_gas_cost: u32,        // Mist
    pub avg_gas_cost: u32,        // Mist
    pub max_gas_cost: u32,        // Mist
    pub total_gas_cost: u32,      // Mist
}
