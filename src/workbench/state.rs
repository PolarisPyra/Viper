#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Home,
    Albums,
}

pub(crate) struct WorkbenchState {
    pub(crate) page: Page,
    pub(crate) show_preferences: bool,
    pub(crate) show_album_details: bool,
    pub(crate) show_queue: bool,
    pub(crate) show_shortcuts: bool,
}

impl WorkbenchState {
    pub(crate) fn new(page: Page) -> Self {
        Self {
            page,
            show_preferences: false,
            show_album_details: false,
            show_queue: false,
            show_shortcuts: false,
        }
    }
}
