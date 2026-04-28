use renge::ParametricService;

pub struct ImageId {}

impl ImageId {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct ImageService {}

impl ImageService {
    pub fn new() -> Self {
        Self {}
    }
}

// impl ParametricService for ImageService {
//     type Params;

//     fn serve(
//         self,
//         params: Self::Params,
//         cancellation_token: renge::CancellationToken,
//     ) -> impl Future<Output = ()> + Send {
//         todo!()
//     }
// }
