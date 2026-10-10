pub(crate) struct PreferencesState {
    pub(crate) category: Option<super::screen::Category>,
    pub(crate) discord_application_id_draft: String,
    #[cfg(target_os = "linux")]
    pub(crate) smb_url_draft: String,
    #[cfg(target_os = "linux")]
    pub(crate) smb_username: String,
    #[cfg(target_os = "linux")]
    pub(crate) smb_password: String,
    #[cfg(target_os = "linux")]
    pub(crate) smb_workgroup: String,
}

impl PreferencesState {
    pub(crate) fn new(discord_application_id_draft: String) -> Self {
        Self {
            category: None,
            discord_application_id_draft,
            #[cfg(target_os = "linux")]
            smb_url_draft: String::new(),
            #[cfg(target_os = "linux")]
            smb_username: String::new(),
            #[cfg(target_os = "linux")]
            smb_password: String::new(),
            #[cfg(target_os = "linux")]
            smb_workgroup: String::new(),
        }
    }
}
