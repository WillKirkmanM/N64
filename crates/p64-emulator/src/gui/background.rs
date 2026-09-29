use super::{library::discover_roms, P64Gui};
use crate::rom::RomInfo;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, TryRecvError},
        Arc,
    },
    thread::{Builder, JoinHandle},
    time::Duration,
};

const SESSION_STACK_BYTES: usize = 16 * 1024 * 1024;
const BACKGROUND_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(super) enum DialogKind {
    Folder,
    Rom,
}
pub(super) struct PendingDialog {
    pub kind: DialogKind,
    pub result: Receiver<Option<PathBuf>>,
}
pub(super) struct LibraryScan {
    result: Receiver<Result<HashMap<PathBuf, RomInfo>, String>>,
    cancelled: Arc<AtomicBool>,
}
pub(super) enum SessionOutcome {
    GameStopped,
    ProfileSaved(String),
    Cancelled,
}
pub(super) struct SessionWorker {
    result: Receiver<Result<SessionOutcome, String>>,
    worker: JoinHandle<()>,
}

impl Drop for LibraryScan {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl SessionWorker {
    pub fn start(
        operation: impl FnOnce() -> Result<SessionOutcome, String> + Send + 'static,
    ) -> Result<Self, String> {
        let (tx, result) = mpsc::channel();
        let worker = Builder::new()
            .name("p64-session".to_owned())
            .stack_size(SESSION_STACK_BYTES)
            .spawn(move || {
                let _ = tx.send(operation());
            })
            .map_err(|error| format!("Unable to start session: {error}"))?;
        Ok(Self { result, worker })
    }
}

impl P64Gui {
    pub(super) fn start_session(
        &mut self,
        operation: impl FnOnce() -> Result<SessionOutcome, String> + Send + 'static,
        status: &str,
    ) -> bool {
        match SessionWorker::start(operation) {
            Ok(session) => {
                self.session = Some(session);
                self.status = Some(status.to_owned());
                true
            }
            Err(error) => {
                self.status = Some(error);
                false
            }
        }
    }

    pub(super) fn start_scan(&mut self) {
        self.scan = None;
        let folder = self.library.folder.clone();
        let (tx, result) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancellation = cancelled.clone();
        let worker = Builder::new()
            .name("p64-rom-scan".to_owned())
            .spawn(move || {
                let mut info = HashMap::new();
                let paths = match discover_roms(folder.as_deref()) {
                    Ok(paths) => paths,
                    Err(error) => {
                        let _ = tx.send(Err(error.to_string()));
                        return;
                    }
                };
                for path in paths {
                    if cancellation.load(Ordering::Relaxed) {
                        return;
                    }
                    if let Some(metadata) = RomInfo::new(&path) {
                        info.insert(path, metadata);
                    }
                }
                let _ = tx.send(Ok(info));
            });
        match worker {
            Ok(_) => self.scan = Some(LibraryScan { result, cancelled }),
            Err(error) => self.status = Some(format!("Unable to scan ROM directory: {error}")),
        }
    }

    pub(super) fn poll_background(&mut self, ctx: &eframe::egui::Context) {
        if let Some(dialog) = self.dialog.take() {
            match dialog.result.try_recv() {
                Ok(Some(path)) => match dialog.kind {
                    DialogKind::Folder => {
                        self.library.folder = Some(path);
                        self.start_scan();
                        self.save_config();
                    }
                    DialogKind::Rom => self.launch_rom(path),
                },
                Ok(None) | Err(TryRecvError::Disconnected) => {}
                Err(TryRecvError::Empty) => self.dialog = Some(dialog),
            }
        }
        if let Some(scan) = self.scan.take() {
            match scan.result.try_recv() {
                Ok(Ok(info)) => {
                    self.library.roms = info.keys().cloned().collect();
                    self.library.roms.sort();
                    self.library.info = info;
                }
                Ok(Err(error)) => {
                    self.status = Some(format!("Unable to scan ROM directory: {error}"))
                }
                Err(TryRecvError::Empty) => self.scan = Some(scan),
                Err(TryRecvError::Disconnected) => {
                    self.status = Some("ROM scan stopped unexpectedly".to_owned())
                }
            }
        }
        if let Some(session) = self.session.take() {
            if !session.worker.is_finished() {
                self.session = Some(session);
            } else {
                let result = session.result.try_recv();
                match (session.worker.join(), result) {
                    (Ok(()), Ok(Ok(SessionOutcome::ProfileSaved(profile)))) => {
                        if !self.controllers.profiles.contains(&profile) {
                            self.controllers.profiles.push(profile);
                        }
                        self.status = Some("Input profile saved".to_owned());
                    }
                    (Ok(()), Ok(Ok(SessionOutcome::GameStopped))) => {
                        self.status = Some("Game stopped".to_owned())
                    }
                    (Ok(()), Ok(Ok(SessionOutcome::Cancelled))) => {
                        self.status = Some("Profile configuration cancelled".to_owned())
                    }
                    (_, Ok(Err(error))) => self.status = Some(error),
                    _ => self.status = Some("Session stopped unexpectedly".to_owned()),
                }
            }
        }
        if self.dialog.is_some() || self.scan.is_some() || self.session.is_some() {
            ctx.request_repaint_after(BACKGROUND_POLL_INTERVAL);
        }
    }
}
