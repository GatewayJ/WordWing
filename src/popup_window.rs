// src/popup_window.rs
use gtk::prelude::*;

pub struct PopupWindow {
    window: gtk::Window,
    label: gtk::Label,
}

impl PopupWindow {
    pub fn new() -> Self {
        gtk::init().expect("Failed to initialize GTK");

        let window = gtk::Window::new(gtk::WindowType::Popup);
        window.set_default_size(300, 100);
        window.set_decorated(false);
        window.set_keep_above(true);

        let label = gtk::Label::new(None);
        label.set_line_wrap(true);
        label.set_max_width_chars(40);
        label.set_margin(10);
        
        window.add(&label);
        window.show_all();
        window.hide(); // 初始隐藏

        Self { window, label }
    }

    pub fn show_at_mouse(&self, text: &str, x: i32, y: i32) {
        self.label.set_text(text);
        self.window.move_(x, y);
        self.window.show_all();
    }

    pub fn hide(&self) {
        self.window.hide();
    }
}