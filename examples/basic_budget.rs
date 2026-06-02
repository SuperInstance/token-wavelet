//! Basic budget tracking example.
//!
//! This example shows how to:
//! 1. Create token datapoints
//! 2. Set up daily/weekly/monthly budgets
//! 3. Run conservation law checks
//! 4. Generate and print a spending report

use token_wavelet::{
    Budget, ConservationLaw, ReportBuilder, TokenDatapoint, TokenSeries,
};

fn main() {
    println!("=== Basic Budget Tracking ===\n");

    // Simulate 24 hours of token usage for two models
    let mut datapoints = Vec::new();
    let mut gpt4_values = Vec::new();
    let mut claude_values = Vec::new();
    let mut timestamps = Vec::new();

    for hour in 0..24 {
        let t = hour as f64;

        // GPT-4: gradual increase with a morning spike
        let gpt4 = 1000.0 + hour as f64 * 25.0
            + if (6..=10).contains(&hour) { 400.0 } else { 0.0 };

        // Claude-3: lower baseline, evening peak
        let claude = 700.0 + hour as f64 * 15.0
            + if (14..=18).contains(&hour) { 300.0 } else { 0.0 };

        gpt4_values.push(gpt4);
        claude_values.push(claude);
        timestamps.push(t);

        datapoints.push(TokenDatapoint {
            model: "gpt-4".to_string(),
            timestamp: t,
            tokens_in: gpt4 as u64,
            tokens_out: (gpt4 * 1.5) as u64,
        });
        datapoints.push(TokenDatapoint {
            model: "claude-3".to_string(),
            timestamp: t,
            tokens_in: claude as u64,
            tokens_out: (claude * 1.3) as u64,
        });
    }

    // Build time series
    let gpt4_series = TokenSeries {
        model: "gpt-4".to_string(),
        timestamps: timestamps.clone(),
        values: gpt4_values,
        unit: "hour".to_string(),
    };
    let claude_series = TokenSeries {
        model: "claude-3".to_string(),
        timestamps,
        values: claude_values,
        unit: "hour".to_string(),
    };

    // Calculate totals for budget checks
    let daily_total: f64 = datapoints.iter().map(|dp| dp.total_tokens() as f64).sum();
    let weekly_total = daily_total * 5.0; // projection
    let monthly_total = daily_total * 20.0; // projection

    println!("Daily token burn:   {:.0}", daily_total);
    println!("Projected weekly:   {:.0}", weekly_total);
    println!("Projected monthly:  {:.0}\n", monthly_total);

    // Configure budgets
    let law = ConservationLaw::new(
        vec![
            Budget::new(250_000.0, "daily"),
            Budget::new(1_500_000.0, "weekly"),
            Budget::new(6_000_000.0, "monthly"),
        ],
        0.8,  // warning at 80%
        0.1,  // 10% overage tolerance
    );

    // Build report
    let report = ReportBuilder::new()
        .with_datapoints(datapoints)
        .with_series(vec![gpt4_series, claude_series])
        .with_budgets(law)
        .with_period_spending(vec![
            ("daily".to_string(), daily_total),
            ("weekly".to_string(), weekly_total),
            ("monthly".to_string(), monthly_total),
        ])
        .build();

    // Print report
    println!("{}", token_wavelet::format_report(&report));

    // Check predicted overspend
    if let Some(os) = report.predicted_overspend {
        println!("⚠  Predicted overspend by {:.0} tokens — consider scaling back.", os);
    } else {
        println!("✓ On track to stay within budget.");
    }
}
