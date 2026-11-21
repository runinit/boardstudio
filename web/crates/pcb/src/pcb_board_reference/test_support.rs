use futures_channel::oneshot;
use std::cell::RefCell;

struct ImportGate {
    entered: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
}

thread_local! {
    static IMPORT_GATE: RefCell<Option<ImportGate>> = const { RefCell::new(None) };
}

pub fn install_import_gate() -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    IMPORT_GATE.with(|gate| {
        *gate.borrow_mut() = Some(ImportGate {
            entered: entered_tx,
            release: release_rx,
        });
    });
    (entered_rx, release_tx)
}

pub async fn pause_import_after_file_read() {
    let gate = IMPORT_GATE.with(|gate| gate.borrow_mut().take());
    if let Some(gate) = gate {
        let _ = gate.entered.send(());
        let _ = gate.release.await;
    }
}
