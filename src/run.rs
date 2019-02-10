use crate::event;
use std::future::Future as StdFuture;
use tokio::prelude::*;
use tokio::runtime::Runtime;

async fn map_ok<T: StdFuture>(future: T) -> Result<(), ()> {
    let _ = await!(future);
    Ok(())
}

pub fn run_spdk<F>(future: F)
where
    F: StdFuture<Output = ()> + Send + 'static,
{
    use tokio_async_await::compat::backward;
    let future = backward::Compat::new(map_ok(future));

    let mut rt = Runtime::new().unwrap();
    rt.block_on(future);
    rt.shutdown_now().wait().unwrap();
    event::app_stop(true);
}
