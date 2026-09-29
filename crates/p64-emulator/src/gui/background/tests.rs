use super::*;

#[inline(never)]
fn emulation_stack_workload() {
    let mut bytes = [0_u8; 4 * 1024 * 1024];
    std::hint::black_box(&mut bytes)[0] = 7;
    assert_eq!(std::hint::black_box(&bytes)[0], 7);
}

#[test]
fn session_worker_supports_large_emulation_stack_frames() {
    let session = SessionWorker::start(|| {
        emulation_stack_workload();
        Ok(SessionOutcome::GameStopped)
    })
    .unwrap();
    session.worker.join().unwrap();
    assert!(matches!(
        session.result.try_recv(),
        Ok(Ok(SessionOutcome::GameStopped))
    ));
}
