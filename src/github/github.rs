use notify_rust::Notification;
use ureq::{Body, Error, RequestBuilder};
use ureq::http::Response;
use time;
use crate::app_config::AppConfig;

#[derive(Debug, serde::Deserialize, Clone)]
pub struct NotificationDto {
    pub id: String,
    pub reason: String,
    pub repository: Repository,
    pub subject: Subject,
}


#[derive(Debug, serde::Deserialize, Clone)]
pub struct Repository {
    pub full_name: String,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct Subject {
    pub title: String,
    #[serde(rename = "type")]
    pub type_field: String,

    pub url: String,
    pub latest_comment_url: Option<String>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct NotificationDetailDto {
    #[serde(rename = "_links")]
    pub links: NotificationDetailLinks,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct CommentDto {
    #[serde(rename = "html_url")]
    pub url: String,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct NotificationDetailLinks {
    pub html: NotificationDetailLinkHref,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct NotificationDetailLinkHref {
    pub href: String,
}

fn notify_error(title: &str, msg: String ){
    Notification::new()
        .summary(&title)
        .body(&msg)
        .image("./assets/github-error.png")
        .unwrap()
        .show();
}
fn notify_warning(title: &str, msg: String ){
    Notification::new()
        .summary(&title)
        .body(&msg)
        .image("./assets/github-warning.png")
        .unwrap()
        .show();
}

// This function processes the response received from the API call.
// It tries to deserialize the response body into a desired type `T` (generic and must implement `DeserializeOwned`).
// If the response is successful and contains a JSON body, it parses it into `Vec<Notification>` in this context.
// Otherwise, if there's an error (e.g., HTTP error or deserialization issue), it logs the error and returns the provided `default` value.
fn process_response<T>(res: Result<Response<Body>, Error>, default: T) -> T
where
    T: serde::de::DeserializeOwned,
{
    match res {
        Ok(mut response) => {
            if response.status().is_success() {
                let parsed = response
                    .into_body()
                    .read_json::<T>();

                match parsed {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        notify_error("Parsing error!", format!("There was error while processing the API response: {:?}", e));

                        eprintln!("Error while parsing response body: {}", e);
                        default
                    }
                }
            } else {
                eprintln!("GitHub API returned error: {}", response.status());
                notify_error("GitHub API returned error", format!("There was an error when sending request to Github: {:?}", response.status().to_string()));
                default
            }
        }
        Err(Error::StatusCode(code)) => {
            eprintln!("GitHub returned status: {} ", code);
            notify_error("GitHub API returned error", format!("There was an error when sending request to Github: {:?}", code));
            default
        }
        Err(e) => {
            eprintln!("Request error: {}", e);
            notify_error("GitHub API returned error", format!("There was an error when sending request to Github: {:?}", e));
            default
        }
    }
}

fn prepare_headers<T>(req: RequestBuilder<T>) -> RequestBuilder<T> {
    let config = AppConfig::load();

    req
        .header("Authorization", &format!("token {}", config.github_token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "my-rust-app")

}

pub fn fetch_notifications(since: Option<time::OffsetDateTime>) -> Vec<NotificationDto> {
    let mut request = ureq::get("https://api.github.com/notifications");
    request = prepare_headers(request);

    if let Some(since) = since {
        let since = since.format(&time::format_description::well_known::Rfc3339).unwrap_or_else(|_| String::new());
        request = request.query("since", &since);
    }

    let res = request.call();

    process_response( res, Vec::new())
}

pub fn fetch_notification_details(url: &str) -> Option<NotificationDetailDto> {
    let mut request = ureq::get(url);
    request = prepare_headers(request);

    let res = request.call();
    process_response( res, None)
}

pub fn fetch_issue_comment(url: &str) -> Option<CommentDto> {
    let mut request = ureq::get(url);
    request = prepare_headers(request);

    let res = request.call();
    process_response( res, None)
}

pub fn mark_notification_as_read(notification_id: &String) -> bool {
    let url = format!("https://api.github.com/notifications/threads/{}", notification_id);

    let mut request = ureq::patch(&url);
    request = prepare_headers(request);

    let res = request.send("");

    match res {
        Ok(response) if response.status().is_success() => {
            eprintln!("Marked notificattion  {} as read", notification_id);
            true
        }
        Ok(response) => {
            eprintln!("Failed to mark notification as read. Status: {}", response.status());
            false
        }
        Err(Error::StatusCode(code)) => {
            eprintln!("GitHub returned status: {} ", code);
            false
        }
        Err(e) => {
            eprintln!("Request error: {}", e);
            false
        }
    }
}