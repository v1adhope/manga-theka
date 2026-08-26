use std::future::Future;

use tokio::task::JoinSet;

pub(super) async fn run_concurrently<T, R, E, F, Fut>(
    items: Vec<T>,
    concurrency: usize,
    op: F,
) -> Result<Vec<R>, E>
where
    T: Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
    F: Fn(T) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = Result<R, E>> + Send + 'static,
{
    let mut slots: Vec<Option<R>> = (0..items.len()).map(|_| None).collect();
    let mut remaining = items.into_iter().enumerate();

    loop {
        let chunk: Vec<(usize, T)> = remaining.by_ref().take(concurrency).collect();
        if chunk.is_empty() {
            break;
        }

        let mut tasks = JoinSet::new();
        for (index, item) in chunk {
            let op = op.clone();
            tasks.spawn(async move { (index, op(item).await) });
        }

        while let Some(joined) = tasks.join_next().await {
            let (index, result) = joined.expect("run_concurrently task panicked");
            slots[index] = Some(result?);
        }
    }

    Ok(slots
        .into_iter()
        .map(|slot| slot.expect("run_concurrently leaves every index filled"))
        .collect())
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use super::run_concurrently;

    #[tokio::test]
    async fn preserves_input_order_regardless_of_completion_order() {
        let items = vec![3u64, 1, 2];

        let result = run_concurrently(items, 5, |n| async move {
            tokio::time::sleep(Duration::from_millis(n * 5)).await;
            Ok::<u64, ()>(n * 10)
        })
        .await
        .unwrap();

        assert_eq!(result, vec![30, 10, 20]);
    }

    #[tokio::test]
    async fn never_runs_more_than_the_given_concurrency_at_once() {
        let items: Vec<u32> = (0..9).collect();
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));

        run_concurrently(items, 3, {
            let active = active.clone();
            let peak = peak.clone();
            move |_| {
                let active = active.clone();
                let peak = peak.clone();
                async move {
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok::<(), ()>(())
                }
            }
        })
        .await
        .unwrap();

        assert!(peak.load(Ordering::SeqCst) <= 3);
    }

    #[tokio::test]
    async fn surfaces_the_error_from_a_failing_item() {
        let items = vec![1u32, 2, 3];

        let result = run_concurrently(items, 5, |n| async move {
            if n == 2 { Err("boom") } else { Ok(n) }
        })
        .await;

        assert_eq!(result, Err("boom"));
    }
}
