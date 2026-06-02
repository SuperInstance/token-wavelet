pub mod data;
pub mod conservation;
pub mod wavelet;
pub mod spectral;
pub mod report;

pub use data::{TokenDatapoint, TokenSeries};
pub use conservation::{Budget, BudgetError, ConservationLaw, ConservationResult};
pub use wavelet::{decompose_series, WaveletComponents};
pub use spectral::{analyze, build_correlation_matrix, detect_communities, detect_structural_holes, CorrelationMatrix, ModelCommunity, SpectralAnalysis, StructuralHole};
pub use report::{format_report, ReportBuilder, SpendingPhase, SpendingReport};
