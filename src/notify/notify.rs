use notify_rust::{Hint, Notification};
use crate::github::github::{fetch_issue_comment, fetch_notification_details, mark_notification_as_read, CommentDto, NotificationDetailDto, NotificationDetailLinkHref, NotificationDetailLinks, NotificationDto};

static SEMAPHORE: once_cell::sync::Lazy<tokio::sync::Semaphore> = once_cell::sync::Lazy::new(|| {
    tokio::sync::Semaphore::new(1)
});

fn open_browser(notification: &NotificationDto, details: &Option<NotificationDetailDto>, comment: &Option<CommentDto>) {

    if details.is_none() {
        eprintln!("Notificattion {} is missing details", notification.id);
        return;
    }

    mark_notification_as_read(&notification.id);

    let url: &str = if let Some(comment) = comment {
        comment.url.as_str()
    } else if let Some(details) = details {
        details.links.html.href.as_str()
    } else {
        println!("No URL found to open.");
        return;
    };

    eprintln!("Opening browser for notification  {} at {} ", notification.id, url);
    if let Err(e) = webbrowser::open(url) {
        eprintln!("Failed to open browser: {}", e);
    }
}
fn only_mark_as_read(notification: &NotificationDto) {
    mark_notification_as_read(&notification.id);
}
pub async fn github_notification(notification: NotificationDto) {


    let details = fetch_notification_details(notification.subject.url.as_str());
    let latest_comment =
        match &notification.subject.latest_comment_url {
            Some(comment_url) => {
                fetch_issue_comment(comment_url.as_str())
            }
            None => None
        };

    let image = match(notification.subject.type_field.as_str()) {
        "PullRequest" => "./assets/github-pr.png",
        _ => "./assets/github.png",
    };

    let handle = Notification::new()
        .summary(&notification.repository.full_name)
        .body(&notification.subject.title)
        .icon("github")
        .action("default", "default")
        .action("clicked_a", "✅ Mark as read")
        .action("clicked_b", "🌐 Open in browser")

        .image(image)
        .unwrap()
        .show()
        .unwrap();


    handle.wait_for_action(|action| match action {
        "default" => open_browser(&notification, &details, &latest_comment),
        "clicked_a" => only_mark_as_read(&notification),
        "clicked_b" => open_browser(&notification, &details, &latest_comment),
        "__closed" => println!("the notification was closed"),
        _ => println!("Not matching Action: {} ", action),
    });
}