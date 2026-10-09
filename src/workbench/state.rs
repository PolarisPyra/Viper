#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Home,
    Albums,
}

pub(crate) struct WorkbenchState {
    pub(crate) page: Page,
    pub(crate) show_preferences: bool,
    pub(crate) show_album_details: bool,
}

impl WorkbenchState {
    pub(crate) fn new(page: Page) -> Self {
        Self {
            page,
            show_preferences: false,
            show_album_details: false,
        }
    }
}
