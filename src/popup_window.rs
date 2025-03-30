// src/popup_window.rs
use gtk::prelude::*;
use glib;
use tracing::info;
pub struct PopupWindow {
    window: gtk::Window,
    label: gtk::Label,
}

impl PopupWindow {
    pub fn new(tx: std::sync::mpsc::Sender<String>) -> Self {
        gtk::init().expect("Failed to initialize GTK");

        let window = gtk::Window::new(gtk::WindowType::Popup);
        window.set_default_size(300, 100);
        window.set_decorated(false);
        window.set_keep_above(true);
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_data(
            b"
            window.popup-window {
                border-radius: 15px;
                background-color: #f0f0f0;
                border: 1px solid #cccccc;
                box-shadow: 5px 5px 10px rgba(0, 0, 0, 0.3);
            }
            label {
                color: #333333;
                font-family: sans-serif;
                font-size: 14px;
            }
        ",
        );

        gtk::StyleContext::add_provider_for_screen(
             &gtk::prelude::WidgetExt::screen(&window).unwrap(),
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        
        // 给窗口添加CSS类名
        window.style_context().add_class("popup-window");
        let label = gtk::Label::new(None);
        label.set_line_wrap(true);
        label.set_max_width_chars(100);
        label.set_margin(0);

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


        // 添加窗口拖拽功能
        let drag_start_coords: std::rc::Rc<std::cell::RefCell<Option<(f64, f64)>>> = 
            std::rc::Rc::new(std::cell::RefCell::new(None));
        let drag_start_coords_clone = drag_start_coords.clone();
        let window_clone = window.clone();
        
        // 鼠标按下事件：记录起始坐标
        window.connect_button_press_event(move |_, event| {
            if event.button() == 1 { // 左键
                let coords = event.root();
                *drag_start_coords_clone.borrow_mut() = Some(coords);
            };
           glib::Propagation::Proceed
        });
        
        let drag_start_coords_clone2 = drag_start_coords.clone();
        let window_clone2 = window.clone();
        
        // 鼠标移动事件：更新窗口位置
        window.connect_motion_notify_event(move |window, event| {
            if let Ok(mut coords_guard) = drag_start_coords_clone2.try_borrow_mut() {
                if let Some(start_coords) = *coords_guard {
                    let (start_x, start_y) = start_coords;
                    let (current_x, current_y) = event.root();
                    
                    // 计算窗口应该移动到的新位置
                    let new_x = (window.position().0 as f64 + (current_x - start_x)) as i32;
                    let new_y = (window.position().1 as f64 + (current_y - start_y)) as i32;
                    
                    window.move_(new_x, new_y);
                    
                    // 更新起始坐标，为下一次移动做准备
                    *coords_guard = Some((current_x, current_y));
                }
            }
            glib::Propagation::Proceed
        });
        
        let drag_start_coords_clone3 = drag_start_coords.clone();
        
        // 鼠标释放事件：结束拖拽
        window.connect_button_release_event(move |_, _| {
            if let Ok(mut coords_guard) = drag_start_coords_clone3.try_borrow_mut() {
                *coords_guard = None;
            }
            glib::Propagation::Proceed
        });


        window.show_all();
        window.hide();

        let w_clone = window.clone();
        close_button.connect_clicked(move |_| {
            w_clone.hide();
        });

        // 连接复制按钮的点击事件
        let label_clone = label.clone();
        copy_button.connect_clicked(move |_| {
            let text = label_clone.text();
            info!("{}", text);
            tx.send(text.to_string()).expect("Failed to send text");
        });

        Self { window, label }
    }

    pub fn show_at_mouse(&self, text: &str, x: i32, y: i32) {
        self.label.set_text(text);
        let (width, height) = self.window.size_request();
        // 修复GTK调整窗口大小的错误，确保宽度和高度大于0
        let width = if width > 0 { width } else { 300 };
        let height = if height > 0 { height } else { 100 };
        self.window.resize(width, height);
        self.window.move_(x, y);
        self.window.show_all();
    }
}