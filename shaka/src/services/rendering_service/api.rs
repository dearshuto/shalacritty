pub enum RenderingServiceRequest {
    Capture,
}

pub struct CaptureRequest {
    pub handle: tokio::sync::oneshot::Sender<CaptureResponse>,
}

#[derive(Debug)]
pub struct CaptureResponse {
    pub data: Vec<u8>,
}
