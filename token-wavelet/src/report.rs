use crate::conservation::{ConservationLaw, ConservationResult};
use crate::data::{TokenDatapoint, TokenSeries};
use crate::spectral::SpectralAnalysis;
use crate::wavelet::WaveletComponents;
use serde::{Deserialize, Serialize};

/// The phase of token spending (based on wavelet trend direction).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpendingPhase {
    /// Spending is trending upward
    Rising,
    /// Spending is relatively flat
    Steady,
    /// Spending is trending downward
    Falling,
    /// Spending is volatile — high oscillation relative to trend
    Volatile,
}

/// Model-level budget status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBudgetStatus {
    pub model: String,
    pub spent: f64,
    pub budget: f64,
    pub utilization_pct: f64,
    pub is_warning: bool,
    pub is_exceeded: bool,
}

/// A comprehensive spending report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingReport {
    /// Overall spending phase
    pub current_phase: SpendingPhase,
    /// Budget status across all periods
    pub budget_statuses: Vec<ConservationResult>,
    /// Per-model budget status
    pub model_statuses: Vec<ModelBudgetStatus>,
    /// Predicted overspend for current period
    pub predicted_overspend: Option<f64>,
    /// Wavelet decomposition components
    pub wavelet_components: WaveletComponents,
    /// Model correlation matrix
    pub spectral_analysis: SpectralAnalysis,
    /// Timestamp of report generation
    pub generated_at: f64,
    /// Total tokens consumed
    pub total_tokens: f64,
    /// Number of models contributing
    pub model_count: usize,
}

/// Report builder with fluent API.
pub struct ReportBuilder {
    datapoints: Vec<TokenDatapoint>,
    series: Vec<TokenSeries>,
    budgets: Option<ConservationLaw>,
    period_spending: Vec<(String, f64)>,
}

impl ReportBuilder {
    pub fn new() -> Self {
        Self {
            datapoints: Vec::new(),
            series: Vec::new(),
            budgets: None,
            period_spending: Vec::new(),
        }
    }

    /// Add raw datapoints for spectral analysis.
    pub fn with_datapoints(mut self, data: Vec<TokenDatapoint>) -> Self {
        self.datapoints = data;
        self
    }

    /// Add time series for wavelet decomposition (one per model).
    pub fn with_series(mut self, series: Vec<TokenSeries>) -> Self {
        self.series = series;
        self
    }

    /// Set conservation budgets.
    pub fn with_budgets(mut self, law: ConservationLaw) -> Self {
        self.budgets = Some(law);
        self
    }

    /// Set period spending data for budget checks.
    /// Each entry is (window_name, total_tokens_spent).
    pub fn with_period_spending(mut self, spending: Vec<(String, f64)>) -> Self {
        self.period_spending = spending;
        self
    }

    /// Build the final spending report.
    pub fn build(self) -> SpendingReport {
        let law = self.budgets.unwrap_or_default();

        // Conservation checks
        let budget_statuses = law.check_all(&self.period_spending);

        // Compute total tokens and per-model spending
        let total_tokens: f64 = self.datapoints.iter().map(|dp| dp.total_tokens() as f64).sum();

        // Model-level budget status
        let mut model_spending: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for dp in &self.datapoints {
            *model_spending.entry(dp.model.clone()).or_default() += dp.total_tokens() as f64;
        }
        let model_count = model_spending.len().max(1);
        let model_statuses: Vec<ModelBudgetStatus> = model_spending
            .into_iter()
            .map(|(model, spent)| {
                let budget = law
                    .budgets
                    .first()
                    .map(|b| b.token_limit / model_count as f64)
                    .unwrap_or(f64::MAX);
                let utilization_pct = if budget > 0.0 {
                    (spent / budget) * 100.0
                } else {
                    0.0
                };
                let is_warning = utilization_pct > law.warning_threshold * 100.0;
                let is_exceeded = spent > budget * (1.0 + law.overage_tolerance);
                ModelBudgetStatus {
                    model,
                    spent,
                    budget,
                    utilization_pct,
                    is_warning,
                    is_exceeded,
                }
            })
            .collect();

        // Wavelet analysis on aggregated series
        let aggregated = TokenSeries {
            model: "__all__".to_string(),
            timestamps: Vec::new(),
            values: vec![total_tokens],
            unit: "total".to_string(),
        };

        // Find first actual time series for decomposition
        let wavelet_components = if !self.series.is_empty() {
            crate::wavelet::decompose_series(&self.series[0], false)
        } else {
            crate::wavelet::decompose_series(&aggregated, false)
        };

        // Current phase based on trend
        let current_phase = determine_phase(&wavelet_components, &law);

        // Predicted overspend for daily budget
        let predicted_overspend = if !self.period_spending.is_empty() {
            let daily_spending = self
                .period_spending
                .iter()
                .find(|(p, _)| p == "daily");
            if let Some((_, spent)) = daily_spending {
                law.predict_overspend("daily", *spent, 0.5) // conservative: assume half period elapsed
            } else {
                None
            }
        } else {
            None
        };

        // Spectral analysis
        let spectral_analysis = crate::spectral::analyze(&self.datapoints);

        SpendingReport {
            current_phase,
            budget_statuses,
            model_statuses,
            predicted_overspend,
            wavelet_components,
            spectral_analysis,
            generated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
            total_tokens,
            model_count: self.datapoints.iter().map(|dp| dp.model.as_str()).collect::<std::collections::HashSet<_>>().len(),
        }
    }
}

impl Default for ReportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine spending phase from wavelet components.
fn determine_phase(components: &WaveletComponents, _law: &ConservationLaw) -> SpendingPhase {
    if components.levels == 0 {
        return SpendingPhase::Steady;
    }

    // Check trend direction
    let trend = &components.trend;
    if trend.len() < 2 {
        return SpendingPhase::Steady;
    }

    let first_half = &trend[..trend.len() / 2];
    let second_half = &trend[trend.len() / 2..];

    let mean_first = first_half.iter().sum::<f64>() / first_half.len() as f64;
    let mean_second = second_half.iter().sum::<f64>() / second_half.len() as f64;

    let trend_change = if mean_first.abs() > 1e-10 {
        (mean_second - mean_first) / mean_first.abs()
    } else {
        mean_second - mean_first
    };

    // Check oscillation magnitude
    let osc_mag: f64 = components
        .oscillation
        .iter()
        .map(|v| v.abs())
        .sum::<f64>()
        / components.oscillation.len().max(1) as f64;
    let trend_mag: f64 = components
        .trend
        .iter()
        .map(|v| v.abs())
        .sum::<f64>()
        / components.trend.len().max(1) as f64;

    let volatility_ratio = if trend_mag.abs() > 1e-10 {
        osc_mag / trend_mag.abs()
    } else {
        osc_mag
    };

    if volatility_ratio > 0.5 {
        SpendingPhase::Volatile
    } else if trend_change > 0.1 {
        SpendingPhase::Rising
    } else if trend_change < -0.1 {
        SpendingPhase::Falling
    } else {
        SpendingPhase::Steady
    }
}

/// Format a SpendingReport as a human-readable string.
pub fn format_report(report: &SpendingReport) -> String {
    let mut out = String::new();

    out.push_str("╔══════════════════════════════════════╗\n");
    out.push_str("║       TOKEN SPENDING REPORT          ║\n");
    out.push_str("╚══════════════════════════════════════╝\n\n");

    // Phase
    out.push_str(&format!(
        "Phase:            {:?}\n",
        report.current_phase
    ));
    out.push_str(&format!(
        "Total Tokens:     {:.0}\n",
        report.total_tokens
    ));
    out.push_str(&format!(
        "Models Tracked:   {}\n\n",
        report.model_count
    ));

    // Budget statuses
    out.push_str("── Budget Status ──\n");
    for bs in &report.budget_statuses {
        let icon = if bs.is_exceeded {
            "🔴"
        } else if bs.is_warning {
            "🟡"
        } else {
            "🟢"
        };
        out.push_str(&format!(
            "  {} {:>8}: {:>10.0} / {:<10.0} ({:.1}%)\n",
            icon, bs.period, bs.spent, bs.budget, bs.utilization_pct,
        ));
    }

    // Predicted overspend
    if let Some(os) = report.predicted_overspend {
        out.push_str(&format!(
            "\n  ⚠ Predicted overspend: {:.0} tokens\n",
            os
        ));
    }

    // Model statuses
    out.push_str("\n── Model Status ──\n");
    for ms in &report.model_statuses {
        let icon = if ms.is_exceeded {
            "🔴"
        } else if ms.is_warning {
            "🟡"
        } else {
            "🟢"
        };
        out.push_str(&format!(
            "  {} {:>20}: {:>10.0} / {:<10.0} ({:.1}%)\n",
            icon, ms.model, ms.spent, ms.budget, ms.utilization_pct,
        ));
    }

    // Wavelet analysis
    out.push_str("\n── Wavelet Decomposition ──\n");
    out.push_str(&format!(
        "  Levels: {}\n",
        report.wavelet_components.levels
    ));
    out.push_str(&format!(
        "  Trend entries:   {}\n",
        report.wavelet_components.trend.len()
    ));
    out.push_str(&format!(
        "  Oscillation entries: {}\n",
        report.wavelet_components.oscillation.len()
    ));
    out.push_str(&format!(
        "  Noise entries:   {}\n",
        report.wavelet_components.noise.len()
    ));

    // Communities
    if !report.spectral_analysis.communities.is_empty() {
        out.push_str("\n── Model Communities ──\n");
        for comm in &report.spectral_analysis.communities {
            out.push_str(&format!("  {}: {}\n", comm.name, comm.models.join(", ")));
        }
    }

    // Structural holes
    if !report.spectral_analysis.structural_holes.is_empty() {
        out.push_str("\n── Structural Holes ──\n");
        for (i, hole) in report.spectral_analysis.structural_holes.iter().enumerate() {
            if i >= 3 {
                out.push_str(&format!(
                    "  ... and {} more opportunities\n",
                    report.spectral_analysis.structural_holes.len() - 3
                ));
                break;
            }
            out.push_str(&format!(
                "  [{:.2}] {} ↔ {}: {}\n",
                hole.score, hole.model_a, hole.model_b, hole.reason
            ));
        }
    }

    // Correlation matrix summary
    if !report.spectral_analysis.correlation_matrix.is_empty() {
        out.push_str("\n── Correlations (top pairs) ──\n");
        let mut sorted = report.spectral_analysis.correlation_matrix.pairs.clone();
        sorted.sort_by(|a, b| b.pearson_r.abs().partial_cmp(&a.pearson_r.abs()).unwrap());
        for pair in sorted.iter().take(5) {
            out.push_str(&format!(
                "  {} ↔ {}: r = {:.3}\n",
                pair.model_a, pair.model_b, pair.pearson_r
            ));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_builder() {
        let data = vec![
            TokenDatapoint {
                model: "gpt-4".to_string(),
                timestamp: 0.0,
                tokens_in: 1000,
                tokens_out: 2000,
            },
            TokenDatapoint {
                model: "claude-3".to_string(),
                timestamp: 1.0,
                tokens_in: 500,
                tokens_out: 1500,
            },
        ];

        let report = ReportBuilder::new()
            .with_datapoints(data)
            .with_period_spending(vec![
                ("daily".to_string(), 5000.0),
                ("weekly".to_string(), 5000.0),
            ])
            .build();

        assert_eq!(report.total_tokens, 5000.0);
        assert!(report.model_count >= 1);
        assert!(!report.budget_statuses.is_empty());
    }

    #[test]
    fn test_spending_phase_determination() {
        let components = WaveletComponents {
            trend: vec![100.0, 120.0, 140.0, 160.0, 180.0],
            oscillation: vec![0.0; 5],
            noise: vec![0.0; 5],
            levels: 1,
        };
        let law = ConservationLaw::default();
        let phase = determine_phase(&components, &law);
        assert_eq!(phase, SpendingPhase::Rising);
    }

    #[test]
    fn test_spending_phase_volatile() {
        let components = WaveletComponents {
            trend: vec![100.0; 8],
            oscillation: vec![80.0; 8],
            noise: vec![0.0; 8],
            levels: 1,
        };
        let law = ConservationLaw::default();
        let phase = determine_phase(&components, &law);
        assert_eq!(phase, SpendingPhase::Volatile);
    }

    #[test]
    fn test_format_report() {
        let data = vec![
            TokenDatapoint {
                model: "gpt-4".to_string(),
                timestamp: 0.0,
                tokens_in: 1000,
                tokens_out: 2000,
            },
        ];
        let report = ReportBuilder::new()
            .with_datapoints(data)
            .with_period_spending(vec![("daily".to_string(), 3000.0)])
            .build();
        let formatted = format_report(&report);
        assert!(formatted.contains("TOKEN SPENDING REPORT"));
        assert!(formatted.contains("gpt-4") || formatted.contains("GPT-4"));
    }
}
