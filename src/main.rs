extern crate core;

mod github;
mod notify;
mod app_config;

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use chrono::{DateTime, Utc, TimeZone};
use crate::github::github::{fetch_issue_comment, fetch_notification_details, fetch_notifications};
use crate::notify::notify::github_notification;

const INTERVAL_SECONDS: u64 = 60;
const INTERVAL_TO_NEXT_NOTIFICATION_SECONDS: u64 = 12;
const LAST_CHECK_FILE_NAME: &str = "last_check";
const CONFIG_DIR_NAME: &str = ".config/github-notifier";
const API_URL: &str = "https://api.github.com/notifications";


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


#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> io::Result<()> {
    let last_check_time_file = ensure_config_dir()?;
    let saved_since =  load_last_check_time(&last_check_time_file).unwrap_or_else(|_| Utc.timestamp_opt(0, 0).unwrap());;

    let mut since =  to_offset_date_time(saved_since).ok();


    eprintln!("Starting polling Github notifications.");
    eprintln!("Listening for notifications since: {}", since.unwrap());

    let mut tasks = Vec::new();

    loop {
        let new_since = to_offset_date_time(Utc::now())
            .expect("Failed to convert to OffsetDateTime");

        let notifications = fetch_notifications(since);


        let tasks_amount = tasks.len();

        let new_tasks: Vec<_> = notifications.into_iter()
            .map(
                |n| {
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
        //     save_last_check_time(&last_check_time_file, new_since)?;
        // }

        //save_last_check_time(&last_check_time_file, new_since)?;
        std::thread::sleep(Duration::from_secs(INTERVAL_SECONDS));

        //TODO: Original idea was to filter notification posted after last check
        // however as for now I would like not opened notification to be re-sent
        // I think in future this behaviour should be configurable.
        // since = Some(new_since);
    }
}

