use token_wavelet::{
    Budget, ConservationLaw, ReportBuilder, TokenDatapoint, TokenSeries,
};
use token_wavelet::{format_report, wavelet::decompose_series};

fn demo_data() -> (Vec<TokenDatapoint>, Vec<TokenSeries>) {
    let mut datapoints = Vec::new();
    let mut gpt4_vals = Vec::new();
    let mut claude_vals = Vec::new();
    let mut gemini_vals = Vec::new();
    let mut timestamps = Vec::new();

    // Simulate 48 hours of token usage with different patterns
    for hour in 0..48 {
        let t = hour as f64;

        // GPT-4: steady increase with some noise
        let gpt4 = 1000.0 + hour as f64 * 30.0 + (hour as f64 * 0.5).sin() * 200.0;
        gpt4_vals.push(gpt4);
        timestamps.push(t);

        // Claude-3: correlated with GPT-4 but slightly different pattern
        let claude = 800.0 + hour as f64 * 20.0 + (hour as f64 * 0.3).cos() * 150.0;
        claude_vals.push(claude);

        // Gemini: every other hour, lower usage
        if hour % 2 == 0 {
            let gemini = 300.0 + (hour as f64 * 0.7).sin() * 100.0;
            gemini_vals.push(gemini);
        }

        // Daily spike at hours 6-10
        let spike = if hour % 24 >= 6 && hour % 24 <= 10 {
            500.0
        } else {
            0.0
        };

        datapoints.push(TokenDatapoint {
            model: "gpt-4".to_string(),
            timestamp: t,
            tokens_in: (gpt4 + spike * 0.6) as u64,
            tokens_out: (gpt4 * 2.0 + spike) as u64,
        });

        datapoints.push(TokenDatapoint {
            model: "claude-3".to_string(),
            timestamp: t,
            tokens_in: (claude + spike * 0.4) as u64,
            tokens_out: (claude * 1.8 + spike * 0.8) as u64,
        });

        if hour % 2 == 0 {
            let gi = (hour / 2) as usize;
            datapoints.push(TokenDatapoint {
                model: "gemini".to_string(),
                timestamp: t,
                tokens_in: (gemini_vals[gi]) as u64,
                tokens_out: (gemini_vals[gi] * 1.5) as u64,
            });
        }
    }

    let gpt4_series = TokenSeries {
        model: "gpt-4".to_string(),
        timestamps: timestamps.clone(),
        values: gpt4_vals,
        unit: "hour".to_string(),
    };

    let claude_series = TokenSeries {
        model: "claude-3".to_string(),
        timestamps: timestamps.clone(),
        values: claude_vals,
        unit: "hour".to_string(),
    };

    let gemini_series = TokenSeries {
        model: "gemini".to_string(),
        timestamps: timestamps.iter().enumerate().filter(|(i, _)| i % 2 == 0).map(|(_, t)| *t).collect(),
        values: gemini_vals,
        unit: "hour".to_string(),
    };

    (datapoints, vec![gpt4_series, claude_series, gemini_series])
}

fn main() {
    println!("token-wavelet: Token Spending Intelligence\n");
    println!("==========================================\n");

    // Load demo data
    let (datapoints, series) = demo_data();

    // Build conservation law with explicit budgets
    let law = ConservationLaw::new(
        vec![
            Budget::new(500_000.0, "daily"),
            Budget::new(3_000_000.0, "weekly"),
            Budget::new(12_000_000.0, "monthly"),
        ],
        0.8,
        0.1,
    );

    // Calculate period spending from demo data
    let daily_spent: f64 = datapoints.iter().take(48).map(|dp| dp.total_tokens() as f64).sum();
    let weekly_spent = daily_spent * 5.0;
    let monthly_spent = daily_spent * 20.0;

    // Build the report
    let report = ReportBuilder::new()
        .with_datapoints(datapoints.clone())
        .with_series(series.clone())
        .with_budgets(law)
        .with_period_spending(vec![
            ("daily".to_string(), daily_spent),
            ("weekly".to_string(), weekly_spent),
            ("monthly".to_string(), monthly_spent),
        ])
        .build();

    // Print formatted report
    println!("{}", format_report(&report));

    // Wavelet analysis on GPT-4 series
    println!("\n\n── Wavelet Deep-Dive: GPT-4 ──\n");
    let components = decompose_series(&series[0], false);
    println!("Trend (first 8):  {:?}", &components.trend[..8]);
    println!("Oscillation (first 8): {:?}", &components.oscillation[..8]);
    println!("Noise (first 8):     {:?}", &components.noise[..8]);
    println!("Levels: {}", components.levels);

    // D4 wavelet comparison
    println!("\n── D4 Wavelet Comparison ──\n");
    let d4_components = decompose_series(&series[0], true);
    println!(
        "D4 Trend (first 8): {:?}",
        &d4_components.trend[..8]
    );
    println!("D4 Levels: {}", d4_components.levels);

    // Spectral analysis deep-dive
    println!("\n── Spectral Analysis Deep-Dive ──\n");
    let analysis = token_wavelet::spectral::analyze(&datapoints);
    println!("Models detected: {}", analysis.models.join(", "));
    println!(
        "Communities: {}",
        analysis.communities.len()
    );
    println!(
        "Structural holes: {}",
        analysis.structural_holes.len()
    );
    for hole in &analysis.structural_holes {
        println!(
            "  [{:.2}] {} ↔ {}",
            hole.score, hole.model_a, hole.model_b
        );
    }

    // Serialize to JSON example
    println!("\n── JSON Report (truncated) ──\n");
    let json = serde_json::to_string_pretty(&report).unwrap();
    let lines: Vec<&str> = json.lines().collect();
    for line in lines.iter().take(40) {
        println!("{}", line);
    }
    println!("  ... ({})", json.lines().count() - 40);

    println!("\nDone. 🧠 Token spending has structure. Wavelets reveal the rhythm. Conservation laws enforce the budget. Spectral analysis finds the waste.");
}
