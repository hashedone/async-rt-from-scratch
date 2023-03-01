use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

pub struct Timer {
    triggers: Instant,
}

impl Future for Timer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if Instant::now() < self.triggers {
            return Poll::Pending;
        }

        Poll::Ready(())
    }
}

pub async fn sleep(d: Duration) {
    let timer = Timer {
        triggers: Instant::now() + d,
    };

    timer.await
}
