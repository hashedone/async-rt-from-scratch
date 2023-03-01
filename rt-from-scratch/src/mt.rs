use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::task::{Context, Poll};
use std::time::Duration;

use futures::task::noop_waker;
use futures::{pin_mut, FutureExt};
use scoped_tls::scoped_thread_local;

use crate::timer::sleep;
use crate::yield_now::yield_now;

type Fut = dyn Future<Output = ()> + 'static + Send;

scoped_thread_local!(static SENDERS: Box<[Sender<Pin<Box<Fut>>>]>);
static NEXT_SENDER: AtomicUsize = AtomicUsize::new(0);
static DONE: AtomicBool = AtomicBool::new(true);

const THREADS: usize = 4;

fn spawn<T: 'static + Send>(
    f: impl Future<Output = T> + 'static + Send,
) -> impl Future<Output = T> {
    let (remote, handle) = f.remote_handle();
    SENDERS.with(move |senders| {
        let idx = NEXT_SENDER
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |idx| {
                Some((idx + 1) % senders.len())
            })
            .unwrap();

        senders[idx % senders.len()].send(Box::pin(remote)).ok();
    });

    handle
}

fn execute<T: 'static + Send>(f: impl Future<Output = T> + 'static + Send) -> T {
    DONE.store(false, Ordering::SeqCst);
    NEXT_SENDER.store(1, Ordering::SeqCst);

    let (remote, handle) = f.remote_handle();
    let (sends, recvs): (Vec<_>, Vec<_>) = (0..THREADS).map(|_| channel::<Pin<Box<Fut>>>()).unzip();
    let sends = sends.into_boxed_slice();

    for recv in recvs {
        let sends = sends.clone();

        std::thread::spawn(move || {
            let waker = noop_waker();
            let mut cx = Context::from_waker(&waker);

            SENDERS.set(&sends, || {
                while !DONE.load(Ordering::SeqCst) {
                    let mut fut = match recv.recv() {
                        Ok(fut) => fut,
                        Err(_) => return,
                    };

                    if matches!(fut.as_mut().poll(&mut cx), Poll::Pending) {
                        SENDERS.with(move |senders| {
                            let idx = NEXT_SENDER
                                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |idx| {
                                    Some((idx + 1) % senders.len())
                                })
                                .unwrap();

                            senders[idx % senders.len()].send(fut).ok();
                        });
                    }
                }
            })
        });
    }

    sends[0].send(Box::pin(remote)).ok();
    drop(sends);

    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    pin_mut!(handle);
    loop {
        if let Poll::Ready(v) = handle.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

async fn printn(s: String, n: usize) {
    for i in 0..n {
        println!("{s}: {i}/{n}, TID: {:?}", std::thread::current().id());
        yield_now().await;
    }
}

pub fn main() {
    execute(printn("Next".to_owned(), 10));

    execute(async {
        println!("Waiting...");
        sleep(Duration::from_secs(2)).await;
        println!("Waiting done");
    });

    execute(async {
        let futs: Vec<_> = (0..10)
            .map(|id| spawn(printn(format!("Task{id}"), 10)))
            .collect();

        for f in futs {
            f.await;
        }
    });
}
