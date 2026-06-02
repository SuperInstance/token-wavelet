# token-wavelet 🧮🔒🔬

> Token spending has structure. Wavelets reveal the rhythm. Conservation laws enforce the budget. Spectral analysis finds the waste.

[![CI](https://github.com/SuperInstance/token-wavelet/actions/workflows/ci.yml/badge.svg)](https://github.com/SuperInstance/token-wavelet/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org)

**token-wavelet** is a Rust crate for AI token spending intelligence. It combines three mathematical techniques to give you complete visibility into how your LLM tokens are being consumed:

1. **🧮 Wavelet Decomposition** — Separate token spending into *trend*, *oscillation*, and *noise* using Haar or Daubechies-4 wavelets
2. **🔒 Conservation Laws** — Enforce one-sided budgets (spending can go up, but never exceed your limits without warning)
3. **🔬 Spectral Analysis** — Build model correlation graphs, detect communities (models used together), and find structural holes (missing models you should adopt)

---

## The Problem

AI token spending is **opaque**. You get a monthly bill, not understanding. Traditional tracking answers "how much did I spend?" but not *why*, *what pattern*, or *what's coming next*.

- **Simple tracking**: Counting tokens per model. Total spent. Maybe a trendline.
- **Wavelet intelligence**: Frequency decomposition reveals spending *rhythms*. Conservation laws *predict* overspend before it happens. Spectral analysis shows *which models belong together*.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
token-wavelet = "0.1"
```

```rust
use token_wavelet::{
    Budget, ConservationLaw, ReportBuilder, TokenDatapoint, TokenSeries,
};

// Your token data
let datapoints = vec![
    TokenDatapoint {
        model: "gpt-4".to_string(),
        timestamp: 0.0,
        tokens_in: 1000,
        tokens_out: 2000,
    },
];

let series = vec![
    TokenSeries {
        model: "gpt-4".to_string(),
        timestamps: vec![0.0, 1.0, 2.0],
        values: vec![3000.0, 3200.0, 3500.0],
        unit: "hour".to_string(),
    },
];

// Set budgets with progressive warnings
let law = ConservationLaw::new(
    vec![
        Budget::new(1_000_000.0, "daily"),
        Budget::new(5_000_000.0, "weekly"),
        Budget::new(20_000_000.0, "monthly"),
    ],
    0.8,    // warning at 80%
    0.1,    // 10% overage tolerance
);

// Generate a full intelligence report
let report = ReportBuilder::new()
    .with_datapoints(datapoints)
    .with_series(series)
    .with_budgets(law)
    .with_period_spending(vec![
        ("daily".to_string(), 500_000.0),
    ])
    .build();

println!("Phase: {:?}", report.current_phase);
for bs in &report.budget_statuses {
    println!("  {}: {:.1}% used", bs.period, bs.utilization_pct);
}
```

## Architecture

```
TokenDatapoint ──► spectral::analyze() ──► CorrelationMatrix
                                          ├── ModelCommunity[]
                                          └── StructuralHole[]

TokenSeries ──────► wavelet::decompose_series() ──► WaveletComponents
                                                    ├── trend: Vec<f64>
                                                    ├── oscillation: Vec<f64>
                                                    ├── noise: Vec<f64>
                                                    └── levels: usize

ConservationLaw ─► check_all() ──► ConservationResult[]
                  ├── predict_overspend()
                  └── check()
```

## User Guide

### Setting Budgets

Budgets are one-sided conservation limits — you can spend *up to* your limit, but the system warns you progressively:

| Zone | Utilization | Signal |
|------|-------------|--------|
| 🟢 Green | ≤80% | All clear |
| 🟡 Yellow | 80–100% | Warning issued |
| 🔴 Red | >110% | Hard exceeded |

```rust
let law = ConservationLaw::new(
    vec![
        Budget::new(1_000_000.0, "daily"),
        Budget::new(5_000_000.0, "weekly"),
        Budget::new(20_000_000.0, "monthly"),
    ],
    0.8,   // warn at 80%
    0.1,   // allow 10% overage before hard block
);

// Predict overspend mid-cycle
if let Some(overspend) = law.predict_overspend("daily", 600_000.0, 0.25) {
    println!("On track to exceed daily budget by {:.0} tokens", overspend);
}
```

### Interpreting Wavelet Components

After decomposition, the three components tell you:

- **Trend**: The smooth direction — is spending growing, shrinking, or stable?
- **Oscillation**: Weekly patterns, daily rhythms, recurring spikes
- **Noise**: Random fluctuations and anomalies — unexpected jumps or drops

```rust
let components = decompose_series(&gpt4_series, false /* use Haar */);
println!("Trend direction: {:?}", &components.trend[..5]);

// Oscillation reveals the daily spike pattern
let avg_oscillation = components.oscillation.iter().map(|v| v.abs()).sum::<f64>()
    / components.oscillation.len() as f64;
println!("Avg oscillation magnitude: {:.1}", avg_oscillation);
```

### Reading the Correlation Matrix

```rust
let analysis = spectral::analyze(&datapoints);

for pair in &analysis.correlation_matrix.pairs {
    println!("{} ↔ {}: r = {:.3} ({} overlaps)",
        pair.model_a, pair.model_b, pair.pearson_r, pair.overlap);
}
```

A correlation near **+1** means the models follow the same spending pattern. Near **-1** means they're substitutes — one goes up when the other goes down. Communities are groups of models with |r| > threshold. Structural holes are pairs that *should* be used together but aren't.

## Templates

### Solo Developer

Track a few models with a modest monthly budget:

```rust
let law = ConservationLaw::new(
    vec![Budget::new(500_000.0, "monthly")],
    0.8, 0.1,
);
```

### Small Team

Track 3-5 models across daily and monthly budgets:

```rust
let law = ConservationLaw::new(
    vec![
        Budget::new(200_000.0, "daily"),
        Budget::new(5_000_000.0, "monthly"),
    ],
    0.85, 0.05,
);
```

### Enterprise

Multi-department tracking with all three windows:

```rust
let law = ConservationLaw::new(
    vec![
        Budget::new(5_000_000.0, "daily"),
        Budget::new(30_000_000.0, "weekly"),
        Budget::new(120_000_000.0, "monthly"),
    ],
    0.75, 0.15,
);
```

Plus spectral analysis across 10+ models to find workflow optimization opportunities.

## Real Scenario

> "You spend $200/day on Claude API calls. Wavelets show a strong weekly spike every Monday when batch jobs run. The conservation law catches it before you exceed your $5000/month budget. Spectral analysis shows Claude and GPT-4 are highly negatively correlated — they're substitutes. You could save 15% by routing Monday batch jobs to GPT-4-mini."

## Modules

| Module | What it does |
|--------|-------------|
| `wavelet` | Haar & D4 wavelet decomposition → trend / oscillation / noise components |
| `conservation` | Daily/weekly/monthly budget enforcement with progressive warnings |
| `spectral` | Pearson correlation matrix, community detection, structural hole analysis |
| `report` | `SpendingReport` with phase, budget status, predicted overspend, and full analytics |

## CLI Demo

```bash
cd token-wavelet
cargo run
```

Generates a full spending report with synthetic data showing all three analysis modules in action.

## Examples

```bash
# Basic budget tracking
cargo run --example basic_budget

# Multi-model spectral analysis
cargo run --example multi_model_analysis
```

## License

MIT
