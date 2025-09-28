use std::{sync::Arc, time::Duration};

use tokio::{runtime::Runtime, task::JoinHandle};

use crate::{CancelRequest, CancellationToken};

pub trait Service {
    fn serve(self, cancellation_token: CancellationToken) -> impl Future<Output = ()> + Send;
}

pub struct ServiceRunner {
    runtime: Arc<Runtime>,
    requests: Vec<CancelRequest>,
    handles: Vec<JoinHandle<()>>,
}

impl ServiceRunner {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self {
            runtime,
            requests: Default::default(),
            handles: Default::default(),
        }
    }

    pub fn push<T>(&mut self, service: T)
    where
        T: Service + Send + 'static,
    {
        let (request, token) = CancellationToken::new();
        let handle = self.runtime.spawn(service.serve(token));

        self.requests.push(request);
        self.handles.push(handle);
    }
}

impl Drop for ServiceRunner {
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
