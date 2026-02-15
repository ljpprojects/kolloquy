use alloc::sync::Arc;
use handlebars::Handlebars;
use worker::{Context, Env};

#[derive(Clone)]
pub struct WorkerState {
    pub env: Arc<Env>,
    pub ctx: Arc<Context>,
    pub hbars: Arc<Handlebars<'static>>,
}
