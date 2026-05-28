#![allow(deprecated)]

use gtk4::prelude::*;
use gtk4::{
    self, Application, ApplicationWindow, Box, Button, ButtonsType, Dialog, Entry, Expander,
    HeaderBar, Label, ListBox, ListBoxRow, MessageDialog, MessageType, Notebook, Paned,
    PasswordEntry, ProgressBar, ResponseType, ScrolledWindow, SearchEntry, Separator,
    TextBuffer, TextView,
};
use glib::{self, Object as GtkObject};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_std::task;

use crate::backends::BackendManager;
use crate::utils::helpers;
use super::package_list::PackageList;
use super::package_detail::PackageDetail;

#[derive(Clone)]
struct AppState {
    manager: Arc<BackendManager>,
    package_list: Arc<Mutex<PackageList>>,
    detail_view: Arc<PackageDetail>,
    status_label: Label,
    progress_bar: ProgressBar,
    output_buffer: TextBuffer,
    output_expander: Expander,
    notebook: Notebook,
    sidebar_map: Arc<HashMap<usize, String>>,
    current_category: Arc<Mutex<String>>,
}

impl AppState {
    fn set_progress(&self, active: bool) {
        self.progress_bar.set_visible(active);
        if active {
            self.progress_bar.pulse();
        }
    }

    fn append_output(&self, text: &str) {
        let mut end = self.output_buffer.end_iter();
        self.output_buffer.insert(&mut end, text);
        let mut end = self.output_buffer.end_iter();
        self.output_buffer.insert(&mut end, "\n");
        self.output_expander.set_visible(true);
        self.output_expander.set_expanded(true);
    }

    fn clear_output(&self) {
        self.output_buffer.set_text("");
        self.output_expander.set_visible(false);
    }

    fn show_error(&self, title: &str, msg: &str) {
        let dialog = MessageDialog::builder()
            .message_type(MessageType::Error)
            .text(title)
            .secondary_text(msg)
            .buttons(ButtonsType::Ok)
            .modal(true)
            .build();
        dialog.connect_response(|d, _| d.close());
        dialog.present();
    }
}

pub struct AppWindow;

impl AppWindow {
    pub fn new(app: &Application) {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("AcreetionOS AppStore")
            .default_width(1024)
            .default_height(768)
            .build();

        let header = HeaderBar::new();
        let title = Label::builder()
            .label("AcreetionOS AppStore")
            .css_classes(["title-2"])
            .build();
        header.set_title_widget(Some(&title));

        let search_entry = SearchEntry::builder()
            .placeholder_text("Search...")
            .build();
        header.pack_start(&search_entry);

        let refresh_btn = Button::from_icon_name("view-refresh");
        header.pack_end(&refresh_btn);
        window.set_titlebar(Some(&header));

        let main_box = Box::new(gtk4::Orientation::Vertical, 0);
        let paned = Paned::new(gtk4::Orientation::Horizontal);
        let sidebar_sw = ScrolledWindow::builder().width_request(220).build();
        let sidebar_list = ListBox::new();
        sidebar_sw.set_child(Some(&sidebar_list));
        paned.set_start_child(Some(&sidebar_sw));
        paned.set_resize_start_child(false);

        let right_box = Box::new(gtk4::Orientation::Vertical, 0);
        let notebook = Notebook::builder()
            .show_tabs(false)
            .vexpand(true)
            .build();
        let package_list = Arc::new(Mutex::new(PackageList::new()));
        notebook.append_page(
            &package_list.lock().unwrap().scrolled,
            Some(&Label::new(Some("List"))),
        );

        let detail_view = Arc::new(PackageDetail::new());
        notebook.append_page(
            &detail_view.scrolled,
            Some(&Label::new(Some("Detail"))),
        );

        right_box.append(&notebook);
        paned.set_end_child(Some(&right_box));
        main_box.append(&paned);

        let op_box = Box::new(gtk4::Orientation::Vertical, 2);
        let progress_bar = ProgressBar::builder().visible(false).build();
        op_box.append(&progress_bar);

        let output_expander = Expander::builder()
            .label("Output")
            .visible(false)
            .build();
        let out_sw = ScrolledWindow::builder().min_content_height(100).build();
        let output_buffer = TextBuffer::new(None);
        let output_view = TextView::builder()
            .editable(false)
            .buffer(&output_buffer)
            .build();
        out_sw.set_child(Some(&output_view));
        output_expander.set_child(Some(&out_sw));
        op_box.append(&output_expander);
        main_box.append(&op_box);

        let status_bar = Box::new(gtk4::Orientation::Horizontal, 0);
        status_bar.set_css_classes(&["statusbar"]);
        let status_label = Label::new(None);
        status_bar.append(&status_label);
        main_box.append(&status_bar);

        window.set_child(Some(&main_box));

        let state = AppState {
            manager: Arc::new(BackendManager::new()),
            package_list,
            detail_view,
            status_label,
            progress_bar,
            output_buffer,
            output_expander,
            notebook,
            sidebar_map: Arc::new(HashMap::new()),
            current_category: Arc::new(Mutex::new("installed".to_string())),
        };

        Self::populate_sidebar(&sidebar_list, &state);
        Self::connect_signals(
            &sidebar_list,
            &search_entry,
            &refresh_btn,
            state.clone(),
        );

        let s = state.clone();
        glib::spawn_future_local(async move {
            Self::load_packages(s).await;
        });

        window.present();
    }

    fn populate_sidebar(sidebar: &ListBox, state: &AppState) {
        let mut map = state.sidebar_map.as_ref().clone();
        let add_row =
            |sidebar: &ListBox,
             map: &mut HashMap<usize, String>,
             id: &str,
             label: &str,
             icon: &str| {
                let row = ListBoxRow::new();
                let hbox = Box::new(gtk4::Orientation::Horizontal, 8);
                hbox.set_margin_start(8);
                hbox.set_margin_end(8);
                hbox.set_margin_top(6);
                hbox.set_margin_bottom(6);
                hbox.append(&gtk4::Image::from_icon_name(icon));
                hbox.append(&Label::builder().label(label).xalign(0.0).build());
                row.set_child(Some(&hbox));
                map.insert(row.entity_id(), id.to_string());
                sidebar.append(&row);
            };

        add_row(
            sidebar,
            &mut map,
            "installed",
            "Installed",
            "emblem-system",
        );
        add_row(
            sidebar,
            &mut map,
            "updates",
            "Updates Available",
            "software-update-available",
        );
        sidebar.append(&Separator::new(gtk4::Orientation::Horizontal));

        for (key, backend) in &state.manager.backends {
            if backend.enabled() {
                add_row(sidebar, &mut map, key, backend.name(), backend.icon());
            }
        }
    }

    fn connect_signals(
        sidebar: &ListBox,
        search: &SearchEntry,
        refresh: &Button,
        state: AppState,
    ) {
        let s = state.clone();
        sidebar.connect_row_activated(move |_, row| {
            if let Some(id) = s.sidebar_map.get(&row.entity_id()) {
                *s.current_category.lock().unwrap() = id.clone();
                let s_clone = s.clone();
                glib::spawn_future_local(async move {
                    Self::load_packages(s_clone).await;
                });
            }
        });

        let s = state.clone();
        search.connect_search_changed(move |entry| {
            let query = entry.text().to_string();
            let s_clone = s.clone();
            glib::spawn_future_local(async move {
                if !query.is_empty() {
                    Self::show_search_results(s_clone, &query).await;
                } else {
                    Self::load_packages(s_clone).await;
                }
            });
        });

        let s = state.clone();
        refresh.connect_clicked(move |_| {
            let s_clone = s.clone();
            glib::spawn_future_local(async move {
                s_clone.status_label.set_text("Refreshing...");
                let manager = s_clone.manager.clone();
                task::spawn_blocking(move || {
                    manager.refresh_all();
                })
                .await;
                s_clone.status_label.set_text("Refresh complete.");
                Self::load_packages(s_clone).await;
            });
        });

        let s = state.clone();
        state.package_list.lock().unwrap().listbox.connect_row_activated(
            move |_, row| {
                let s_clone = s.clone();
                let idx = row.index();
                if idx >= 0 {
                    if let Some(pkg) =
                        s_clone.package_list.lock().unwrap().get_package(idx as usize)
                    {
                        let manager = s_clone.manager.clone();
                        let detail = s_clone.detail_view.clone();
                        let notebook = s_clone.notebook.clone();
                        let pkg_name = pkg.name.clone();
                        let backend_key = pkg.backend_key.clone();
                        glib::spawn_future_local(async move {
                            let versions = task::spawn_blocking(move || {
                                manager
                                    .backends
                                    .get(&backend_key)
                                    .map(|b| b.get_versions(&pkg_name))
                                    .unwrap_or_default()
                            })
                            .await;
                            detail.show_package(pkg, versions);
                            notebook.set_current_page(Some(1));
                        });
                    }
                }
            },
        );

        let s = state.clone();
        state.detail_view.back_btn.connect_clicked(move |_| {
            s.notebook.set_current_page(Some(0));
        });

        let s = state.clone();
        state.detail_view.install_btn.connect_clicked(move |_| {
            let s_clone = s.clone();
            let pkg_opt = s_clone.detail_view.current_pkg.lock().unwrap().clone();
            if let Some(pkg) = pkg_opt {
                glib::spawn_future_local(Self::request_install(s_clone, pkg));
            }
        });

        let s = state.clone();
        state.detail_view.remove_btn.connect_clicked(move |_| {
            let s_clone = s.clone();
            let pkg_opt = s_clone.detail_view.current_pkg.lock().unwrap().clone();
            if let Some(pkg) = pkg_opt {
                glib::spawn_future_local(Self::request_remove(s_clone, pkg));
            }
        });
    }

    async fn request_install(state: AppState, pkg: crate::backends::PackageInfo) {
        if pkg.backend_key == "appimage" {
            Self::request_appimage_install(state);
            return;
        }

        let needs_sudo = matches!(
            pkg.backend_key.as_str(),
            "pacman" | "flatpak" | "snap"
        );

        if needs_sudo && !helpers::has_sudo_password() {
            Self::run_install_with_password(state, pkg);
        } else {
            Self::run_install(state, pkg).await;
        }
    }

    fn request_appimage_install(state: AppState) {
        let dialog = Dialog::builder()
            .title("Download AppImage")
            .modal(true)
            .build();

        let content = dialog.content_area();
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);

        let label = Label::new(Some("Enter the download URL for the AppImage:"));
        content.append(&label);

        let entry = Entry::builder()
            .placeholder_text("https://example.com/app.AppImage")
            .margin_top(8)
            .build();
        content.append(&entry);

        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Download", ResponseType::Ok);

        let s = state.clone();
        dialog.connect_response(move |d, resp| {
            if resp == ResponseType::Ok {
                let url = entry.text().to_string();
                if !url.is_empty() {
                    let s_clone = s.clone();
                    glib::spawn_future_local(Self::download_appimage_file(s_clone, url));
                }
            }
            d.close();
        });

        dialog.present();
    }

    async fn download_appimage_file(state: AppState, url: String) {
        state.clear_output();
        state.set_progress(true);
        state.append_output(&format!("Downloading AppImage from {}", url));
        state.status_label.set_text("Downloading AppImage...");

        let home = std::env::var("HOME").unwrap_or_default();
        let apps_dir = format!("{}/Applications", home);

        let app_url = url.clone();
        let result = task::spawn_blocking(move || {
            let _ = std::fs::create_dir_all(&apps_dir);

            let filename = std::path::Path::new(&app_url)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("downloaded.AppImage");
            let dest = format!("{}/{}", apps_dir, filename);

            let r = helpers::run_cmd(
                &["curl", "-L", "-o", &dest, "--progress-bar", &app_url],
                600,
            );

            if r.code != 0 {
                let r2 = helpers::run_cmd(
                    &["wget", "-O", &dest, &app_url],
                    600,
                );
                if r2.code != 0 {
                    return (1, String::new(), format!("Download failed: {}", r2.stderr));
                }
            }

            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&dest) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&dest, perms);
            }

            (0, format!("Downloaded {} to {}", filename, dest), String::new())
        }).await;

        state.set_progress(false);

        if result.0 == 0 {
            state.append_output(&result.1);
            state.status_label.set_text("AppImage installed successfully");
        } else {
            state.append_output(&format!("Error: {}", result.2));
            state.status_label.set_text("Failed to download AppImage");
            state.show_error("Download Failed", &result.2);
        }

        let s = state.clone();
        Self::load_packages(s).await;
    }

    fn run_install_with_password(
        state: AppState,
        pkg: crate::backends::PackageInfo,
    ) {
        let dialog = Dialog::builder()
            .title("Authentication Required")
            .modal(true)
            .build();

        let content = dialog.content_area();
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);

        let label = Label::new(Some("Enter your password to install packages:"));
        content.append(&label);

        let entry = PasswordEntry::builder()
            .show_peek_icon(true)
            .margin_top(8)
            .build();
        content.append(&entry);

        dialog.add_button("Cancel", ResponseType::Cancel);
        dialog.add_button("Authenticate", ResponseType::Ok);

        let s = state.clone();
        dialog.connect_response(move |d, resp| {
            if resp == ResponseType::Ok {
                let pw = entry.text().to_string();
                if !pw.is_empty() {
                    helpers::set_sudo_password(pw);
                    let s_clone = s.clone();
                    glib::spawn_future_local(Self::run_install(s_clone, pkg.clone()));
                }
            }
            d.close();
        });

        dialog.present();
    }

    async fn run_install(
        state: AppState,
        pkg: crate::backends::PackageInfo,
    ) {
        state.clear_output();
        state.set_progress(true);
        state.append_output(&format!("Installing {}...", pkg.name));
        state.status_label.set_text(&format!("Installing {}...", pkg.name));

        let manager = state.manager.clone();
        let name = pkg.name.clone();
        let backend_key = pkg.backend_key.clone();
        let version = state.detail_view.selected_version();

        let result = task::spawn_blocking(move || {
            if let Some(backend) = manager.backends.get(&backend_key) {
                backend.install(&name, version.as_deref())
            } else {
                (-1, String::new(), format!("Backend '{}' not found", backend_key))
            }
        }).await;

        state.set_progress(false);

        if result.0 == 0 {
            state.append_output(&format!("Successfully installed {}", pkg.name));
            state.status_label.set_text(&format!("{} installed successfully", pkg.name));
        } else {
            let err = if !result.2.is_empty() {
                result.2.clone()
            } else {
                result.1.clone()
            };
            state.append_output(&format!("Error: {}", err));
            state.status_label.set_text(&format!("Failed to install {}", pkg.name));
            state.show_error("Installation Failed", &err);
        }

        let s = state.clone();
        Self::load_packages(s).await;
    }

    async fn request_remove(state: AppState, pkg: crate::backends::PackageInfo) {
        let confirm = MessageDialog::builder()
            .message_type(MessageType::Question)
            .text("Remove Package")
            .secondary_text(&format!(
                "Are you sure you want to remove {}?",
                pkg.name
            ))
            .buttons(ButtonsType::OkCancel)
            .modal(true)
            .build();

        let s = state.clone();
        let pkg_clone = pkg.clone();
        confirm.connect_response(move |d, resp| {
            d.close();
            if resp == ResponseType::Ok {
                let s_clone = s.clone();
                glib::spawn_future_local(Self::run_remove(s_clone, pkg_clone.clone()));
            }
        });

        confirm.present();
    }

    async fn run_remove(
        state: AppState,
        pkg: crate::backends::PackageInfo,
    ) {
        let needs_sudo = matches!(
            pkg.backend_key.as_str(),
            "pacman" | "flatpak" | "snap"
        );

        if needs_sudo && !helpers::has_sudo_password() {
            let dialog = Dialog::builder()
                .title("Authentication Required")
                .modal(true)
                .build();

            let content = dialog.content_area();
            content.set_margin_start(12);
            content.set_margin_end(12);
            content.set_margin_top(12);
            content.set_margin_bottom(12);

            let label =
                Label::new(Some("Enter your password to remove packages:"));
            content.append(&label);

            let entry = PasswordEntry::builder()
                .show_peek_icon(true)
                .margin_top(8)
                .build();
            content.append(&entry);

            dialog.add_button("Cancel", ResponseType::Cancel);
            dialog.add_button("Authenticate", ResponseType::Ok);

            let s = state.clone();
            dialog.connect_response(move |d, resp| {
                if resp == ResponseType::Ok {
                    let pw = entry.text().to_string();
                    if !pw.is_empty() {
                        helpers::set_sudo_password(pw);
                        let s_clone = s.clone();
                        glib::spawn_future_local(
                            Self::do_remove(s_clone, pkg.clone()),
                        );
                    }
                }
                d.close();
            });

            dialog.present();
        } else {
            Self::do_remove(state, pkg).await;
        }
    }

    async fn do_remove(state: AppState, pkg: crate::backends::PackageInfo) {
        state.clear_output();
        state.set_progress(true);
        state.append_output(&format!("Removing {}...", pkg.name));
        state.status_label.set_text(&format!("Removing {}...", pkg.name));

        let manager = state.manager.clone();
        let name = pkg.name.clone();
        let backend_key = pkg.backend_key.clone();

        let result = task::spawn_blocking(move || {
            if let Some(backend) = manager.backends.get(&backend_key) {
                backend.remove(&name)
            } else {
                (-1, String::new(), format!("Backend '{}' not found", backend_key))
            }
        }).await;

        state.set_progress(false);

        if result.0 == 0 {
            state.append_output(&format!("Successfully removed {}", pkg.name));
            state.status_label.set_text(&format!("{} removed successfully", pkg.name));
        } else {
            let err = if !result.2.is_empty() {
                result.2.clone()
            } else {
                result.1.clone()
            };
            state.append_output(&format!("Error: {}", err));
            state.status_label.set_text(&format!("Failed to remove {}", pkg.name));
            state.show_error("Removal Failed", &err);
        }

        let s = state.clone();
        Self::load_packages(s).await;
    }

    async fn load_packages(state: AppState) {
        state.status_label.set_text("Loading packages...");
        let cat = state.current_category.lock().unwrap().clone();
        let manager = state.manager.clone();

        let results = task::spawn_blocking(move || {
            match cat.as_str() {
                "installed" => manager
                    .enabled_backends()
                    .into_iter()
                    .flat_map(|(_, b)| b.list_installed())
                    .collect(),
                "updates" => manager
                    .enabled_backends()
                    .into_iter()
                    .flat_map(|(_, b)| b.check_updates())
                    .collect(),
                key => manager
                    .backends
                    .get(key)
                    .map(|b| b.list_installed())
                    .unwrap_or_default(),
            }
        })
        .await;

        let count = results.len();
        let msg = format!("{} packages found", count);
        state.package_list.lock().unwrap().set_packages(results);
        state.status_label.set_text(&msg);
    }

    async fn show_search_results(state: AppState, query: &str) {
        state.status_label.set_text(&format!("Searching for '{}'...", query));
        let q = query.to_string();
        let manager = state.manager.clone();

        let results = task::spawn_blocking(move || manager.search_all(&q)).await;

        let count = results.len();
        state.package_list.lock().unwrap().set_packages(results);
        state.notebook.set_current_page(Some(0));
        state.status_label
            .set_text(&format!("Found {} results for '{}'", count, query));
    }
}

trait EntityId {
    fn entity_id(&self) -> usize;
}

impl<T: IsA<GtkObject>> EntityId for T {
    fn entity_id(&self) -> usize {
        self.as_ptr() as usize
    }
}
