pub enum GraphicsServiceRequest {}

struct GraphicsServiceRequestAdapter {
    request: GraphicsServiceRequest,
    sender: tokio::sync::oneshot::Sender<u64>,
}

pub struct GraphicsServiceFacade {
    sender: tokio::sync::mpsc::Sender<GraphicsServiceRequestAdapter>,
}

impl GraphicsServiceFacade {
    pub fn new(sender: tokio::sync::mpsc::Sender<GraphicsServiceRequestAdapter>) -> Self {
        todo!()
    }

    pub async fn request<T>(self, request: GraphicsServiceRequest) -> T
    where
        T: ash::vk::Handle,
    {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request_arapter = GraphicsServiceRequestAdapter { request, sender };
        self.sender.send(request_arapter).await;
        let handle = receiver.await.unwrap();
        T::from_raw(handle)
    }
}
