mod math;

mod delta;
mod velocity;
mod regression;
mod forecast;
mod correlation;
mod trajectory;

pub use delta::{Delta, compare_dimensions};
pub use velocity::{Direction, Velocity, calculate_velocities};
pub use regression::{RegressionSeverity, Regression, detect_regressions};
pub use forecast::{Forecast, forecast_breaches};
pub use correlation::{CorrelationStrength, Correlation, analyze_correlations};
pub use trajectory::{Trajectory, build_trajectory};
pub use math::dimension_series;
