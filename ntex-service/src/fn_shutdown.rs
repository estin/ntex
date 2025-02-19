use std::cell::Cell;
use std::future::{ready, Ready};
use std::marker::PhantomData;
use std::task::{Context, Poll};

use crate::Service;

#[inline]
/// Create `FnShutdown` for function that can act as a `on_shutdown` callback.
pub fn fn_shutdown<Req, Err, F>(f: F) -> FnShutdown<Req, Err, F>
where
    F: FnOnce(),
{
    FnShutdown::new(f)
}

pub struct FnShutdown<Req, Err, F> {
    f_shutdown: Cell<Option<F>>,
    _t: PhantomData<(Req, Err)>,
}

impl<Req, Err, F> FnShutdown<Req, Err, F> {
    pub(crate) fn new(f: F) -> Self {
        Self {
            f_shutdown: Cell::new(Some(f)),
            _t: PhantomData,
        }
    }
}

impl<Req, Err, F> Clone for FnShutdown<Req, Err, F>
where
    F: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        let f = self.f_shutdown.take();
        self.f_shutdown.set(f.clone());
        Self {
            f_shutdown: Cell::new(f),
            _t: PhantomData,
        }
    }
}

impl<Req, Err, F> Service<Req> for FnShutdown<Req, Err, F>
where
    F: FnOnce(),
{
    type Response = Req;
    type Error = Err;
    type Future<'f> = Ready<Result<Req, Err>> where Self: 'f, Req: 'f;

    #[inline]
    fn poll_shutdown(&self, _: &mut Context<'_>) -> Poll<()> {
        if let Some(f) = self.f_shutdown.take() {
            (f)()
        }
        Poll::Ready(())
    }

    #[inline]
    fn call(&self, req: Req) -> Self::Future<'_> {
        ready(Ok(req))
    }
}

#[cfg(test)]
mod tests {
    use ntex_util::future::lazy;
    use std::{rc::Rc, task::Poll};

    use crate::{fn_service, pipeline};

    use super::*;

    #[ntex::test]
    async fn test_fn_shutdown() {
        let is_called = Rc::new(Cell::new(false));
        let srv = fn_service(|_| async { Ok::<_, ()>("pipe") });
        let is_called2 = is_called.clone();
        let on_shutdown = fn_shutdown(|| {
            is_called2.set(true);
        });

        let pipe = pipeline(srv).and_then(on_shutdown).clone();

        let res = pipe.call(()).await;
        assert_eq!(lazy(|cx| pipe.poll_ready(cx)).await, Poll::Ready(Ok(())));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "pipe");
        assert_eq!(lazy(|cx| pipe.poll_shutdown(cx)).await, Poll::Ready(()));
        assert!(is_called.get());
    }
}
