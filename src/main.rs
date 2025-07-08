// main.rs - Fixed version
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIcon, TrayIconBuilder,
};

mod backup_manager;
mod ui;
mod autostart;

use backup_manager::BackupManager;
use ui::{BackupApp, AppMessage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Shared state zwischen Tray und GUI
    let app_state = Arc::new(Mutex::new(AppState::default()));
    let app_state_tray = app_state.clone();
    
    // Shared BackupManager instance
    let backup_manager = Arc::new(Mutex::new(BackupManager::new()));
    let backup_manager_tray = backup_manager.clone();
    
    // Start scheduled backups
    BackupManager::start_scheduled_backups(backup_manager.clone(), 24);
    
    // Tray Icon in separatem Thread
    thread::spawn(move || {
        if let Err(e) = run_tray(app_state_tray, backup_manager_tray) {
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
            Box::new(BackupApp::new(cc, app_state.clone(), backup_manager.clone()))
        }),
    )?;
    
    Ok(())
}

#[derive(Default)]
struct AppState {
    show_window: bool,
    message_queue: Vec<AppMessage>,
}

fn run_tray(app_state: Arc<Mutex<AppState>>, backup_manager: Arc<Mutex<BackupManager>>) -> Result<(), Box<dyn std::error::Error>> {
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
    let tray = match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Browser Favoriten Backup")
        .with_icon(icon)
        .build() {
        Ok(tray) => tray,
        Err(e) => {
            eprintln!("Failed to create tray icon: {}", e);
            return Err(Box::new(e));
        }
    };
    
    let menu_channel = MenuEvent::receiver();
    
    loop {
        if let Ok(event) = menu_channel.recv() {
            match event.id {
                id if id == backup_now.id() => {
                    let results = backup_manager.lock().unwrap().backup_all();
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
                    let backup_dir = backup_manager.lock().unwrap().get_backup_directory().to_path_buf();
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
    // Try to load from embedded resource first
    #[cfg(target_os = "windows")]
    {
        if let Ok(icon) = tray_icon::Icon::from_resource(1, None) {
            return icon;
        }
    }
    
    // Fallback: Create a simple icon programmatically
    let size = 16;
    let mut pixels = vec![255u8; size * size * 4];
    
    // Create a simple backup icon (folder with arrow)
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            
            // Blue folder shape
            if (x >= 2 && x < 14 && y >= 4 && y < 13) {
                pixels[idx] = 33;     // R
                pixels[idx + 1] = 150; // G
                pixels[idx + 2] = 243; // B
                pixels[idx + 3] = 255; // A
            }
            
            // Green arrow pointing up
            if ((x >= 7 && x < 9 && y >= 2 && y < 8) ||
                (x >= 5 && x < 11 && y >= 6 && y < 8 && (x < 7 || x >= 9))) {
                pixels[idx] = 76;     // R
                pixels[idx + 1] = 175; // G
                pixels[idx + 2] = 80;  // B
                pixels[idx + 3] = 255; // A
            }
        }
    }
    
    tray_icon::Icon::from_rgba(pixels, size as u32, size as u32)
        .expect("Failed to create icon")
}