use gtk4::prelude::*;
use gtk4::{self, ListBox, ListBoxRow, ScrolledWindow, Label};
use std::sync::Arc;

use crate::backends::PackageInfo;

pub struct PackageList {
    pub scrolled: ScrolledWindow,
    pub listbox: ListBox,
    packages: Arc<Vec<PackageInfo>>,
}

impl PackageList {
    pub fn new() -> Self {
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .build();
        let listbox = ListBox::builder()
            .build();
        scrolled.set_child(Some(&listbox));
        Self {
            scrolled,
            listbox,
            packages: Arc::new(Vec::new()),
        }
    }

    pub fn set_packages(&mut self, packages: Vec<PackageInfo>) {
        self.packages = Arc::new(packages);
        while let Some(child) = self.listbox.first_child() {
            self.listbox.remove(&child);
        }
        for pkg in self.packages.iter() {
            let row = PackageRow::new(pkg.clone());
            self.listbox.append(&row);
        }
    }

    pub fn get_package(&self, index: usize) -> Option<PackageInfo> {
        self.packages.get(index).cloned()
    }
}

pub struct PackageRow;

impl PackageRow {
    pub fn new(pkg: PackageInfo) -> ListBoxRow {
        let row = ListBoxRow::new();

        let hbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .margin_bottom(6)
            .build();

        let icon = gtk4::Image::from_icon_name("package-x-generic");
        hbox.append(&icon);

        let text_vbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();

        let name_hbox = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .build();

        let name_label = Label::builder()
            .xalign(0.0)
            .css_classes(["title-4"])
            .build();
        name_label.set_markup(&format!("<b>{}</b>", &pkg.name));
        name_hbox.append(&name_label);

        let ver_label = Label::builder()
            .xalign(0.0)
            .label(&pkg.version)
            .css_classes(["caption"])
            .opacity(0.7)
            .build();
        name_hbox.append(&ver_label);

        text_vbox.append(&name_hbox);

        let desc = if pkg.description.chars().count() > 120 {
            format!("{}...", &pkg.description.chars().take(117).collect::<String>())
        } else {
            pkg.description.clone()
        };
        let desc_label = Label::builder()
            .xalign(0.0)
            .label(&desc)
            .wrap(true)
            .max_width_chars(60)
            .build();
        text_vbox.append(&desc_label);

        hbox.append(&text_vbox);

        let badge = Label::builder()
            .label(badge_label(&pkg.backend))
            .css_classes(["badge", "accent"])
            .halign(gtk4::Align::End)
            .build();
        hbox.append(&badge);

        row.set_child(Some(&hbox));
        row
    }
}

fn badge_label(backend: &str) -> &str {
    match backend {
        "pacman" => "Repo",
        "AUR (yay)" | "AUR (paru)" => "AUR",
        "Flatpak" => "Flatpak",
        "Snap" => "Snap",
        "AppImage" => "AppImage",
        _ => backend,
    }
}
