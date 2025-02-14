// src/popup_window.rs
use gtk::{prelude::*, Window};

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

        // 创建垂直布局容器
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
        vbox.set_margin(10);
        // 创建水平按钮布局
        let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        button_box.set_halign(gtk::Align::End);

        // 创建复制按钮
        let copy_button = gtk::Button::with_label("复制");
        copy_button.set_size_request(80, -1);

        // 创建关闭按钮
        let close_button = gtk::Button::with_label("关闭");
        close_button.set_size_request(80, -1);

        // 将按钮添加到按钮盒中
        button_box.add(&copy_button);
        button_box.add(&close_button);

        // 将组件添加到主布局中
        vbox.add(&label);
        vbox.pack_end(&button_box, false, false, 0);

        window.add(&vbox);

        window.show_all();
        window.hide();

        let w_clone = window.clone();
        close_button.connect_clicked(move |_| {
            w_clone.hide();
        });

        //     // 连接复制按钮的点击事件
        //     let label_clone = label.clone();
        //     copy_button.connect_clicked(move |_| {
        //         let text = label_clone.text();
        //         Self::copy_to_clipboard(&text);
        //     });

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
