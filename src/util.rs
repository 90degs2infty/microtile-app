use futures::{future::poll_fn, task::Poll, Future};
use nb::{Error, Result as NbResult};

pub fn nb_async<F, T, E>(mut f: F) -> impl Future<Output = Result<T, E>>
where
    F: FnMut() -> NbResult<T, E>,
{
    poll_fn(move |_| {
        f().map_or_else(
            |e| match e {
                Error::WouldBlock => Poll::Pending,
                Error::Other(e) => Poll::Ready(Err(e)),
            },
            |val| Poll::Ready(Ok(val)),
        )
    })
}
