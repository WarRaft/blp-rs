use crate::error::error::BlpError;
use std::error::Error;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Cause {
    Blp(BlpError),
    Std(Arc<dyn Error + Send + Sync>),
}
