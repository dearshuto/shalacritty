use crate::{renderer::IRenderDecorator, IBackend};

pub struct BackgroundRenderer<TBackend: IBackend> {
    backend: TBackend,
}

impl<TBackend: IBackend> IRenderDecorator for BackgroundRenderer<TBackend> {
    fn render(&self) {
        //
    }
}
