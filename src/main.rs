extern crate core;

mod github;
mod notify;
mod app_config;
mod utils;

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use chrono::{DateTime, TimeZone, Utc};
use tokio::sync::mpsc::unbounded_channel;
use tokio_util::sync::CancellationToken;
use crate::github::github::fetch_notifications;
use crate::notify::notify::github_notification;
use notify::tray::{GuiMessage, SnoozeMessage, Tray};
use crate::notify::notification_manager::{NotificationManager, NotificationManagerMessage};
use crate::notify::snooze_config_store::SnoozeConfigStore;

const INTERVAL_SECONDS: u64 = 60;
const INTERVAL_TO_NEXT_NOTIFICATION_SECONDS: u64 = 12;
const LAST_CHECK_FILE_NAME: &str = "last_check";
const CONFIG_DIR_NAME: &str = ".config/github-notifier";
const API_URL: &str = "https://api.github.com/notifications";

const DB_FILE_NAME: &str = "config.db";

fn ensure_config_dir() -> io::Result<PathBuf> {
    let home_dir = dirs::home_dir().expect("Failed to get home directory");
    let config_dir = home_dir.join(CONFIG_DIR_NAME);
    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join(LAST_CHECK_FILE_NAME))
}

fn save_last_check_time(path: &PathBuf, offset_time: time::OffsetDateTime) -> io::Result<()> {
    let timestamp = offset_time.unix_timestamp();
    let t = DateTime::<Utc>::from_utc(chrono::NaiveDateTime::from_timestamp_opt(timestamp, 0).expect("Invalid timestamp"), Utc);

    let mut file = File::create(path)?;
    file.write_all(t.to_rfc3339().as_bytes())?;
    Ok(())
}

fn load_last_check_time(path: &PathBuf) -> io::Result<DateTime<Utc>> {
    let mut file = File::open(path)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;
    let date_time = DateTime::parse_from_rfc3339(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(date_time.with_timezone(&Utc))
}

fn to_offset_date_time(t: DateTime<Utc>) -> Result<time::OffsetDateTime, time::error::ComponentRange> {
    time::OffsetDateTime::from_unix_timestamp(t.timestamp())

}

fn load_icon(path: &std::path::Path) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

enum Message {
    Quit,
    Snooze,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> io::Result<()> {
    let last_check_time_file = ensure_config_dir()?;
    let saved_since =  load_last_check_time(&last_check_time_file).unwrap_or_else(|_| Utc.timestamp_opt(0, 0).unwrap());;



    let mut since =  to_offset_date_time(saved_since).ok();
    

    let (tx, rx) = mpsc::sync_channel::<Message>(2);

    let cancellation_token = Arc::new(CancellationToken::new());

    let (notification_send, mut notification_recv) = unbounded_channel::<NotificationManagerMessage>();
    let (gui_send, mut gui_recv) = unbounded_channel::<GuiMessage>();
    let (snooze_send, mut snooze_recv) = unbounded_channel::<SnoozeMessage>();

    let mt_gui_recv = Arc::new(Mutex::new(gui_recv));
    let mt_snooze_send = Arc::new(Mutex::new(snooze_send));
    let mt_notification_recv = Arc::new(Mutex::new(notification_recv));
    let mt_notification_send = Arc::new(Mutex::new(notification_send));

    let mut store = Arc::new(SnoozeConfigStore::open_default().unwrap());
    // let (gui_tx, gui_rx) = MainContext::channel(gtk::glib::Priority::default());
    let mut notifications_manager = NotificationManager::new(cancellation_token.clone(), mt_notification_send,mt_gui_recv.clone(), store.clone());
    let tray = Tray::new(cancellation_token.clone(), mt_gui_recv, mt_snooze_send, store.clone());
    let trayHandle = tray.run();


    eprintln!("Starting polling Github notifications.");
    eprintln!("Listening for notifications since: {}", since.unwrap());


    gui_send.send(GuiMessage::Quit);
    notifications_manager.run();
    Ok(())



}

