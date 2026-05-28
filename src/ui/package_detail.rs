use gtk4::prelude::*;
use gtk4::{self, ScrolledWindow, StringList};

use crate::backends::PackageInfo;

pub struct PackageDetail {
    pub scrolled: ScrolledWindow,
    vbox: gtk4::Box,
    pub back_btn: gtk4::Button,
    pub install_btn: gtk4::Button,
    pub remove_btn: gtk4::Button,
    pub version_combo: gtk4::DropDown,
    pub current_pkg: std::sync::Mutex<Option<PackageInfo>>,
}

impl PackageDetail {
    pub fn new() -> Self {
        let scrolled = ScrolledWindow::builder().build();
        let vbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .margin_start(24)
            .margin_end(24)
            .margin_top(24)
            .margin_bottom(24)
            .build();

        let placeholder = gtk4::Label::builder()
            .label("Select a package to view details")
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .opacity(0.5)
            .vexpand(true)
            .build();
        vbox.append(&placeholder);
        scrolled.set_child(Some(&vbox));

        let model = StringList::new(&[]);
        let version_combo = gtk4::DropDown::builder()
            .model(&model)
            .build();

        let install_btn = gtk4::Button::builder()
            .label("Install")
            .css_classes(["suggested-action"])
            .build();
        let remove_btn = gtk4::Button::builder()
            .label("Remove")
            .css_classes(["destructive-action"])
            .build();
        let back_btn = gtk4::Button::builder()
            .label("Back to List")
            .icon_name("go-previous")
            .build();

        Self {
            scrolled,
            vbox,
            back_btn,
            install_btn,
            remove_btn,
            version_combo,
            current_pkg: std::sync::Mutex::new(None),
        }
    }

    pub fn show_package(&self, pkg: PackageInfo, versions: Vec<String>) {
        while let Some(child) = self.vbox.first_child() {
            self.vbox.remove(&child);
        }

        *self.current_pkg.lock().unwrap() = Some(pkg.clone());

        self.vbox.append(&self.back_btn);

        let name_label = gtk4::Label::builder()
            .xalign(0.0)
            .label(&pkg.name)
            .css_classes(["title-1"])
            .build();
        self.vbox.append(&name_label);

        let grid = gtk4::Grid::builder()
            .column_spacing(12)
            .row_spacing(6)
            .margin_top(6)
            .margin_bottom(12)
            .build();

        let installed_str = if pkg.is_installed() { pkg.installed_version.clone() } else { "Not installed".to_string() };
        let repo_str = if pkg.repo.is_empty() { "N/A".to_string() } else { pkg.repo.clone() };
        let size_str = if pkg.size.is_empty() { "N/A".to_string() } else { pkg.size.clone() };
        let license_str = if pkg.license.is_empty() { "N/A".to_string() } else { pkg.license.clone() };

        let fields = [
            ("Version", &pkg.version),
            ("Installed", &installed_str),
            ("Repository", &repo_str),
            ("Backend", &pkg.backend),
            ("Size", &size_str),
            ("License", &license_str),
        ];

        for (i, (key, val)) in fields.iter().enumerate() {
            let k = gtk4::Label::builder()
                .xalign(1.0)
                .label(&format!("{}:", key))
                .css_classes(["dim-label"])
                .build();
            grid.attach(&k, 0, i as i32, 1, 1);
            let v = gtk4::Label::builder()
                .xalign(0.0)
                .label(*val)
                .selectable(true)
                .wrap(true)
                .max_width_chars(60)
                .build();
            grid.attach(&v, 1, i as i32, 1, 1);
        }
        self.vbox.append(&grid);

        if !pkg.description.is_empty() {
            let sep = gtk4::Separator::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .build();
            self.vbox.append(&sep);
            let desc = gtk4::Label::builder()
                .xalign(0.0)
                .label(&pkg.description)
                .wrap(true)
                .max_width_chars(70)
                .build();
            self.vbox.append(&desc);
        }

        let btn_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_top(12)
            .build();

        if !versions.is_empty() {
            let versions_str: Vec<&str> = versions.iter().map(|s| s.as_str()).collect();
            let model = StringList::new(&versions_str);
            self.version_combo.set_model(Some(&model));
            if !versions.is_empty() {
                self.version_combo.set_selected(0);
            }
            let vl = gtk4::Label::builder().label("Version:").build();
            btn_box.append(&vl);
            btn_box.append(&self.version_combo);
        }

        self.install_btn.set_sensitive(!pkg.is_installed());
        self.remove_btn.set_sensitive(pkg.is_installed());
        btn_box.append(&self.install_btn);
        btn_box.append(&self.remove_btn);
        self.vbox.append(&btn_box);
    }

    pub fn selected_version(&self) -> Option<String> {
        let model = self.version_combo.model()?;
        let idx = self.version_combo.selected();
        let string_list = model.downcast::<StringList>().ok()?;
        string_list.string(idx).map(|s| s.to_string())
    }
}
