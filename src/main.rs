use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, TrayIconEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    // Su macOS l'event loop DEVE girare sul thread principale
    let event_loop = EventLoop::new().expect("Impossibile creare event loop");

    // --- Icona ---
    // Carica il PNG dell'icona PDF dalla cartella icons/
    // In alternativa puoi usare un'icona di sistema con load_from_name()
    let icon = load_icon();

    // --- Menu ---
    let menu = Menu::new();

    let item_matteo = MenuItem::new("Mostra Matteo", true, None);
    let item_quit   = MenuItem::new("Esci", true, None);

    menu.append(&item_matteo).unwrap();
    menu.append(&item_quit).unwrap();

    // Salva gli ID per riconoscere quale voce è stata cliccata
    let id_matteo = item_matteo.id().clone();
    let id_quit   = item_quit.id().clone();

    // --- Tray icon ---
    // `with_menu` mostra il menu al click sull'icona nella menu bar
    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("AIUsageBar")
        .with_icon(icon)
        .build()
        .expect("Impossibile creare la tray icon");

    // --- Event loop ---
    // Usiamo i receiver cross-thread forniti da tray-icon
    let menu_channel  = MenuEvent::receiver();
    let tray_channel  = TrayIconEvent::receiver();

    event_loop
        .run(move |_event, event_loop_window_target| {
            // Nessuna finestra aperta → l'app vive solo nella menu bar
            event_loop_window_target.set_control_flow(ControlFlow::Wait);

            // Gestisci eventi menu
            if let Ok(menu_event) = menu_channel.try_recv() {
                if menu_event.id == id_matteo {
                    // Click su "Mostra Matteo" → stampa in console
                    // (puoi sostituire con una notifica o una finestra)
                    println!("matteo");
                } else if menu_event.id == id_quit {
                    event_loop_window_target.exit();
                }
            }

            // Gestisci eventi sull'icona stessa (click diretto, non sul menu)
            if let Ok(tray_event) = tray_channel.try_recv() {
                // Click sinistro sull'icona → scrivi "matteo"
                if let tray_icon::TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    ..
                } = tray_event
                {
                    println!("matteo");
                }
            }
        })
        .expect("Errore nell'event loop");
}

/// Carica icons/pdf_icon.png come TrayIcon Icon.
/// Se il file non esiste, genera un'icona rossa 32x32 come fallback.
fn load_icon() -> tray_icon::Icon {
    let icon_path = std::path::Path::new("icons/app_icon.png");

    let (rgba, width, height) = if icon_path.exists() {
        let img = image::open(icon_path)
            .expect("Impossibile aprire icons/pdf_icon.png")
            .into_rgba8();
        let (w, h) = img.dimensions();
        (img.into_raw(), w, h)
    } else {
        // Fallback: quadrato rosso 32x32 (rosso = colore tipico icone PDF)
        eprintln!("⚠️  icons/app_icon.png not found, using placeholder icon.");
        let size = 32u32;
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        for _ in 0..(size * size) {
            pixels.extend_from_slice(&[0xCC, 0x00, 0x00, 0xFF]); // RGBA rosso
        }
        (pixels, size, size)
    };

    tray_icon::Icon::from_rgba(rgba, width, height)
        .expect("Impossibile creare l'icona")
}
