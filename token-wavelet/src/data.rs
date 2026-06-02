use serde::{Deserialize, Serialize};

/// A single token usage data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDatapoint {
    pub model: String,
    pub timestamp: f64,  // seconds since epoch or ordinal index
    pub tokens_in: u64,
    pub tokens_out: u64,
}

impl TokenDatapoint {
    pub fn total_tokens(&self) -> u64 {
        self.tokens_in + self.tokens_out
    }
}

/// A time series of token spending for one model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSeries {
    pub model: String,
    pub timestamps: Vec<f64>,
    pub values: Vec<f64>,  // tokens per time unit
    pub unit: String,       // "hour", "day", "week", "month"
}

impl TokenSeries {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
