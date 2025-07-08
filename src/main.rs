// main.rs
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIcon, TrayIconBuilder,
};

mod backup_manager;
mod ui;

use backup_manager::BackupManager;
use ui::{BackupApp, AppMessage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Shared state zwischen Tray und GUI
    let app_state = Arc::new(Mutex::new(AppState::default()));
    let app_state_tray = app_state.clone();
    
    // Tray Icon in separatem Thread
    thread::spawn(move || {
        if let Err(e) = run_tray(app_state_tray) {
            eprintln!("Tray error: {}", e);
        }
    });
    
    // GUI starten
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 500.0])
            .with_min_inner_size([600.0, 400.0])
            .with_icon(create_icon()),
        ..Default::default()
    };
    
    eframe::run_native(
        "Browser Favoriten Backup",
        options,
        Box::new(move |cc| {
            Box::new(BackupApp::new(cc, app_state.clone()))
        }),
    )?;
    
    Ok(())
}

#[derive(Default)]
struct AppState {
    show_window: bool,
    message_queue: Vec<AppMessage>,
}

fn run_tray(app_state: Arc<Mutex<AppState>>) -> Result<(), Box<dyn std::error::Error>> {
    let menu = Menu::new();
    let backup_now = MenuItem::new("Backup jetzt erstellen", true, None);
    let restore = MenuItem::new("Wiederherstellen...", true, None);
    let settings = MenuItem::new("Einstellungen", true, None);
    let open_folder = MenuItem::new("Backup-Ordner öffnen", true, None);
    let quit = MenuItem::new("Beenden", true, None);
    
    menu.append(&backup_now)?;
    menu.append(&restore)?;
    menu.append(&MenuItem::separator())?;
    menu.append(&settings)?;
    menu.append(&open_folder)?;
    menu.append(&MenuItem::separator())?;
    menu.append(&quit)?;
    
    let icon = create_tray_icon_image();
    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Browser Favoriten Backup")
        .with_icon(icon)
        .build()?;
    
    let menu_channel = MenuEvent::receiver();
    let backup_manager = BackupManager::new();
    
    loop {
        if let Ok(event) = menu_channel.recv() {
            match event.id {
                id if id == backup_now.id() => {
                    let results = backup_manager.backup_all();
                    // Notification anzeigen
                    let success_count = results.iter().filter(|r| r.success).count();
                    let message = format!(
                        "Backup abgeschlossen!\nErfolgreich: {} von {}",
                        success_count, results.len()
                    );
                    
                    #[cfg(target_os = "windows")]
                    {
                        use winapi::um::winuser::{MessageBoxW, MB_OK, MB_ICONINFORMATION};
                        use std::ptr;
                        unsafe {
                            let title: Vec<u16> = "Backup Status\0".encode_utf16().collect();
                            let msg: Vec<u16> = format!("{}\0", message).encode_utf16().collect();
                            MessageBoxW(ptr::null_mut(), msg.as_ptr(), title.as_ptr(), MB_OK | MB_ICONINFORMATION);
                        }
                    }
                }
                id if id == restore.id() => {
                    let mut state = app_state.lock().unwrap();
                    state.show_window = true;
                    state.message_queue.push(AppMessage::ShowRestore);
                }
                id if id == settings.id() => {
                    let mut state = app_state.lock().unwrap();
                    state.show_window = true;
                    state.message_queue.push(AppMessage::ShowSettings);
                }
                id if id == open_folder.id() => {
                    let backup_dir = backup_manager.get_backup_directory();
                    #[cfg(target_os = "windows")]
                    {
                        std::process::Command::new("explorer")
                            .arg(&backup_dir)
                            .spawn()
                            .ok();
                    }
                }
                id if id == quit.id() => {
                    break;
                }
                _ => {}
            }
        }
    }
    
    Ok(())
}

fn create_icon() -> eframe::IconData {
    let size = 32;
    let mut pixels = vec![0u8; size * size * 4];
    
    // Einfaches Icon erstellen (blauer Ordner mit Pfeil)
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            
            // Ordner-Form
            if (x >= 4 && x < 28 && y >= 8 && y < 26) {
                pixels[idx] = 33;     // R
                pixels[idx + 1] = 150; // G
                pixels[idx + 2] = 243; // B
                pixels[idx + 3] = 255; // A
            }
            
            // Pfeil
            if ((x >= 14 && x < 18 && y >= 4 && y < 16) ||
                (x >= 10 && x < 22 && y >= 12 && y < 16 && (x < 14 || x >= 18))) {
                pixels[idx] = 76;     // R
                pixels[idx + 1] = 175; // G
                pixels[idx + 2] = 80;  // B
                pixels[idx + 3] = 255; // A
            }
        }
    }
    
    eframe::IconData {
        rgba: pixels,
        width: size,
        height: size,
    }
}

fn create_tray_icon_image() -> tray_icon::Icon {
    let icon_bytes = include_bytes!("../icon.ico");
    tray_icon::Icon::from_resource(icon_bytes, None).unwrap_or_else(|_| {
        // Fallback: Erstelle ein einfaches Icon
        let pixels = vec![255u8; 16 * 16 * 4];
        tray_icon::Icon::from_rgba(pixels, 16, 16).expect("Failed to create icon")
    })
}

#[cfg(target_os = "windows")]
fn setup_autostart(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    use winreg::enums::*;
    use winreg::RegKey;
    
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let (key, _) = hkcu.create_subkey(path)?;
    
    if enable {
        let exe_path = std::env::current_exe()?;
        key.set_value("BrowserBackup", &exe_path.to_string_lossy().as_ref())?;
    } else {
        key.delete_value("BrowserBackup").ok();
    }
    
    Ok(())
}
