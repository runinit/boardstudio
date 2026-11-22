use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

type Detached = Pin<Box<dyn Future<Output = ()>>>;
thread_local! { static TASKS: RefCell<Vec<Detached>> = RefCell::default(); }
pub fn spawn_local(task: impl Future<Output = ()> + 'static) {
    TASKS.with_borrow_mut(|tasks| tasks.push(Box::pin(task)));
}
pub fn poll_detached() {
    TASKS.with_borrow_mut(|tasks| {
        tasks.retain_mut(|task| {
            task.as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        })
    });
}
#[allow(dead_code)] // Tests without delayed ports only use the detached queue.
pub mod future {
    use super::*;
    pub struct TimeoutFuture(bool);
    impl TimeoutFuture {
        pub fn new(_: u32) -> Self {
            Self(false)
        }
    }
    impl Future for TimeoutFuture {
        type Output = ();
        fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
            if self.0 {
                Poll::Ready(())
            } else {
                self.0 = true;
                Poll::Pending
            }
        }
    }
}
