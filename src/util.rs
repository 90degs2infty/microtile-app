use core::{cmp::min, str::FromStr};
use futures::{task::Poll, Future};
use heapless::String;
use nb::{Error, Result as NbResult};

struct BusyWait<F> {
    f: F,
}

impl<F> BusyWait<F> {
    fn new(f: F) -> Self {
        Self { f }
    }
}

// TODO: am I allowed to implement Unpin?!
impl<F> Unpin for BusyWait<F> {}

// `BusyWait`'s implementation of Future works by busy waiting for the result to
// become ready. I.e. in the poll method, when obtaining a `Error::WouldBlock`
// value, we wake the owning task directly again.
// Note that while this works for RTIC's executor, it may well lead to
// starvation of other tasks when executed on other executors.
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
