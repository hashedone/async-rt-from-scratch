use std::task::{Context, Poll};
use std::time::Duration;

use futures::pin_mut;
use futures::task::noop_waker;

use crate::timer::sleep;
use crate::yield_now::yield_now;

fn block_on<T>(f: impl std::future::Future<Output = T>) -> T {
    pin_mut!(f);
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

pub fn main() {
    block_on(async {
        for i in 0..10 {
            println!("Next: {i}");
            yield_now().await;
        }
    });

    block_on(async {
        println!("Waiting...");
        sleep(Duration::from_secs(2)).await;
        println!("Waiting done");
    });
}
