use core::{cmp::min, str::FromStr};
use futures::{future::poll_fn, task::Poll, Future};
use heapless::String;
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

pub struct StringIter<'a, const N: usize> {
    raw: &'a str,
    cursor: usize,
}

impl<'a, const N: usize> From<&'a str> for StringIter<'a, N> {
    fn from(value: &'a str) -> Self {
        Self {
            raw: value,
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
