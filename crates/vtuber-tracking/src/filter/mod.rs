//! Tracking filters: quaternion-centered rotation smoothing and expression
//! normalization / smoothing.

pub mod expression;
pub mod head;

pub use expression::{
    ExpressionCalibration, ExpressionCalibrationError, ExpressionChannel, ExpressionFilter,
    ExpressionFilterParams, ExpressionRange, MissingChannelFallback, MissingChannelPolicy,
};
pub use head::{HeadFilterParams, HeadRotationFilter};
