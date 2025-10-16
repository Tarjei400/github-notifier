use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use glib::DateTime;
use gtk::prelude::SocketExtManual;
use log::info;
use tokio::task::yield_now;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tray_icon::{TrayIconBuilder, TrayIconEvent};
use tokio_util::sync::CancellationToken;
use crate::load_icon;

use tray_icon::menu::{AboutMetadata, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem};
use crate::notify::notify::NotificationType;
use crate::notify::snooze_config_store::SnoozeConfigStore;

pub struct Tray {
    cancellation_token: Arc<CancellationToken>,
    state: Arc<Mutex<TrayState>>,
    gui_recv: Arc<Mutex<UnboundedReceiver<GuiMessage>>>,
    snooze_send: Arc<Mutex<UnboundedSender<SnoozeMessage>>>,
    store: Arc<SnoozeConfigStore>,

}

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
pub enum GuiMessage {
    UpdateRepositories(RepositoryMenuItemData),
    UpdateAuthors(RepositoryMenuItemData),

    Quit,
}

#[derive(Debug)]
pub enum SnoozeMessage {
    SnoozeAuthor(AuthorMenuItemData),
    UnSnoozeAuthor(AuthorMenuItemData),
    SnoozeRepository(RepositoryMenuItemData),
    UnSnoozeRepository(RepositoryMenuItemData),
    ToggleNotificationType(NotificationType),
    ShowMentions,
    ShowSetAsReviewer,
    Quit,
}


pub struct TrayState {

    pub repository_items: HashMap<String, RepositoryMenuItemData>,
    pub author_items: HashMap<String, RepositoryMenuItemData>,
}

impl TrayState {
    fn new() -> Self {
        Self {
            repository_items: HashMap::new(),
            author_items: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.repository_items.clear();
        self.author_items.clear();
    }
}
// helper: build a stable id for each repo action
fn repo_action_id(owner: &str, repo: &str, action: &str) -> MenuId {
    // id format: "repo/<owner>/<repo>/<action>"
    MenuId::new(format!("repo:{}:{}:{}", owner, repo, action))
}

impl Tray {

    pub fn new(
        cancellation_token: Arc<CancellationToken>,
        gui_recv: Arc<Mutex<UnboundedReceiver<GuiMessage>>>,
        snooze_send: Arc<Mutex<UnboundedSender<SnoozeMessage>>>,
        store: Arc<SnoozeConfigStore>
    ) -> Arc<Self> {
        Arc::new(Self {
            cancellation_token,
            gui_recv,
            snooze_send,
            state: Arc::new(Mutex::new(TrayState::new())),
            store

        })
    }
    fn process_messages(self: &Arc<Self>) {
        loop {
            match self.gui_recv.lock().unwrap().try_recv() {
                Ok(msg) => {
                    // Process the message
                    info!("Received Gui message: {:?}", msg);
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    // No messages – yield to let other tasks run
                    info!("No Gui messages:");
                    return;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    // Sender is dropped – exit the loop
                    info!("Gui channel disconnected:");
                    break;
                }
            }
        }
    }
    fn process_menu_clicks(self: &Arc<Self>) {
        loop {
            match TrayIconEvent::receiver().try_recv() {
                Ok(msg) => {
                    // Process the message
                    info!("Received Tray event message: {:?}", msg);
                }
                Err(Error) => {
                    // No messages – yield to let other tasks run
                    info!("Some error eccoured - we ignore it:");
                    return;
                }

            }
        }
    }


    fn build_repositories_menu(self: &Arc<Tray>) -> Submenu {
        let repositories = self.store.list_all_repos().unwrap();
        let sub_menu = Submenu::new("Repositories", true);
        let mut owners: HashMap<String, Submenu> = HashMap::new();

        for (owner, repo_name) in repositories {
            let owner_menu = owners
                .entry(owner.clone())
                .or_insert_with(|| Submenu::new(owner.clone(), true));

            let repo_menu = Submenu::new(repo_name.clone(), true);
            let snooze_menu = Submenu::new("Snooze", true);
            let filter_menu = Submenu::new("Snoose reasons", true);
            let all_reasons = vec![
                "assign", "author", "ci_activity", "comment", "manual", "mention",
                "push", "review_requested", "security_alert", "state_change",
                "subscribed", "team_mention", "your_activity",
            ];

            for reason in all_reasons {
                let reason_id = repo_action_id(&owner, &repo_name, format!("reason:{}", reason).as_str());
                let is_snoozed = self.store.is_repo_snoozed_for_reason(owner.as_str(), repo_name.as_str(), reason);
                let reason_menu_item = CheckMenuItem::with_id(
                    reason_id,
                    reason,
                    true,
                    is_snoozed.unwrap_or(false),
                    None
                );
                filter_menu.append(&reason_menu_item);
            }

            repo_menu.append(&snooze_menu);
            repo_menu.append(&filter_menu);

            // give EACH item a unique id that encodes which repo/action it is
            let snooze_tommorow_id = repo_action_id(&owner, &repo_name, "snooze:day");
            let snooze_week_id = repo_action_id(&owner, &repo_name, "snooze:week");
            let snooze_month_id = repo_action_id(&owner, &repo_name, "snooze:month");

            let snooze_tomorrow = MenuItem::with_id(
                snooze_tommorow_id,
                "For a Day",
                true,
                None
            );
            let snooze_week = MenuItem::with_id(
                snooze_week_id,
                "For a Week",
                true,
                None
            );
            let snooze_month = MenuItem::with_id(
                snooze_month_id,
                "For a Month",
                true,
                None
            );

            snooze_menu.append(&snooze_tomorrow);
            snooze_menu.append(&snooze_week);
            snooze_menu.append(&snooze_month);

            owner_menu.append(&repo_menu);
        }

        for (_, submenu) in owners {
            sub_menu.append(&submenu);
        }
        sub_menu
    }
    fn regenerate_menu(self: &Arc<Self>) -> Box<Menu> {
        let quit_id = "quit";

        let quit_menu_item = MenuItem::with_id(quit_id, "Quit", true, None);
        let menu = Box::new(Menu::new());

        let submenu = self.build_repositories_menu();
        menu.append(&submenu);


        menu.append_items(&[
            &PredefinedMenuItem::about(
                None,
                Some(AboutMetadata {
                    name: Some("Github Notifier2".to_string()),
                    copyright: Some("Copyright Techyon".to_string()),
                    ..Default::default()
                }),
            ),
            &PredefinedMenuItem::separator(),
            &quit_menu_item,
        ]).unwrap();

        menu
    }
    async fn setup_gtk_gui(self: Arc<Self>) {

        let icon = load_icon(std::path::Path::new("./assets/github.png"));
        // let (_, gui_rx) = MainContext::channel(gtk::glib::Priority::default());
        gtk::init().unwrap();

        //self.clone().default_menu_items(&menu2);
        let menu = self.regenerate_menu();

        let _tray_icon2 = Box::new(TrayIconBuilder::new());

        let menu_built = Rc::new(RefCell::new(_tray_icon2

                .with_menu(menu)
                .with_icon(icon.clone())

                .build()
                .unwrap()));



        TrayIconEvent::set_event_handler(Some(move |event| {
            info!("Received tray event: {:?}", event);
        }));

        let moved_self= self.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            info!("Received tray menu event: {:?}", event);
            let id: String = event.id.as_ref().to_owned();

            if id == "quit" {
                moved_self.clone().cancellation_token.cancel();
            }
            if let Some(rest) = id.strip_prefix("repo:") {
                if let Some((owner, rest)) = rest.split_once(':') {
                    if let Some((repo, rest)) = rest.split_once(":") {
                        if let Some((command, action)) = rest.split_once(":") {
                            if command == "snooze" {
                                let mut until = DateTime::now_utc();

                                if (action == "day") {
                                    until = DateTime::now_utc().unwrap().add_days(1);
                                }
                                if (action == "week") {
                                    until = DateTime::now_utc().unwrap().add_days(7);
                                }
                                if (action == "month") {
                                    until = DateTime::now_utc().unwrap().add_days(30);
                                }
                                moved_self.clone().store.snooze_repo(owner, repo, until.unwrap());
                            }
                            if command == "reason" {
                                match moved_self.clone().store.toggle_reason(owner, repo, action) {
                                    Ok(now_enabled) => {
                                        // reflect in UI
                                        let item_id = format!("reason:{}:{}:{}", action, owner, repo);
                                        //let item = app.tray_handle().get_item(&item_id);
                                        // let _ = item.set_selected(now_enabled); // or set_checked() depending on API
                                        println!("toggle_reason: {} for {}/{} id: {}", action, owner, repo, item_id);
                                    }
                                    Err(e) => {
                                        eprintln!("toggle_reason error: {e}");
                                        // optionally show a notification/toast
                                    }
                                }
                            }

                        }
                    }
                }
            }
        }));

        loop {
            if self.cancellation_token.is_cancelled() {
                break;
            }
            self.process_menu_clicks();
            self.process_messages();
            gtk::main_iteration();
        }

    }
    pub fn run(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move { self.setup_gtk_gui().await })
    }
}