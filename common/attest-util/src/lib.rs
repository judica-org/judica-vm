use std::{sync::Once, time::Instant};

static START: Once = Once::new();

static mut TIME: Option<Instant> = None;
static mut OFFSET: u64 = 0;

/// get the current time in milliseconds from UNIX_EPOCH
pub fn now() -> u64 {
    START.call_once(|| {
        let t2 = Instant::now();
        let t = std::time::SystemTime::now();
        let delta = t2.elapsed();
        let v = (t.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()
            - (delta.as_millis() / 2)) as u64;

        unsafe {
            OFFSET = v;
            TIME = Some(t2);
        }
    });
    let t = unsafe { OFFSET };
    let i = unsafe { TIME }.unwrap();
    i.elapsed().as_millis() as u64 + t
}

/// Helps with type inference
pub const INFER_UNIT: Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> = Ok(());
pub type AbstractResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;