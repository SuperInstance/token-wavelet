use crate::data::TokenSeries;
use serde::{Deserialize, Serialize};

/// The three components of a wavelet decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveletComponents {
    /// The smooth/trend component (approximation coefficients)
    pub trend: Vec<f64>,
    /// The oscillatory component (detail coefficients level 1)
    pub oscillation: Vec<f64>,
    /// The noise component (detail coefficients level 2+)
    pub noise: Vec<f64>,
    /// Number of decomposition levels used
    pub levels: usize,
}

/// Daubechies-4 wavelet coefficients (normalized).
/// These are the scaling (low-pass) and wavelet (high-pass) filter coefficients
/// for the D4 wavelet.
fn d4_scaling() -> [f64; 4] {
    let s3 = 3.0_f64.sqrt();
    let d = 4.0 * 2.0_f64.sqrt();
    [
        (1.0 + s3) / d,
        (3.0 + s3) / d,
        (3.0 - s3) / d,
        (1.0 - s3) / d,
    ]
}

fn d4_wavelet() -> [f64; 4] {
    let h = d4_scaling();
    // Wavelet coefficients: reverse and alternate signs of scaling coeffs
    [h[3], -h[2], h[1], -h[0]]
}

/// Perform a single level of Haar wavelet decomposition.
fn haar_transform_1d(data: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = data.len();
    let half = n / 2;
    let mut approx = Vec::with_capacity(half);
    let mut detail = Vec::with_capacity(half);

    for i in 0..half {
        let a = data[2 * i];
        let b = data[2 * i + 1];
        // Haar: average (scaling) and difference (wavelet)
        approx.push((a + b) / 2.0_f64.sqrt());
        detail.push((a - b) / 2.0_f64.sqrt());
    }

    (approx, detail)
}

/// Perform a single level of D4 wavelet decomposition.
/// This uses periodic boundary conditions (wrap-around).
fn d4_transform_1d(data: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = data.len();
    let half = n / 2;
    let scaling = d4_scaling();
    let wavelet = d4_wavelet();
    let mut approx = Vec::with_capacity(half);
    let mut detail = Vec::with_capacity(half);

    for i in 0..half {
        let mut s = 0.0_f64;
        let mut d = 0.0_f64;
        for k in 0..4 {
            let idx = (2 * i + k) % n;
            s += scaling[k] * data[idx];
            d += wavelet[k] * data[idx];
        }
        approx.push(s);
        detail.push(d);
    }

    (approx, detail)
}

/// Pad data to power-of-two length by reflection (mirroring).
fn pad_to_power_of_two(data: &[f64]) -> Vec<f64> {
    let n = data.len();
    if n.is_power_of_two() {
        return data.to_vec();
    }
    let next_pow2 = n.next_power_of_two();
    let mut padded = Vec::with_capacity(next_pow2);
    padded.extend_from_slice(data);
    // Mirror reflection padding
    let mut i = n;
    while i < next_pow2 {
        let src = if i < n * 2 - 1 {
            2 * n - 2 - i
        } else {
            i % n
        };
        padded.push(data[src]);
        i += 1;
    }
    padded
}

/// Decompose a token time series into trend + oscillation + noise.
///
/// Uses Haar wavelet for simplicity and efficiency. The decomposition proceeds:
/// - Level 0: original signal
/// - Level 1 coefficients → oscillation component
/// - Levels 2+ coefficients → noise component
/// - Final approximation coefficients → trend component
///
/// For D4 wavelet, the same structure applies but with smoother basis functions.
pub fn decompose_series(series: &TokenSeries, use_d4: bool) -> WaveletComponents {
    let n = series.values.len();
    if n < 4 {
        // Not enough data for meaningful decomposition
        return WaveletComponents {
            trend: series.values.clone(),
            oscillation: vec![0.0; n],
            noise: vec![0.0; n],
            levels: 0,
        };
    }

    let padded = pad_to_power_of_two(&series.values);
    let mut current = padded;
    let max_levels = (current.len() as f64).log2().floor() as usize;
    // Use at most 3 levels, or as many as we can
    let levels = max_levels.min(3);

    let mut all_details: Vec<Vec<f64>> = Vec::new();
    let mut approximations = Vec::new();

    for _level in 0..levels {
        let (approx, detail) = if use_d4 {
            d4_transform_1d(&current)
        } else {
            haar_transform_1d(&current)
        };
        all_details.push(detail);
        approximations.push(approx.clone());
        current = approx;
        if current.len() < 2 {
            break;
        }
    }

    // Reconstruct components at original length
    // Trend: upsample the final approximation to original length
    let trend = upsample_to_length(&current, n);

    // Oscillation: combine level-1 details, upsampled to original length
    let oscillation = if !all_details.is_empty() {
        upsample_to_length(&all_details[0], n)
    } else {
        vec![0.0; n]
    };

    // Noise: combine remaining detail levels (2+), upsampled to original length
    let noise = if all_details.len() > 1 {
        let mut noise_signal: Vec<f64> = vec![0.0; all_details[1].len()];
        for d in all_details.iter().skip(1) {
            for (i, v) in d.iter().enumerate() {
                if i < noise_signal.len() {
                    noise_signal[i] += v;
                }
            }
        }
        upsample_to_length(&noise_signal, n)
    } else {
        vec![0.0; n]
    };

    WaveletComponents {
        trend,
        oscillation,
        noise,
        levels,
    }
}

/// Simple upsampling by linear interpolation or repeat.
fn upsample_to_length(data: &[f64], target_len: usize) -> Vec<f64> {
    if data.is_empty() || target_len == 0 {
        return vec![0.0; target_len];
    }
    if data.len() == target_len {
        return data.to_vec();
    }

    let mut result = Vec::with_capacity(target_len);
    let scale = data.len() as f64 / target_len as f64;

    for i in 0..target_len {
        let pos = i as f64 * scale;
        let idx = pos.floor() as usize;
        let frac = pos - idx as f64;
        let idx = idx.min(data.len() - 1);
        let next_idx = (idx + 1).min(data.len() - 1);
        let val = data[idx] * (1.0 - frac) + data[next_idx] * frac;
        result.push(val);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::TokenSeries;

    #[test]
    fn test_haar_transform_power_of_two() {
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        let (approx, detail) = haar_transform_1d(&data);
        assert_eq!(approx.len(), 4);
        assert_eq!(detail.len(), 4);
        //  (10+20)/√2 ≈ 21.21
        assert!((approx[0] - 21.213203).abs() < 1e-3);
        assert!((detail[0] - (10.0 - 20.0) / 2.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_pad_to_power_of_two() {
        let data = vec![1.0, 2.0, 3.0];
        let padded = pad_to_power_of_two(&data);
        assert_eq!(padded.len(), 4);
        assert_eq!(padded[3], data[1]); // mirror
    }

    #[test]
    fn test_decompose_series() {
        let series = TokenSeries {
            model: "test".to_string(),
            timestamps: (0..16).map(|i| i as f64).collect(),
            values: (0..16).map(|i| (i * 100) as f64).collect(),
            unit: "hour".to_string(),
        };
        let comp = decompose_series(&series, false);
        assert_eq!(comp.trend.len(), 16);
        assert_eq!(comp.oscillation.len(), 16);
        assert_eq!(comp.noise.len(), 16);
        assert!(comp.levels > 0);
    }

    #[test]
    fn test_d4_scaling_coefficients() {
        let s = d4_scaling();
        // D4 scaling coefficients should sum to √2 ≈ 1.414
        let sum: f64 = s.iter().sum();
        assert!((sum - 2.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_d4_wavelet_orthogonality() {
        let s = d4_scaling();
        let w = d4_wavelet();
        // Scaling and wavelet sequences should be orthogonal
        let dot: f64 = s.iter().zip(w.iter()).map(|(a, b)| a * b).sum();
        assert!(dot.abs() < 1e-10);
    }
}
