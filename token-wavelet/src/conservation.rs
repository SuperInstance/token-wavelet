use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors related to budget enforcement.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum BudgetError {
    #[error("Budget exceeded: spent {spent} of {budget} ({pct:.1}%)")]
    BudgetExceeded { budget: f64, spent: f64, pct: f64 },

    #[error("Warning: spent {spent} of {budget} ({pct:.1}%) — approaching limit")]
    Warning { budget: f64, spent: f64, pct: f64 },
}

/// A budget limit for a specific time window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub token_limit: f64,
    pub window: String, // "hourly", "daily", "weekly", "monthly"
}

impl Budget {
    pub fn new(limit: f64, window: &str) -> Self {
        Self {
            token_limit: limit,
            window: window.to_string(),
        }
    }
}

/// The result of checking conservation for a period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationResult {
    pub period: String,
    pub budget: f64,
    pub spent: f64,
    pub remaining: f64,
    pub utilization_pct: f64,
    pub is_warning: bool,
    pub is_exceeded: bool,
    pub predicted_overspend: Option<f64>,
}

/// One-sided conservation law: spending can go up, but must not exceed budget
/// without warning. This implements a "soft cap" with progressive warnings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConservationLaw {
    /// Budgets in priority order (daily, weekly, monthly)
    pub budgets: Vec<Budget>,
    /// Warning threshold as fraction of budget (0.0–1.0)
    pub warning_threshold: f64,
    /// Overage allowed before hard block (0.0–1.0, as fraction of budget)
    pub overage_tolerance: f64,
}

impl Default for ConservationLaw {
    fn default() -> Self {
        Self {
            budgets: vec![
                Budget::new(1_000_000.0, "daily"),
                Budget::new(5_000_000.0, "weekly"),
                Budget::new(20_000_000.0, "monthly"),
            ],
            warning_threshold: 0.8,
            overage_tolerance: 0.1,
        }
    }
}

impl ConservationLaw {
    /// Create a new conservation law with custom budgets.
    pub fn new(budgets: Vec<Budget>, warning_threshold: f64, overage_tolerance: f64) -> Self {
        Self {
            budgets,
            warning_threshold,
            overage_tolerance,
        }
    }

    /// Check conservation for a single period with actual spending.
    pub fn check(&self, window: &str, budget: &Budget, spent: f64) -> ConservationResult {
        let remaining = (budget.token_limit - spent).max(0.0);
        let utilization_pct = if budget.token_limit > 0.0 {
            (spent / budget.token_limit) * 100.0
        } else {
            100.0
        };
        let threshold_amount = budget.token_limit * self.warning_threshold;
        let overage_limit = budget.token_limit * (1.0 + self.overage_tolerance);

        let is_warning = spent > threshold_amount && spent <= budget.token_limit;
        let is_exceeded = spent > overage_limit;

        let predicted_overspend = if is_exceeded {
            Some(spent - budget.token_limit)
        } else {
            None
        };

        ConservationResult {
            period: window.to_string(),
            budget: budget.token_limit,
            spent,
            remaining,
            utilization_pct,
            is_warning,
            is_exceeded,
            predicted_overspend,
        }
    }

    /// Check all budgets for cumulative spending data.
    /// `period_spending` maps window name → total tokens spent in that window.
    pub fn check_all(
        &self,
        period_spending: &[(String, f64)],
    ) -> Vec<ConservationResult> {
        let mut results = Vec::new();
        for (period, spent) in period_spending {
            if let Some(budget) = self.budgets.iter().find(|b| b.window == *period) {
                results.push(self.check(period, budget, *spent));
            }
        }
        results
    }

    /// Predict overspend based on current rate and remaining time.
    /// `current_usage` is tokens consumed so far, `period_elapsed` is fraction
    /// of the period completed (0.0–1.0).
    pub fn predict_overspend(
        &self,
        window: &str,
        current_usage: f64,
        period_elapsed: f64,
    ) -> Option<f64> {
        if period_elapsed <= 0.0 || period_elapsed >= 1.0 {
            return None;
        }
        if let Some(budget) = self.budgets.iter().find(|b| b.window == *window) {
            let projected = current_usage / period_elapsed;
            if projected > budget.token_limit {
                return Some(projected - budget.token_limit);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conservation_within_budget() {
        let law = ConservationLaw::default();
        let budget = Budget::new(1000.0, "daily");
        let result = law.check("daily", &budget, 500.0);
        assert!(!result.is_warning);
        assert!(!result.is_exceeded);
        assert!((result.utilization_pct - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_conservation_warning() {
        let law = ConservationLaw::default();
        let budget = Budget::new(1000.0, "daily");
        // 85% usage — above 80% warning threshold
        let result = law.check("daily", &budget, 850.0);
        assert!(result.is_warning);
        assert!(!result.is_exceeded);
    }

    #[test]
    fn test_conservation_exceeded() {
        let law = ConservationLaw::default();
        let budget = Budget::new(1000.0, "daily");
        // 115% of budget — exceeds 10% overage tolerance
        let result = law.check("daily", &budget, 1150.0);
        assert!(result.is_exceeded);
        assert!(result.predicted_overspend.is_some());
    }

    #[test]
    fn test_check_all() {
        let law = ConservationLaw::default();
        let spending = vec![
            ("daily".to_string(), 500_000.0),
            ("weekly".to_string(), 2_000_000.0),
            ("monthly".to_string(), 15_000_000.0),
        ];
        let results = law.check_all(&spending);
        assert_eq!(results.len(), 3);
        // Daily should be fine
        assert!(!results[0].is_warning);
    }

    #[test]
    fn test_predict_overspend() {
        let law = ConservationLaw::default();
        // Used 600K of 1M daily budget, only 25% of day elapsed
        let predicted = law.predict_overspend("daily", 600_000.0, 0.25);
        assert!(predicted.is_some());
        // Projected: 600K / 0.25 = 2.4M, overspend = 1.4M
        assert!((predicted.unwrap() - 1_400_000.0).abs() < 1.0);
    }

    #[test]
    fn test_predict_no_overspend() {
        let law = ConservationLaw::default();
        // Used 100K of 1M budget, half day elapsed → projected 200K, under budget
        let predicted = law.predict_overspend("daily", 100_000.0, 0.5);
        assert!(predicted.is_none());
    }

    #[test]
    fn test_custom_budgets() {
        let budgets = vec![
            Budget::new(100.0, "daily"),
        ];
        let law = ConservationLaw::new(budgets, 0.9, 0.05);
        let result = law.check("daily", &law.budgets[0], 95.0);
        assert!(result.is_warning); // 95% > 90% threshold
        assert!(!result.is_exceeded);
    }
}
