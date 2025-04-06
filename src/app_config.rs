use notify_rust::Notification;

pub struct AppConfig {
    pub github_token: String
}

impl AppConfig {
    pub fn load() -> Self {
        let github_token = std::env::var("GITHUB_TOKEN");
        match github_token {
            Ok(github_token) => {
                AppConfig {
                    github_token
                }
            }
            Err(e) => {
                Self::notify_config_issue(&format!("Failed to load GITHUB_TOKEN env variable: {}", e));
                AppConfig {
                    github_token: String::from("")
                }
            }
        }


    }

    fn notify_config_issue(msg: &str) {
        Notification::new()
            .summary("Configuration issue")
            .body(&msg)
            .image("./assets/github-warning.png")
            .unwrap()
            .show();
    }
}