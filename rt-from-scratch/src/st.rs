use futures::task::noop_waker;
use futures::{pin_mut, FutureExt};
use scoped_tls::scoped_thread_local;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::timer::sleep;
use crate::yield_now::yield_now;

type Fut = dyn Future<Output = ()> + 'static;

struct Executor {
    tasks: RefCell<Vec<Pin<Box<Fut>>>>,
    spawning: RefCell<Vec<Pin<Box<Fut>>>>,
}

scoped_thread_local!(static EXECUTOR: Executor);

fn spawn<T: 'static>(f: impl Future<Output = T> + 'static) -> impl Future<Output = T> {
    let (remote, handle) = f.remote_handle();

    EXECUTOR.with(move |exec| exec.spawning.borrow_mut().push(Box::pin(remote)));
    handle
}

fn execute<T: 'static>(f: impl Future<Output = T> + 'static) -> T {
    let (remote, handle) = f.remote_handle();
    pin_mut!(handle);

    let tasks = RefCell::new(vec![Box::pin(remote) as Pin<Box<Fut>>]);
    let spawning = Default::default();

    let exec = Executor { tasks, spawning };
    let exec = &exec;

    EXECUTOR.set(exec, move || {
        let mut idx = 0;

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);

        loop {
            let output = &mut exec.tasks.borrow_mut()[idx].as_mut().poll(&mut cx);

            match output {
                Poll::Ready(()) => {
                    let _ = exec.tasks.borrow_mut().swap_remove(idx);
                }
                Poll::Pending => idx += 1,
            }

            exec.tasks
                .borrow_mut()
                .extend(exec.spawning.borrow_mut().drain(..));

            match handle.as_mut().poll(&mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => (),
            }

            idx %= exec.tasks.borrow().len();
        }
    })
}

async fn printn(s: &str, n: usize) {
    for i in 0..n {
        println!("{s}: {i}/{n}");
        yield_now().await;
    }
}

pub fn main() {
    execute(printn("Next", 10));

    execute(async {
        println!("Waiting...");
        sleep(Duration::from_secs(2)).await;
        println!("Waiting done");
    });

    execute(async {
        let f1 = spawn(printn("Foo", 10));
        let f2 = spawn(printn("Bar", 10));

        f1.await;
        f2.await;
    });
}
