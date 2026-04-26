use std::sync::Arc;
use chrono::Utc;
use glib::{DateTime, TimeZone};
use notify_rust::{Hint, Notification, Timeout};
use crate::github::github::{fetch_issue_comment, fetch_notification_details, mark_notification_as_read, CommentDto, NotificationDetailDto, NotificationDto};
use crate::notify::snooze_config_store::SnoozeConfigStore;

#[derive(Debug)]
pub enum NotificationType {
    Mentions,
    PullRequestOpened,
    PullRequestClosed,
    PullRequestMerged,
    IssueOpened,
    IssueClosed,
    IssueAssigned,
    IssueUnassigned,
    IssueLabeled,
    IssueUnlabeled,
    IssueCommented,
    PullRequestReviewRequested,
    PullRequestReviewRequestRemoved,
}

fn open_browser(notification: &NotificationDto, details: &Option<NotificationDetailDto>, comment: &Option<CommentDto>) {

    if details.is_none() {
        eprintln!("Notificattion {} is missing details", notification.id);
        return;
    }

    mark_notification_as_read(&notification.id);

    let url: &str = if let Some(comment) = comment {
        comment.url.as_str()
    } else if let Some(details) = details {
        if let Some(links) = &details.links {
            links.html.href.as_str()
        } else if let Some(html_url) = &details.html_url {
            html_url.as_str()
        } else {
            println!("No URL found to open.");
            return;
        }
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

    let image = match notification.subject.type_field.as_str() {
        "PullRequest" => {
            if let Some(pr) = &details {
                match (pr.state.as_deref(), pr.merged) {
                    (Some("open"), _) => "./assets/pr-open.png",
                    (Some("closed"), true) => "./assets/pr-merged.png",
                    (Some("closed"), false) => "./assets/pr-closed.png",
                    _ => "./assets/github.png",
                }
            } else {
                "./assets/pr-open.png"
            }
        },
        "Issue" => {
            if let Some(issue) = &details {
                match issue.state.as_deref() {
                    Some("open") => "./assets/issue-open.png",
                    Some("closed") => "./assets/issue-closed.png",
                    _ => "./assets/issue-open.png",
                }
            } else {
                "./assets/issue-open.png"
            }
        },
        "Release" => "./assets/release.png",
        "Commit" => "./assets/commit.png",
        "Discussion" => "./assets/discussion.png",
        _ => "./assets/github.png",
    };

    let mut store = Arc::new(SnoozeConfigStore::open_default().unwrap());
    let should_snooze = store.should_snooze_for_reason(
        &notification.repository.owner.login,
        &notification.repository.full_name,
        &notification.reason,
        DateTime::now_utc().unwrap(),
    ).unwrap_or(false);


    if !should_snooze {
        let handle = Notification::new()
            .summary(&notification.repository.full_name)
            .body(&notification.subject.title)
            .id((notification.id.parse::<u64>().unwrap()  % u32::MAX as u64) as u32)
            .timeout(Timeout::Never)
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
            "__closed" => only_mark_as_read(&notification),
            _ => println!("Not matching Action: {} ", action),
        });
    } else {
        only_mark_as_read(&notification)
    }


}