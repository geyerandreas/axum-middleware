use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tower_layer::Layer;
use tower_service::Service;

#[derive(Debug, Clone)]
pub struct ChaosLayer {
    pub enabled: bool,
    pub error_probability: f64,
    pub min_latency: Duration,
    pub max_latency: Duration,
}

impl ChaosLayer {
    pub fn new(
        enabled: bool,
        error_probability: f64,
        min_latency: Duration,
        max_latency: Duration,
    ) -> Self {
        Self {
            enabled,
            error_probability,
            min_latency,
            max_latency,
        }
    }
}

impl<S> Layer<S> for ChaosLayer {
    type Service = ChaosService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ChaosService::new(
            inner,
            self.enabled,
            self.error_probability,
            self.min_latency,
            self.max_latency,
        )
    }
}

#[derive(Clone, Debug)]
pub struct ChaosService<S> {
    inner: S,
    enabled: bool,
    error_probability: f64,
    min_latency: Duration,
    max_latency: Duration,
}

impl<S> ChaosService<S> {
    pub fn new(
        inner: S,
        enabled: bool,
        error_probability: f64,
        min_latency: Duration,
        max_latency: Duration,
    ) -> Self {
        Self {
            inner,
            enabled,
            error_probability,
            min_latency,
            max_latency,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChaosError {
    message: String,
}

impl ChaosError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ChaosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ChaosError {}

impl<S, Request> Service<Request> for ChaosService<S>
where
    S: Service<Request> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    Request: Send + 'static,
{
    type Response = S::Response;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let enabled = self.enabled;
        let error_probability = self.error_probability;
        let min_latency = self.min_latency;
        let max_latency = self.max_latency;

        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            if !enabled {
                return inner.call(req).await.map_err(Into::into);
            }

            if rand::random::<f64>() < error_probability {
                return Err(Box::new(ChaosError::new("injected chaos failure")));
            }

            let delay = if min_latency == max_latency {
                min_latency
            } else {
                let span = max_latency.saturating_sub(min_latency);
                let jitter_millis = rand::random::<u128>() % span.as_millis().max(1);
                min_latency + Duration::from_millis(jitter_millis as u64)
            };

            tokio::time::sleep(delay).await;

            inner.call(req).await.map_err(Into::into)
        })
    }
}
