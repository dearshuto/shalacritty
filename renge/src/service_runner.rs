use std::{sync::Arc, time::Duration};

use crate::{CancelRequest, CancellationToken};

pub trait Service {
    fn serve(self, cancellation_token: CancellationToken) -> impl Future<Output = ()> + Send;
}

pub trait ParametricService {
    type Params;

    fn serve(
        self,
        params: Self::Params,
        cancellation_token: CancellationToken,
    ) -> impl Future<Output = ()> + Send;
}

pub trait Handle {
    fn is_finished(&self) -> bool;
}

pub trait Runtime {
    type Handle: Handle;

    fn spawn<F>(&self, future: F) -> Self::Handle
    where
        F: Future<Output = ()> + Send + 'static;
}

pub struct ServiceRunner<T: Runtime> {
    runtime: T,
    requests: Vec<CancelRequest>,
    handles: Vec<T::Handle>,
}

impl Default for DefaultServiceRunner {
    fn default() -> Self {
        Self {
            runtime: TokioRuntimeAdapter {},
            requests: Default::default(),
            handles: Default::default(),
        }
    }
}

pub type DefaultServiceRunner = ServiceRunner<TokioRuntimeAdapter>;

impl<T: Runtime> ServiceRunner<T> {
    pub fn new(runtime: T) -> Self {
        Self {
            runtime,
            requests: Default::default(),
            handles: Default::default(),
        }
    }

    pub fn push<S>(&mut self, service: S)
    where
        S: Service + Send + 'static,
    {
        let (request, token) = CancellationToken::new();
        let handle = self.runtime.spawn(service.serve(token));

        self.requests.push(request);
        self.handles.push(handle);
    }

    pub fn push_with_params<S>(&mut self, service: S, params: S::Params)
    where
        S: ParametricService + Send + 'static,
    {
        let (request, token) = CancellationToken::new();
        let handle = self.runtime.spawn(service.serve(params, token));

        self.requests.push(request);
        self.handles.push(handle);
    }
}

impl<T: Runtime> Drop for ServiceRunner<T> {
    fn drop(&mut self) {
        while let Some(request) = self.requests.pop() {
            request.send().unwrap();
        }

        while let Some(handle) = self.handles.pop() {
            if !handle.is_finished() {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

impl Handle for tokio::task::JoinHandle<()> {
    fn is_finished(&self) -> bool {
        self.is_finished()
    }
}

pub struct TokioRuntimeAdapter;
impl Runtime for TokioRuntimeAdapter {
    type Handle = tokio::task::JoinHandle<()>;

    fn spawn<F>(&self, future: F) -> Self::Handle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(future)
    }
}

impl Runtime for Arc<tokio::runtime::Runtime> {
    type Handle = tokio::task::JoinHandle<()>;

    fn spawn<F>(&self, future: F) -> Self::Handle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.as_ref().spawn(future)
    }
}
