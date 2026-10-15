use super::Shutdown;
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::task::{Context, Poll, Wake, Waker};
use std::time::Duration;
use tokio::sync::oneshot;

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

async fn bounded<Output>(future: impl Future<Output = Output>) -> Output {
    tokio::time::timeout(TEST_TIMEOUT, future).await.expect("synthetic shutdown test exceeded its deadline")
}

#[derive(Default)]
struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

struct SyntheticRoot(PathBuf);

impl SyntheticRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("velora-shutdown-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
}

impl Drop for SyntheticRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn closing_blocks_admissions_and_readiness_until_every_permit_drains() {
    let shutdown = Arc::new(Shutdown::default());
    assert!(!shutdown.is_closing());
    assert!(!shutdown.is_ready());
    assert!(!shutdown.mark_ready());
    let first = shutdown.enter().unwrap();
    let second = shutdown.enter().unwrap();

    assert!(shutdown.begin());
    assert!(!shutdown.begin());
    assert!(shutdown.is_closing());
    assert!(matches!(shutdown.enter(), Err("launcher is shutting down")));
    assert!(!shutdown.mark_ready());
    drop(first);
    assert!(!shutdown.mark_ready());
    assert!(!shutdown.is_ready());
    drop(second);

    bounded(shutdown.wait_idle()).await;
    assert!(shutdown.mark_ready());
    assert!(shutdown.is_ready());
    assert!(!shutdown.begin());
    assert!(shutdown.enter().is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn accepted_ordinary_task_completes_without_being_auto_aborted() {
    let shutdown = Arc::new(Shutdown::default());
    let permit = shutdown.enter().unwrap();
    let (started_sender, started) = oneshot::channel();
    let (release_sender, release) = oneshot::channel();
    let task = tokio::spawn(async move {
        let _permit = permit;
        started_sender.send(()).unwrap();
        release.await.unwrap();
        "ordinary work completed"
    });
    bounded(started).await.unwrap();

    assert!(shutdown.begin());
    assert!(!task.is_finished());
    assert!(!shutdown.mark_ready());
    release_sender.send(()).unwrap();
    assert_eq!(bounded(task).await.unwrap(), "ordinary work completed");
    bounded(shutdown.wait_idle()).await;
    assert!(shutdown.mark_ready());
}

#[tokio::test(flavor = "current_thread")]
async fn started_cancellable_task_retains_permit_until_cancellation_is_processed() {
    let shutdown = Arc::new(Shutdown::default());
    let permit = shutdown.enter().unwrap();
    let (started_sender, started) = oneshot::channel();
    let task = tokio::spawn(async move {
        let _permit = permit;
        started_sender.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    shutdown.track_cancellable(task.abort_handle());
    bounded(started).await.unwrap();

    assert!(shutdown.begin());
    assert!(!task.is_finished());
    assert!(!shutdown.mark_ready());
    assert!(bounded(task).await.unwrap_err().is_cancelled());
    bounded(shutdown.wait_idle()).await;
    assert!(shutdown.mark_ready());
}

#[tokio::test(flavor = "current_thread")]
async fn unpolled_cancellable_task_retains_permit_until_abort_is_processed() {
    let shutdown = Arc::new(Shutdown::default());
    let permit = shutdown.enter().unwrap();
    let (polled_sender, mut polled) = oneshot::channel();
    let task = tokio::spawn(async move {
        let _permit = permit;
        polled_sender.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    shutdown.track_cancellable(task.abort_handle());

    assert!(shutdown.begin());
    assert!(!shutdown.begin());
    assert!(!task.is_finished());
    assert!(!shutdown.mark_ready());
    assert!(matches!(polled.try_recv(), Err(oneshot::error::TryRecvError::Empty)));
    assert!(bounded(task).await.unwrap_err().is_cancelled());
    assert!(matches!(polled.try_recv(), Err(oneshot::error::TryRecvError::Closed)));
    bounded(shutdown.wait_idle()).await;
    assert!(shutdown.mark_ready());
}

#[tokio::test(flavor = "current_thread")]
async fn registration_after_begin_aborts_an_already_accepted_task() {
    let shutdown = Arc::new(Shutdown::default());
    let permit = shutdown.enter().unwrap();
    let task = tokio::spawn(async move {
        let _permit = permit;
        std::future::pending::<()>().await;
    });

    assert!(shutdown.begin());
    shutdown.track_cancellable(task.abort_handle());
    assert!(!task.is_finished());
    assert!(!shutdown.mark_ready());
    assert!(bounded(task).await.unwrap_err().is_cancelled());
    bounded(shutdown.wait_idle()).await;
    assert!(shutdown.mark_ready());
}

#[tokio::test(flavor = "current_thread")]
async fn multiple_waiters_recheck_nonfinal_drops_and_all_observe_final_drop() {
    let shutdown = Arc::new(Shutdown::default());
    let first = shutdown.enter().unwrap();
    let second = shutdown.enter().unwrap();
    assert!(shutdown.begin());
    let first_wakes = Arc::new(WakeCounter::default());
    let second_wakes = Arc::new(WakeCounter::default());
    let first_waker = Waker::from(Arc::clone(&first_wakes));
    let second_waker = Waker::from(Arc::clone(&second_wakes));
    let mut first_context = Context::from_waker(&first_waker);
    let mut second_context = Context::from_waker(&second_waker);
    let mut first_waiter = Box::pin(shutdown.wait_idle());
    let mut second_waiter = Box::pin(shutdown.wait_idle());
    assert!(first_waiter.as_mut().poll(&mut first_context).is_pending());
    assert!(second_waiter.as_mut().poll(&mut second_context).is_pending());

    drop(first);
    assert!(first_wakes.0.load(Ordering::SeqCst) > 0);
    assert!(second_wakes.0.load(Ordering::SeqCst) > 0);
    assert!(first_waiter.as_mut().poll(&mut first_context).is_pending());
    assert!(second_waiter.as_mut().poll(&mut second_context).is_pending());
    let first_before = first_wakes.0.load(Ordering::SeqCst);
    let second_before = second_wakes.0.load(Ordering::SeqCst);

    drop(second);
    assert!(first_wakes.0.load(Ordering::SeqCst) > first_before);
    assert!(second_wakes.0.load(Ordering::SeqCst) > second_before);
    assert_eq!(first_waiter.as_mut().poll(&mut first_context), Poll::Ready(()));
    assert_eq!(second_waiter.as_mut().poll(&mut second_context), Poll::Ready(()));
    bounded(shutdown.wait_idle()).await;
    assert!(shutdown.mark_ready());
}

#[tokio::test(flavor = "current_thread")]
async fn final_drop_before_waiter_first_poll_does_not_lose_completion() {
    let shutdown = Arc::new(Shutdown::default());
    let permit = shutdown.enter().unwrap();
    assert!(shutdown.begin());
    let waiter = shutdown.wait_idle();
    drop(permit);
    bounded(waiter).await;
    bounded(shutdown.wait_idle()).await;
    assert!(shutdown.mark_ready());
}

#[tokio::test(flavor = "current_thread")]
async fn blocking_writer_owns_permit_after_awaiting_wrapper_is_cancelled() {
    let root = SyntheticRoot::new();
    let output = root.0.join("synthetic-completion.txt");
    let thread_output = output.clone();
    let shutdown = Arc::new(Shutdown::default());
    let permit = shutdown.enter().unwrap();
    let (started_sender, started) = oneshot::channel();
    let (release_sender, release) = mpsc::channel();
    let (wrapper_sender, wrapper_completion) = oneshot::channel();
    let (done_sender, done) = oneshot::channel();
    let writer = std::thread::spawn(move || {
        let permit = permit;
        let _ = started_sender.send(());
        let result = release
            .recv_timeout(TEST_TIMEOUT)
            .map_err(|error| error.to_string())
            .and_then(|()| std::fs::write(thread_output, b"synthetic write completed").map_err(|error| error.to_string()));
        drop(permit);
        let _ = wrapper_sender.send(());
        let _ = done_sender.send(result);
    });
    let wrapper = tokio::spawn(async move { wrapper_completion.await });
    shutdown.track_cancellable(wrapper.abort_handle());
    bounded(started).await.unwrap();

    assert!(shutdown.begin());
    assert!(bounded(wrapper).await.unwrap_err().is_cancelled());
    assert!(!output.exists());
    assert!(!shutdown.mark_ready());
    let wakes = Arc::new(WakeCounter::default());
    let waker = Waker::from(wakes);
    let mut context = Context::from_waker(&waker);
    let mut waiter = Box::pin(shutdown.wait_idle());
    assert!(waiter.as_mut().poll(&mut context).is_pending());

    release_sender.send(()).unwrap();
    bounded(done).await.unwrap().unwrap();
    writer.join().unwrap();
    bounded(waiter).await;
    assert_eq!(std::fs::read(&output).unwrap(), b"synthetic write completed");
    assert!(shutdown.mark_ready());
}
