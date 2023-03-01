use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct YieldNow {
    yielded: bool,
}

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            return Poll::Ready(());
        }

        self.yielded = true;

        Poll::Pending
    }
}

pub async fn yield_now() {
    YieldNow { yielded: false }.await
}
