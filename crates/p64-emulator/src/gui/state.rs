mod controllers;
mod drag;
mod library;
mod settings;

pub(super) use controllers::ControllerState;
pub(super) use drag::DragState;
pub(super) use library::RomLibraryState;
pub(super) use settings::SettingsState;

pub struct P64Gui {
    pub(super) dialog: Option<super::background::PendingDialog>,
    pub(super) scan: Option<super::background::LibraryScan>,
    pub(super) session: Option<super::background::SessionWorker>,
    pub(super) status: Option<String>,
    pub(super) controllers: ControllerState,
    pub(super) library: RomLibraryState,
    pub(super) settings: SettingsState,
    pub(super) drag: DragState,
}
