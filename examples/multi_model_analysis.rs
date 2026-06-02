//! Multi-model spectral analysis example.
//!
//! This example demonstrates full spectral analysis across 4 models:
//! 1. Build correlation matrix between all model pairs
//! 2. Detect communities of co-used models
//! 3. Find structural holes (model pairs that should be used together)
//! 4. Wavelet decomposition on each model's time series

use token_wavelet::{
    Budget, ConservationLaw, ReportBuilder, TokenDatapoint, TokenSeries,
};
use token_wavelet::spectral::{analyze, build_correlation_matrix};
use token_wavelet::wavelet::decompose_series;

fn main() {
    println!("=== Multi-Model Spectral Analysis ===\n");

    // Simulate 72 hours of data with 4 models
    let mut datapoints = Vec::new();
    let mut models_data: Vec<(String, Vec<f64>, Vec<f64>)> = vec![
        ("gpt-4".to_string(), Vec::new(), Vec::new()),
        ("claude-3".to_string(), Vec::new(), Vec::new()),
        ("gemini".to_string(), Vec::new(), Vec::new()),
        ("llama-3".to_string(), Vec::new(), Vec::new()),
    ];

    for hour in 0..72 {
        let t = hour as f64;

        // GPT-4: strong weekly pattern (Mon = high, weekend = low)
        let gpt4 = 2000.0 + (t / 7.0).sin() * 800.0 + noise(50.0);

        // Claude-3: correlated with GPT-4 (same user), but damper
        let claude = 1500.0 + (t / 7.0).sin() * 500.0 + (t / 24.0).cos() * 200.0 + noise(30.0);

        // Gemini: every 3 hours, strong noise component
        let gemini = if hour % 3 == 0 {
            1000.0 + (t / 12.0).cos() * 400.0 + noise(100.0)
        } else {
            0.0
        };

        // Llama-3: inversely correlated with GPT-4 (substitute)
        let llama = 1200.0 - (t / 7.0).sin() * 600.0 + (t / 6.0).sin() * 300.0 + noise(40.0);

        // Only push non-zero datapoints
        if gpt4 > 0.0 {
            datapoints.push(TokenDatapoint {
                model: "gpt-4".to_string(),
                timestamp: t,
                tokens_in: gpt4 as u64,
                tokens_out: (gpt4 * 1.8) as u64,
            });
        }
        if claude > 0.0 {
            datapoints.push(TokenDatapoint {
                model: "claude-3".to_string(),
                timestamp: t,
                tokens_in: claude as u64,
                tokens_out: (claude * 1.6) as u64,
            });
        }
        if gemini > 0.0 {
            datapoints.push(TokenDatapoint {
                model: "gemini".to_string(),
                timestamp: t,
                tokens_in: gemini as u64,
                tokens_out: (gemini * 1.4) as u64,
            });
        }
        if llama > 0.0 {
            datapoints.push(TokenDatapoint {
                model: "llama-3".to_string(),
                timestamp: t,
                tokens_in: llama as u64,
                tokens_out: (llama * 1.5) as u64,
            });
        }

        models_data[0].1.push(t);
        models_data[0].2.push(gpt4);
        models_data[1].1.push(t);
        models_data[1].2.push(claude);
        models_data[2].1.push(t);
        models_data[2].2.push(gemini);
        models_data[3].1.push(t);
        models_data[3].2.push(llama);
    }

    // Build time series for wavelet analysis
    let series: Vec<TokenSeries> = models_data
        .into_iter()
        .map(|(model, timestamps, values)| TokenSeries {
            model,
            timestamps,
            values,
            unit: "hour".to_string(),
        })
        .collect();

    // 1. Correlation matrix
    println!("── Correlation Matrix ──\n");
    let matrix = build_correlation_matrix(&datapoints);
    for pair in &matrix.pairs {
        let strength = if pair.pearson_r.abs() > 0.5 {
            "strong"
        } else if pair.pearson_r.abs() > 0.3 {
            "moderate"
        } else {
            "weak"
        };
        println!(
            "  {} ↔ {}: r = {:+.3} ({}, {} overlaps)",
            pair.model_a, pair.model_b, pair.pearson_r, strength, pair.overlap
        );
    }

    // 2. Community detection
    println!("\n── Model Communities ──\n");
    let analysis = analyze(&datapoints);
    if analysis.communities.is_empty() {
        println!("  No strong communities detected (correlation < 0.5)");
    } else {
        for comm in &analysis.communities {
            println!("  Community: {}", comm.models.join(", "));
        }
    }

    // 3. Structural holes
    println!("\n── Structural Holes ──\n");
    if analysis.structural_holes.is_empty() {
        println!("  No structural holes found.");
    } else {
        for hole in &analysis.structural_holes {
            println!(
                "  [{:.2}] {} ↔ {}: {}",
                hole.score, hole.model_a, hole.model_b, hole.reason
            );
        }
    }

    // 4. Wavelet decomposition per model
    println!("\n── Wavelet Decomposition ──\n");
    for s in &series {
        let comp = decompose_series(s, false);
        let trend_dir = if comp.trend.len() >= 4 {
            let first = comp.trend[0];
            let last = comp.trend[comp.trend.len() - 1];
            if last > first * 1.1 {
                "rising 📈"
            } else if last < first * 0.9 {
                "falling 📉"
            } else {
                "stable ➡️"
            }
        } else {
            "unknown"
        };

        let osc_mag: f64 = comp.oscillation.iter().map(|v| v.abs()).sum::<f64>()
            / comp.oscillation.len().max(1) as f64;

        println!(
            "  {:>8}: trend {}, oscillation avg {:.1}, levels {}",
            s.model, trend_dir, osc_mag, comp.levels
        );
    }

    // 5. Full spending report (summarized)
    println!("\n── Budget Summary ──\n");
    let daily_total: f64 = datapoints.iter().map(|dp| dp.total_tokens() as f64).sum();
    let weekly_total = daily_total * 5.0;
    let monthly_total = daily_total * 20.0;

    let law = ConservationLaw::new(
        vec![
            Budget::new(500_000.0, "daily"),
            Budget::new(3_000_000.0, "weekly"),
            Budget::new(12_000_000.0, "monthly"),
        ],
        0.8,
        0.1,
    );

    let report = ReportBuilder::new()
        .with_datapoints(datapoints)
        .with_series(series)
        .with_budgets(law)
        .with_period_spending(vec![
            ("daily".to_string(), daily_total),
            ("weekly".to_string(), weekly_total),
            ("monthly".to_string(), monthly_total),
        ])
        .build();

    println!("{}", token_wavelet::format_report(&report));
}

/// Simple gaussian noise generator.
fn noise(scale: f64) -> f64 {
    // Box-Muller transform using std
    use std::f64::consts::TAU;
    let u1: f64 = fastrand::f64();
    let u2: f64 = fastrand::f64();
    let z = (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos();
    z * scale
}
