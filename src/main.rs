mod provider;

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, TrayIconEvent,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

struct App {
    _tray: tray_icon::TrayIcon,
    id_matteo: tray_icon::menu::MenuId,
    id_quit: tray_icon::menu::MenuId,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, _event: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);

        if let Ok(menu_event) = MenuEvent::receiver().try_recv() {
            if menu_event.id == self.id_matteo {
                println!("matteo");
            } else if menu_event.id == self.id_quit {
                event_loop.exit();
            }
        }

        if let Ok(tray_event) = TrayIconEvent::receiver().try_recv() {
            if let tray_icon::TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                ..
            } = tray_event
            {
                println!("matteo");
            }
        }
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    set_accessory_policy();

    let event_loop = EventLoop::new().expect("Impossibile creare event loop");

    let icon = load_icon();

    let menu = Menu::new();
    let item_matteo = MenuItem::new("Mostra Matteo", true, None);
    let item_quit = MenuItem::new("Esci", true, None);
    menu.append(&item_matteo).unwrap();
    menu.append(&item_quit).unwrap();

    let id_matteo = item_matteo.id().clone();
    let id_quit = item_quit.id().clone();

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("AIUsageBar")
        .with_icon(icon)
        .build()
        .expect("Impossibile creare la tray icon");

    let mut app = App { _tray, id_matteo, id_quit };

    event_loop.run_app(&mut app).expect("Errore nell'event loop");
}

#[cfg(target_os = "macos")]
fn set_accessory_policy() {
    use objc2::runtime::AnyClass;
    unsafe {
        let cls = AnyClass::get("NSApplication").unwrap();
        let app: *mut objc2::runtime::AnyObject = objc2::msg_send![cls, sharedApplication];
        let _: bool = objc2::msg_send![app, setActivationPolicy: 1_i64];
    }
}

fn load_icon() -> tray_icon::Icon {
    let icon_path = std::path::Path::new("icons/app_icon.png");

    let (rgba, width, height) = if icon_path.exists() {
        let img = image::open(icon_path)
            .expect("Impossibile aprire icons/app_icon.png")
            .into_rgba8();
        let (w, h) = img.dimensions();
        (img.into_raw(), w, h)
    } else {
        eprintln!("icons/app_icon.png not found, using placeholder icon.");
        let size = 32u32;
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        for _ in 0..(size * size) {
            pixels.extend_from_slice(&[0xCC, 0x00, 0x00, 0xFF]);
        }
        (pixels, size, size)
    };

    tray_icon::Icon::from_rgba(rgba, width, height).expect("Impossibile creare l'icona")
}
