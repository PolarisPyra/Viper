use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq)]
struct PresenceKey {
    track_index: usize,
    title: String,
    artist: String,
    album: String,
    paused: bool,
}

pub(crate) struct DiscordPresence {
    client: Option<DiscordIpcClient>,
    application_id: Option<String>,
    published: Option<PresenceKey>,
    retry_after: Instant,
    refresh_after: Instant,
}

impl Default for DiscordPresence {
    fn default() -> Self {
        Self {
            client: None,
            application_id: None,
            published: None,
            retry_after: Instant::now(),
            refresh_after: Instant::now(),
        }
    }
}

impl DiscordPresence {
    pub(crate) fn update(
        &mut self,
        application_id: Option<&str>,
        current: Option<(usize, &str, &str, &str, Option<Duration>)>,
        paused: bool,
        position: Duration,
    ) {
        let application_id = application_id.map(str::trim).filter(|id| !id.is_empty());
        if self.application_id.as_deref() != application_id {
            self.disconnect();
            self.application_id = application_id.map(str::to_owned);
            self.retry_after = Instant::now();
        }

        let Some(application_id) = self.application_id.as_deref() else {
            return;
        };
        let Some((track_index, title, artist, album, duration)) = current else {
            if self.published.take().is_some() {
                if let Some(client) = &mut self.client {
                    if client.clear_activity().is_err() {
                        self.disconnect();
                    }
                }
            }
            return;
        };

        let key = PresenceKey {
            track_index,
            title: title.to_owned(),
            artist: artist.to_owned(),
            album: album.to_owned(),
            paused,
        };
        let now = Instant::now();
        if self.published.as_ref() == Some(&key) && now < self.refresh_after {
            return;
        }
        if now < self.retry_after {
            return;
        }

        if self.client.is_none() {
            let mut client = DiscordIpcClient::new(application_id);
            match client.connect() {
                Ok(()) => self.client = Some(client),
                Err(_) => {
                    self.retry_after = Instant::now() + Duration::from_secs(10);
                    return;
                }
            }
        }

        let state = match (artist.is_empty(), album.is_empty()) {
            (true, true) => "Unknown artist".to_owned(),
            (true, false) => album.to_owned(),
            (false, true) => artist.to_owned(),
            (false, false) => format!("{artist} • {album}"),
        };
        let state = if paused {
            format!("Paused · {state}")
        } else {
            state
        };
        let mut activity = activity::Activity::new()
            .name("Viper")
            .activity_type(activity::ActivityType::Listening)
            .status_display_type(activity::StatusDisplayType::Details)
            .details(title)
            .state(state);
        if !paused {
            if let Some(duration) = duration {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |time| time.as_secs());
                let start = i64::try_from(now)
                    .unwrap_or(i64::MAX)
                    .saturating_sub(i64::try_from(position.as_secs()).unwrap_or(i64::MAX));
                if let Ok(duration) = i64::try_from(duration.as_secs()) {
                    if let Some(end) = start.checked_add(duration) {
                        activity =
                            activity.timestamps(activity::Timestamps::new().start(start).end(end));
                    }
                }
            }
        }

        match self
            .client
            .as_mut()
            .map(|client| client.set_activity(activity))
        {
            Some(Ok(())) => {
                self.published = Some(key);
                self.refresh_after = Instant::now() + Duration::from_secs(60);
            }
            _ => {
                self.disconnect();
                self.retry_after = Instant::now() + Duration::from_secs(10);
            }
        }
    }

    fn disconnect(&mut self) {
        if let Some(mut client) = self.client.take() {
            let _ = client.clear_activity();
            let _ = client.close();
        }
        self.published = None;
    }
}

impl Drop for DiscordPresence {
    fn drop(&mut self) {
        self.disconnect();
    }
}
