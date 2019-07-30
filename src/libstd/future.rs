//! Asynchronous values.

#![allow(warnings)]
use core::cell::Cell;
use core::marker::Unpin;
use core::pin::Pin;
use core::option::Option;
use core::ptr::NonNull;
use core::task::{Context, Poll};
use core::ops::{Drop, Generator, GeneratorState};

#[doc(inline)]
#[stable(feature = "futures_api", since = "1.36.0")]
pub use core::future::*;

/// Wrap a generator in a future.
///
/// This function returns a `GenFuture` underneath, but hides it in `impl Trait` to give
/// better error messages (`impl Future` rather than `GenFuture<[closure.....]>`).
#[doc(hidden)]
#[unstable(feature = "gen_future", issue = "50547")]
pub fn from_generator<T: Generator<*mut core::task::Context<'static>, Yield = ()>>(x: T) -> impl Future<Output = T::Return> {
    GenFuture(x)
}

/// A wrapper around generators used to implement `Future` for `async`/`await` code.
#[doc(hidden)]
#[unstable(feature = "gen_future", issue = "50547")]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct GenFuture<T: Generator<*mut core::task::Context<'static>, Yield = ()>>(T);

// We rely on the fact that async/await futures are immovable in order to create
// self-referential borrows in the underlying generator.
impl<T: Generator<*mut core::task::Context<'static>, Yield = ()>> !Unpin for GenFuture<T> {}

#[doc(hidden)]
#[unstable(feature = "gen_future", issue = "50547")]
impl<T: Generator<*mut core::task::Context<'static>, Yield = ()>> Future for GenFuture<T> {
    type Output = T::Return;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safe because we're !Unpin + !Drop mapping to a ?Unpin value
        let gen = unsafe { Pin::map_unchecked_mut(self, |s| &mut s.0) };
        // XXX bwb, you removed set_task_context here form the surounding match

        // transmute the context's lifetime to 'static so we can pass it to the next iteration
        // of the generator.
        let cx = unsafe {
            core::mem::transmute::<&mut Context<'_>, &mut Context<'static>>(cx)
        };
        let raw_cx = cx as *mut Context<'_>;
        match gen.resume(raw_cx) {
            GeneratorState::Yielded(()) => Poll::Pending,
            GeneratorState::Complete(x) => Poll::Ready(x),
        }
    }
}

#[doc(hidden)]
#[unstable(feature = "gen_future", issue = "50547")]
/// Polls a future
pub fn poll_with_context<F>(f: Pin<&mut F>, cx: &mut core::task::Context<'_>) -> Poll<F::Output>
where
    F: Future
{
    F::poll(f, cx)
}
