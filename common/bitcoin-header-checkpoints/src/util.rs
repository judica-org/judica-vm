/// Helps with type inference
pub const INFER_UNIT: Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> = Ok(());
pub type AbstractResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;
