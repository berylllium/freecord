use std::rc::Rc;

use smol::LocalExecutor;

async fn async_main(ex: Rc<LocalExecutor<'_>>) {}

fn main() {
    // Create main thread executor.
    let main_thread_executor = Rc::new(LocalExecutor::new());
    let ex = Rc::clone(&main_thread_executor);

    // Spawn the async main function as a new task.
    main_thread_executor
        .spawn(async move {
            async_main(ex).await;
        })
        .detach();

    // Keep trying to poll the main thread executor until no more tasks remain.
    loop {
        if !main_thread_executor.try_tick() && main_thread_executor.is_empty() {
            break;
        }
    }
}
