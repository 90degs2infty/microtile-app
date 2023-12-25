use core::{cmp::min, pin::Pin, str::FromStr};
use futures::{future::poll_fn, task::Poll, Future};
use heapless::String;
use nb::{Error, Result as NbResult};

pub struct BusyWait<F> {
    f: F,
}

impl<F> BusyWait<F> {
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

// TODO: am I allowed to implement Unpin?!
impl<F> Unpin for BusyWait<F> {}

impl<F, T, E> Future for BusyWait<F>
where
    F: FnMut() -> NbResult<T, E>,
{
    type Output = Result<T, E>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        (&mut self.f)().map_or_else(
            |e| match e {
                Error::WouldBlock => {
                    cx.waker().clone().wake();
                    Poll::Pending
                }
                Error::Other(e) => Poll::Ready(Err(e)),
            },
            |val| Poll::Ready(Ok(val)),
        )
    }
}

pub fn nb_async<F, T, E>(f: F) -> impl Future<Output = Result<T, E>>
where
    F: FnMut() -> NbResult<T, E>,
{
    BusyWait::new(f)
}

pub struct StringIter<'a, const N: usize> {
    raw: &'a str,
    cursor: usize,
}

impl<'a, const N: usize, P> From<&'a P> for StringIter<'a, N>
where
    P: AsRef<str>,
{
    fn from(value: &'a P) -> Self {
        Self {
            raw: value.as_ref(),
            cursor: 0,
        }
    }
}

impl<'a, const N: usize> Iterator for StringIter<'a, N> {
    type Item = Result<String<N>, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor < self.raw.len() {
            let chunk_length = min(N, self.raw.len() - self.cursor);
            let res = Some(String::<N>::from_str(
                &self.raw[self.cursor..(self.cursor + chunk_length)],
            ));
            self.cursor += chunk_length;
            res
        } else {
            None
        }
    }
}
