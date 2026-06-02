# token-wavelet

> Token spending has structure. Wavelets reveal the rhythm. Conservation laws enforce the budget. Spectral analysis finds the waste.

**token-wavelet** is a Rust crate for AI token spending intelligence. It combines three mathematical techniques to give you complete visibility into how your LLM tokens are being consumed:

1. **🧮 Wavelet Decomposition** — Separate token spending into *trend*, *oscillation*, and *noise* using Haar or Daubechies-4 wavelets
2. **🔒 Conservation Laws** — Enforce one-sided budgets (spending can go up, but never exceed your limits without warning)
3. **🔬 Spectral Analysis** — Build model correlation graphs, detect communities (models used together), and find structural holes (missing models you should adopt)

---

## Features

| Module | What it does |
|--------|-------------|
| `wavelet` | Haar & D4 wavelet decomposition → trend / oscillation / noise components |
| `conservation` | Daily/weekly/monthly budget enforcement with progressive warnings |
| `spectral` | Pearson correlation matrix, community detection, structural hole analysis |
| `report` | `SpendingReport` with phase, budget status, predicted overspend, and full analytics |

## Quick Start

```rust
use token_wavelet::{
    Budget, ConservationLaw, ReportBuilder, TokenDatapoint, TokenSeries,
};

// Collect your data
let datapoints = vec![
    TokenDatapoint {
        model: "gpt-4".to_string(),
        timestamp: 0.0,
        tokens_in: 1000,
        tokens_out: 2000,
    },
    // ...
];

let series = vec![
    TokenSeries {
        model: "gpt-4".to_string(),
        timestamps: vec![0.0, 1.0, 2.0],
        values: vec![3000.0, 3200.0, 3500.0],
        unit: "hour".to_string(),
    },
];

// Configure budgets
let law = ConservationLaw::new(
    vec![
        Budget::new(1_000_000.0, "daily"),
        Budget::new(5_000_000.0, "weekly"),
        Budget::new(20_000_000.0, "monthly"),
    ],
    0.8,    // warning at 80%
    0.1,    // 10% overage tolerance
);

// Generate report
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

## The Three Techniques

### Wavelet Decomposition

Wavelets decompose a time series into frequency bands at different scales. This crate implements:

- **Haar wavelet** — The simplest orthogonal wavelet. Fast, memory-efficient, perfect for detecting step changes in spending patterns.
- **Daubechies-4 (D4)** — Smoother basis functions with 4-tap filters. Better for continuous spending curves.

The decomposition produces three components:
- **Trend**: Smooth approximation (low frequencies) — the big picture
- **Oscillation**: First-level details — daily or weekly patterns
- **Noise**: Higher-level details — random fluctuations and anomalies

### Conservation Laws

One-sided conservation: you can increase spending, but you can't exceed budget without warnings.

- **Green zone**: ≤80% of budget
- **Yellow zone**: 80–100% — warning issued
- **Red zone**: >110% — hard exceeded flag

Predicts overspend by projecting current usage rate across the remaining window.

### Spectral Analysis

Treats model usage as a graph:

- **Correlation matrix**: Pearson correlation between all model pairs (handles unevenly-sampled time series via linear interpolation)
- **Communities**: Models you tend to use together (detected via union-find on high-correlation edges)
- **Structural holes**: Model pairs with negative correlation or zero co-occurrence — these are opportunities to adopt new models in your workflow

## CLI Demo

```bash
cargo run
```

Generates a full spending report with synthetic data showing all three analysis modules in action.

## License

MIT
