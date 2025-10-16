use std::sync::{Arc, Mutex};
use std::time::Duration;
use chrono::{TimeZone, Utc};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;
use crate::github::github::fetch_notifications;
use crate::{ensure_config_dir, load_last_check_time, save_last_check_time, to_offset_date_time, INTERVAL_SECONDS, INTERVAL_TO_NEXT_NOTIFICATION_SECONDS};
use crate::notify::notify::github_notification;
use crate::notify::snooze_config_store::SnoozeConfigStore;
use crate::notify::tray::GuiMessage;

#[derive(Debug)]
pub struct RepositoryMenuItemData {
    pub id: String,
    pub name: String,
    pub count: String
}

#[derive(Debug)]
pub struct AuthorMenuItemData {
    pub id: String,
    pub name: String,
    pub count: String,
}

#[derive(Debug)]
pub enum NotificationManagerMessage {
    UpdateRepositories(RepositoryMenuItemData),
    UpdateAuthors(RepositoryMenuItemData),

    Quit,
}
#[derive(Debug)]
pub struct NotificationManager {
    cancellation_token: Arc<CancellationToken>,
    pub seen_repositories: Vec<String>,
    pub seen_authors: Vec<String>,
    notification_send: Arc<Mutex<UnboundedSender<NotificationManagerMessage>>>,
    gui_receive: Arc<Mutex<UnboundedReceiver<GuiMessage>>>,
    store: Arc<SnoozeConfigStore>
}


impl NotificationManager {
    pub fn new(
        cancellation_token: Arc<CancellationToken>,
        notification_send: Arc<Mutex<UnboundedSender<NotificationManagerMessage>>>,
        gui_receive: Arc<Mutex<UnboundedReceiver<GuiMessage>>>,
        store: Arc<SnoozeConfigStore>
    ) -> NotificationManager {
        NotificationManager {
            cancellation_token,
            seen_repositories: Vec::new(),
            seen_authors: Vec::new(),
            notification_send,
            gui_receive,
            store
        }
    }

    pub fn run(&mut self) {
        //TODO: Some central config manager?
        let last_check_time_file = ensure_config_dir().unwrap();
        let saved_since =  load_last_check_time(&last_check_time_file).unwrap_or_else(|_| Utc.timestamp_opt(0, 0).unwrap());;

        let mut since =  to_offset_date_time(saved_since).ok();
        let mut tasks = Vec::new();
        loop {
            if self.cancellation_token.is_cancelled(){
                break;
            }
            let new_since = to_offset_date_time(Utc::now())
                .expect("Failed to convert to OffsetDateTime");

            let notifications = fetch_notifications(since);


            let tasks_amount = tasks.len();

            let new_tasks: Vec<_> = notifications.into_iter()
                .map(
                    |n| {
                        self.store.add_repo(&n.repository.owner.login, &n.repository.full_name);
                        let repos = self.store.list_all_repos();
                        std::thread::sleep(Duration::from_secs(INTERVAL_TO_NEXT_NOTIFICATION_SECONDS));
                        tokio::spawn(async move { github_notification(n.clone()).await })
                    }
                )
                .collect();
            tasks.extend(new_tasks);

            tasks.retain(|handle| !handle.is_finished());

            eprintln!("There are {} tasks running.", tasks_amount);
            //Minimize timestamping only to moment when there were actually any notifications present
            // if tasks_amount > 0 {
            //     save_last_check_time(&last_check_time_file, new_since);
            // }

            //save_last_check_time(&last_check_time_file, new_since)?;
            std::thread::sleep(Duration::from_secs(INTERVAL_SECONDS));

            //TODO: Original idea was to filter notification posted after last check
            // however as for now I would like not opened notification to be re-sent
            // I think in future this behaviour should be configurable.
            // since = Some(new_since);
        }
    }
}