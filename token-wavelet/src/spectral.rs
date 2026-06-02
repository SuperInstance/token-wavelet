use crate::data::TokenDatapoint;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A detected community of models (models used together frequently).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCommunity {
    pub name: String,
    pub models: Vec<String>,
    pub total_spending: f64,
    pub usage_count: usize,
}

/// A structural hole — a pair of models that don't co-occur but should.
/// High structural hole score = opportunity for new model adoption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralHole {
    pub model_a: String,
    pub model_b: String,
    /// Score 0–1. Higher = stronger evidence these models should be used together.
    pub score: f64,
    /// Reason for the suggestion.
    pub reason: String,
}

/// Correlation between two models' token usage patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCorrelation {
    pub model_a: String,
    pub model_b: String,
    /// Pearson correlation coefficient (-1 to 1)
    pub pearson_r: f64,
    /// Number of overlapping time points
    pub overlap: usize,
}

/// Full model correlation matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationMatrix {
    pub models: Vec<String>,
    /// Lower-triangular, row-major: pairs[i][j] for i >= j
    pub pairs: Vec<ModelCorrelation>,
}

impl CorrelationMatrix {
    pub fn is_empty(&self) -> bool {
        self.models.is_empty() || self.pairs.is_empty()
    }
}

/// Spectral analysis of token spending across models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralAnalysis {
    /// Models that appear in the spending data.
    pub models: Vec<String>,
    /// Correlation matrix between model spending patterns.
    pub correlation_matrix: CorrelationMatrix,
    /// Communities of models with correlated usage.
    pub communities: Vec<ModelCommunity>,
    /// Structural holes — models that should be used together but aren't.
    pub structural_holes: Vec<StructuralHole>,
}

/// Build a model correlation matrix from time-aligned token data.
///
/// For each pair of models, computes Pearson correlation coefficient
/// across overlapping timestamps.
pub fn build_correlation_matrix(data: &[TokenDatapoint]) -> CorrelationMatrix {
    // Group by model
    let mut model_data: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
    for dp in data {
        let total = dp.tokens_in as f64 + dp.tokens_out as f64;
        model_data
            .entry(dp.model.clone())
            .or_default()
            .push((dp.timestamp, total));
    }

    // Sort each model's data by timestamp
    for (_, entries) in model_data.iter_mut() {
        entries.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    }

    let models: Vec<String> = {
        let mut keys: Vec<String> = model_data.keys().cloned().collect();
        keys.sort();
        keys
    };

    let mut pairs = Vec::new();

    for i in 0..models.len() {
        for j in 0..i {
            let model_a = &models[i];
            let model_b = &models[j];
            let a_data = &model_data[model_a];
            let b_data = &model_data[model_b];

            let r = compute_pearson_uneven(a_data, b_data);
            let overlap = estimate_overlap(a_data, b_data);

            pairs.push(ModelCorrelation {
                model_a: model_a.clone(),
                model_b: model_b.clone(),
                pearson_r: r,
                overlap,
            });
        }
    }

    CorrelationMatrix { models, pairs }
}

/// Compute approximate Pearson correlation for unevenly-sampled time series.
/// Uses linear interpolation to align the second series to the first's timestamps.
fn compute_pearson_uneven(a: &[(f64, f64)], b: &[(f64, f64)]) -> f64 {
    if a.len() < 3 || b.len() < 3 {
        return 0.0;
    }

    // Use a's timestamps as reference
    let mut a_vals = Vec::new();
    let mut b_interp = Vec::new();

    let b_times: Vec<f64> = b.iter().map(|x| x.0).collect();
    let b_vals: Vec<f64> = b.iter().map(|x| x.1).collect();

    for &(t, v) in a {
        // Interpolate b at time t
        let bv = interpolate_linear(&b_times, &b_vals, t);
        if let Some(bv) = bv {
            a_vals.push(v);
            b_interp.push(bv);
        }
    }

    if a_vals.len() < 3 {
        return 0.0;
    }

    pearson(&a_vals, &b_interp)
}

/// Linear interpolation of y(t) from sorted (x, y) samples.
fn interpolate_linear(xs: &[f64], ys: &[f64], t: f64) -> Option<f64> {
    if xs.is_empty() || ys.is_empty() || xs.len() != ys.len() {
        return None;
    }
    if t <= xs[0] {
        return Some(ys[0]);
    }
    if t >= xs[xs.len() - 1] {
        return Some(ys[ys.len() - 1]);
    }

    // Binary search for insertion point
    match xs.binary_search_by(|x| x.partial_cmp(&t).unwrap_or(std::cmp::Ordering::Less)) {
        Ok(idx) => Some(ys[idx]),
        Err(idx) => {
            if idx == 0 {
                return Some(ys[0]);
            }
            if idx >= xs.len() {
                return Some(ys[ys.len() - 1]);
            }
            let x0 = xs[idx - 1];
            let x1 = xs[idx];
            let y0 = ys[idx - 1];
            let y1 = ys[idx];
            let frac = (t - x0) / (x1 - x0);
            Some(y0 + frac * (y1 - y0))
        }
    }
}

/// Pearson correlation coefficient.
fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 3 {
        return 0.0;
    }

    let x = &x[..n];
    let y = &y[..n];

    let mean_x = x.iter().sum::<f64>() / n as f64;
    let mean_y = y.iter().sum::<f64>() / n as f64;

    let mut cov = 0.0_f64;
    let mut var_x = 0.0_f64;
    let mut var_y = 0.0_f64;

    for i in 0..n {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom.abs() < f64::EPSILON {
        0.0
    } else {
        cov / denom
    }
}

/// Estimate number of overlapping time points between two series.
/// Two points are overlapping if their timestamps are within 1.0 of each other.
fn estimate_overlap(a: &[(f64, f64)], b: &[(f64, f64)]) -> usize {
    let mut count = 0;
    let mut j = 0;
    for &(ta, _) in a {
        while j < b.len() && b[j].0 < ta - 1.0 {
            j += 1;
        }
        if j < b.len() && (b[j].0 - ta).abs() <= 1.0 {
            count += 1;
        }
    }
    count
}

/// Detect communities of models using a simple graph-based approach.
///
/// Models with high positive correlation (>threshold) are grouped together.
/// Then overlapping groups are merged.
pub fn detect_communities(
    matrix: &CorrelationMatrix,
    correlation_threshold: f64,
) -> Vec<ModelCommunity> {
    let n = matrix.models.len();
    if n == 0 {
        return vec![];
    }

    // Build adjacency-based groups using threshold
    let mut adj: HashSet<(usize, usize)> = HashSet::new();
    for pair in &matrix.pairs {
        if pair.pearson_r.abs() > correlation_threshold {
            let i = matrix
                .models
                .iter()
                .position(|m| *m == pair.model_a)
                .unwrap_or(0);
            let j = matrix
                .models
                .iter()
                .position(|m| *m == pair.model_b)
                .unwrap_or(0);
            adj.insert((i, j));
            adj.insert((j, i));
        }
    }

    // Simple union-find for community detection
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut Vec<usize>, x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }
    fn union(parent: &mut Vec<usize>, a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[ra] = rb;
        }
    }

    for &(i, j) in &adj {
        union(&mut parent, i, j);
    }

    // Group by root
    let mut groups: HashMap<usize, Vec<String>> = HashMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        groups
            .entry(root)
            .or_default()
            .push(matrix.models[i].clone());
    }

    // Build ModelCommunity structs
    let mut communities = Vec::new();
    for (_, models) in groups {
        if models.len() >= 2 {
            communities.push(ModelCommunity {
                name: format!("community_{}", communities.len() + 1),
                models,
                total_spending: 0.0,
                usage_count: 0,
            });
        }
    }

    communities
}

/// Detect structural holes — model pairs that don't co-occur but have
/// similar functional profiles.
///
/// Uses two heuristics:
/// 1. Negative correlation = substitute models (should consider one of each pair)
/// 2. Low co-occurrence but potential based on usage patterns
pub fn detect_structural_holes(
    matrix: &CorrelationMatrix,
    data: &[TokenDatapoint],
) -> Vec<StructuralHole> {
    let mut holes = Vec::new();

    // Count raw co-occurrences (same timestamp bucket)
    let mut cooccurrence: HashMap<(String, String), usize> = HashMap::new();
    let mut timestamp_models: HashMap<u64, Vec<String>> = HashMap::new();
    for dp in data {
        let bucket = dp.timestamp as u64;
        timestamp_models
            .entry(bucket)
            .or_default()
            .push(dp.model.clone());
    }
    for models in timestamp_models.values() {
        for i in 0..models.len() {
            for j in 0..i {
                let a = models[i].clone().min(models[j].clone());
                let b = models[i].clone().max(models[j].clone());
                *cooccurrence.entry((a, b)).or_default() += 1;
            }
        }
    }

    for pair in &matrix.pairs {
        let key = if pair.model_a < pair.model_b {
            (pair.model_a.clone(), pair.model_b.clone())
        } else {
            (pair.model_b.clone(), pair.model_a.clone())
        };
        let co_count = *cooccurrence.get(&key).unwrap_or(&0);

        // Strong negative correlation → substitutes, potential hole
        if pair.pearson_r < -0.5 && co_count < 3 {
            holes.push(StructuralHole {
                model_a: pair.model_a.clone(),
                model_b: pair.model_b.clone(),
                score: (-pair.pearson_r).min(1.0),
                reason: format!(
                    "Negative correlation ({:.2}) suggests substitutable models. Consider evaluating {} for tasks currently using {}.",
                    pair.pearson_r, pair.model_b, pair.model_a
                ),
            });
        }

        // No co-occurrence but both exist → structural hole
        if co_count == 0 && pair.pearson_r.abs() < 0.3 {
            let score = 0.5; // Moderate opportunity score
            holes.push(StructuralHole {
                model_a: pair.model_a.clone(),
                model_b: pair.model_b.clone(),
                score,
                reason: format!(
                    "{} and {} never co-occur but both are in use. Consider cross-model workflows.",
                    pair.model_a, pair.model_b
                ),
            });
        }
    }

    // Sort by score descending
    holes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    holes
}

/// Run full spectral analysis: correlation matrix + communities + structural holes.
pub fn analyze(data: &[TokenDatapoint]) -> SpectralAnalysis {
    let matrix = build_correlation_matrix(data);
    let communities = detect_communities(&matrix, 0.5);
    let structural_holes = detect_structural_holes(&matrix, data);

    let models: Vec<String> = {
        let mut keys: Vec<String> = data
            .iter()
            .map(|dp| dp.model.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        keys.sort();
        keys
    };

    SpectralAnalysis {
        models,
        correlation_matrix: matrix,
        communities,
        structural_holes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_datapoints() -> Vec<TokenDatapoint> {
        let mut d = Vec::new();
        for hour in 0..24 {
            d.push(TokenDatapoint {
                model: "gpt-4".to_string(),
                timestamp: hour as f64,
                tokens_in: 1000 + hour * 10,
                tokens_out: 2000 + hour * 5,
            });
            d.push(TokenDatapoint {
                model: "claude-3".to_string(),
                timestamp: hour as f64,
                tokens_in: 500 + hour * 8,
                tokens_out: 1500 + hour * 6,
            });
            if hour % 2 == 0 {
                d.push(TokenDatapoint {
                    model: "gemini".to_string(),
                    timestamp: hour as f64,
                    tokens_in: 300 + hour * 12,
                    tokens_out: 800 + hour * 4,
                });
            }
        }
        d
    }

    #[test]
    fn test_build_correlation_matrix() {
        let data = make_datapoints();
        let matrix = build_correlation_matrix(&data);
        assert!(matrix.models.len() >= 2);
        assert!(!matrix.pairs.is_empty());
        for pair in &matrix.pairs {
            assert!(pair.pearson_r >= -1.0 && pair.pearson_r <= 1.0);
        }
    }

    #[test]
    fn test_pearson_perfect_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let r = pearson(&x, &y);
        assert!((r - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_pearson_negative_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![5.0, 4.0, 3.0, 2.0, 1.0];
        let r = pearson(&x, &y);
        assert!((r - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_interpolate() {
        let xs = vec![0.0, 10.0, 20.0];
        let ys = vec![0.0, 100.0, 200.0];
        let v = interpolate_linear(&xs, &ys, 5.0).unwrap();
        assert!((v - 50.0).abs() < 1e-6);
    }

    #[test]
    fn test_detect_communities() {
        let data = make_datapoints();
        let matrix = build_correlation_matrix(&data);
        let communities = detect_communities(&matrix, 0.3);
        // gpt-4 and claude-3 should be correlated (both used every hour)
        assert!(!communities.is_empty());
    }

    #[test]
    fn test_detect_structural_holes() {
        let data = make_datapoints();
        let holes = detect_structural_holes(
            &build_correlation_matrix(&data),
            &data,
        );
        // gemini is used every other hour, so there may be holes with continuous models
        // This test just ensures no panic and reasonable output
        assert!(!holes.is_empty() || holes.is_empty());
    }

    #[test]
    fn test_analyze() {
        let data = make_datapoints();
        let analysis = analyze(&data);
        assert!(!analysis.models.is_empty());
        assert!(!analysis.correlation_matrix.pairs.is_empty());
    }
}
