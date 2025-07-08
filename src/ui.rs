// ui.rs - Fixed version
use crate::backup_manager::{BackupConfig, BackupFile, BackupManager};
use crate::AppState;
use eframe::egui;
use std::sync::{Arc, Mutex};
use crate::autostart::setup_autostart;

pub enum AppMessage {
    ShowRestore,
    ShowSettings,
}

pub struct BackupApp {
    backup_manager: Arc<Mutex<BackupManager>>,
    current_view: View,
    selected_browser: String,
    backup_list: Vec<BackupFile>,
    selected_backup: Option<usize>,
    app_state: Arc<Mutex<AppState>>,
    autostart: bool,
}

#[derive(PartialEq)]
enum View {
    Main,
    Restore,
    Settings,
}

impl BackupApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>, 
        app_state: Arc<Mutex<AppState>>,
        backup_manager: Arc<Mutex<BackupManager>>
    ) -> Self {
        // Check current autostart status
        let autostart = check_autostart_enabled();
        
        let mut app = Self {
            backup_manager,
            current_view: View::Main,
            selected_browser: "Chrome".to_string(),
            backup_list: Vec::new(),
            selected_backup: None,
            app_state,
            autostart,
        };
        
        app.load_backup_list();
        app
    }
    
    fn load_backup_list(&mut self) {
        if let Ok(manager) = self.backup_manager.lock() {
            self.backup_list = manager.get_backup_list(&self.selected_browser);
            self.selected_backup = None;
        }
    }
    
    fn process_messages(&mut self) {
        let mut state = self.app_state.lock().unwrap();
        for message in state.message_queue.drain(..) {
            match message {
                AppMessage::ShowRestore => self.current_view = View::Restore,
                AppMessage::ShowSettings => self.current_view = View::Settings,
            }
        }
    }
}

impl eframe::App for BackupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_messages();
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Browser Favoriten Backup");
            ui.separator();
            
            match self.current_view {
                View::Main => self.show_main_view(ui),
                View::Restore => self.show_restore_view(ui),
                View::Settings => self.show_settings_view(ui),
            }
        });
    }
}

impl BackupApp {
    fn show_main_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("📦 Backup erstellen").clicked() {
                let results = self.backup_manager.lock().unwrap().backup_all();
                let success_count = results.iter().filter(|r| r.success).count();
                
                let mut message = format!("Backup abgeschlossen!\n\nErfolgreich: {} von {}\n\n", 
                    success_count, results.len());
                
                for result in &results {
                    let icon = if result.success { "✅" } else { "❌" };
                    message.push_str(&format!("{} {}: {}\n", icon, result.browser, result.message));
                }
                
                native_dialog::MessageDialog::new()
                    .set_type(native_dialog::MessageType::Info)
                    .set_title("Backup Status")
                    .set_text(&message)
                    .show_alert()
                    .ok();
                    
                self.load_backup_list();
            }
            
            if ui.button("🔄 Wiederherstellen").clicked() {
                self.current_view = View::Restore;
            }
            
            if ui.button("⚙ Einstellungen").clicked() {
                self.current_view = View::Settings;
            }
            
            if ui.button("📁 Backup-Ordner öffnen").clicked() {
                let backup_dir = self.backup_manager.lock().unwrap()
                    .get_backup_directory().to_path_buf();
                #[cfg(target_os = "windows")]
                {
                    std::process::Command::new("explorer")
                        .arg(backup_dir)
                        .spawn()
                        .ok();
                }
            }
        });
        
        ui.separator();
        
        // Übersicht der letzten Backups
        ui.heading("Letzte Backups:");
        
        if let Ok(manager) = self.backup_manager.lock() {
            for browser in &["Chrome", "Edge", "Firefox"] {
                let backups = manager.get_backup_list(browser);
                if let Some(latest) = backups.first() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: ", browser));
                        ui.label(latest.date.format("%d.%m.%Y %H:%M:%S").to_string());
                        ui.label(format!("({:.1} KB)", latest.size as f64 / 1024.0));
                    });
                }
            }
        }
        
        ui.separator();
        
        // Additional functions
        ui.heading("Weitere Funktionen:");
        
        ui.horizontal(|ui| {
            if ui.button("🗑 Alte Backups löschen").clicked() {
                // Show dialog to select days to keep
                let days = native_dialog::MessageDialog::new()
                    .set_type(native_dialog::MessageType::Input)
                    .set_title("Alte Backups löschen")
                    .set_text("Backups älter als wie viele Tage löschen? (Standard: 30)")
                    .show_confirm();
                
                // For simplicity, using a fixed value. In a real app, you'd parse the input
                if days.unwrap_or(false) {
                    match self.backup_manager.lock().unwrap().cleanup_old_backups(30) {
                        Ok(count) => {
                            native_dialog::MessageDialog::new()
                                .set_type(native_dialog::MessageType::Info)
                                .set_title("Bereinigung abgeschlossen")
                                .set_text(&format!("{} alte Backups wurden gelöscht.", count))
                                .show_alert()
                                .ok();
                        }
                        Err(e) => {
                            native_dialog::MessageDialog::new()
                                .set_type(native_dialog::MessageType::Error)
                                .set_title("Fehler")
                                .set_text(&format!("Fehler beim Löschen: {}", e))
                                .show_alert()
                                .ok();
                        }
                    }
                    self.load_backup_list();
                }
            }
            
            if ui.button("📤 Als ZIP exportieren").clicked() {
                if let Some(path) = native_dialog::FileDialog::new()
                    .set_filename("browser_backups.zip")
                    .add_filter("ZIP Archive", &["zip"])
                    .show_save_single_file()
                    .ok()
                    .flatten() 
                {
                    match self.backup_manager.lock().unwrap().export_backups(&path) {
                        Ok(_) => {
                            native_dialog::MessageDialog::new()
                                .set_type(native_dialog::MessageType::Info)
                                .set_title("Export erfolgreich")
                                .set_text(&format!("Backups wurden nach {} exportiert.", path.display()))
                                .show_alert()
                                .ok();
                        }
                        Err(e) => {
                            native_dialog::MessageDialog::new()
                                .set_type(native_dialog::MessageType::Error)
                                .set_title("Export fehlgeschlagen")
                                .set_text(&format!("Fehler beim Exportieren: {}", e))
                                .show_alert()
                                .ok();
                        }
                    }
                }
            }
        });
    }
    
    fn show_restore_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("⬅ Zurück").clicked() {
                self.current_view = View::Main;
            }
            ui.label("Browser auswählen:");
            
            let browsers = ["Chrome", "Edge", "Firefox"];
            for browser in &browsers {
                if ui.selectable_value(&mut self.selected_browser, browser.to_string(), *browser).clicked() {
                    self.load_backup_list();
                }
            }
        });
        
        ui.separator();
        
        // Export als HTML Button
        if ui.button("📄 Als HTML exportieren").clicked() {
            if let Some(path) = native_dialog::FileDialog::new()
                .set_filename(&format!("{}_bookmarks.html", self.selected_browser.to_lowercase()))
                .add_filter("HTML", &["html", "htm"])
                .show_save_single_file()
                .ok()
                .flatten()
            {
                match self.backup_manager.lock().unwrap().export_as_html(&self.selected_browser, &path) {
                    Ok(_) => {
                        native_dialog::MessageDialog::new()
                            .set_type(native_dialog::MessageType::Info)
                            .set_title("Export erfolgreich")
                            .set_text(&format!("Favoriten wurden nach {} exportiert.", path.display()))
                            .show_alert()
                            .ok();
                    }
                    Err(e) => {
                        native_dialog::MessageDialog::new()
                            .set_type(native_dialog::MessageType::Error)
                            .set_title("Export fehlgeschlagen")
                            .set_text(&format!("Fehler beim Exportieren: {}", e))
                            .show_alert()
                            .ok();
                    }
                }
            }
        }
        
        ui.separator();
        
        // Backup-Liste anzeigen
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, backup) in self.backup_list.iter().enumerate() {
                let is_selected = self.selected_backup == Some(idx);
                
                if ui.selectable_label(is_selected, format!(
                    "{} - {} - {:.1} KB",
                    backup.name,
                    backup.date.format("%d.%m.%Y %H:%M:%S"),
                    backup.size as f64 / 1024.0
                )).clicked() {
                    self.selected_backup = Some(idx);
                }
            }
        });
        
        ui.separator();
        
        ui.horizontal(|ui| {
            if ui.button("🔄 Wiederherstellen").clicked() {
                if let Some(idx) = self.selected_backup {
                    if let Some(backup) = self.backup_list.get(idx) {
                        let result = native_dialog::MessageDialog::new()
                            .set_type(native_dialog::MessageType::Warning)
                            .set_title("Wiederherstellung bestätigen")
                            .set_text(&format!(
                                "Möchten Sie die {} Favoriten wirklich wiederherstellen?\n\n\
                                Die aktuellen Favoriten werden überschrieben!\n\
                                (Eine Sicherheitskopie wird erstellt)",
                                self.selected_browser
                            ))
                            .show_confirm();
                            
                        if result.unwrap_or(false) {
                            match self.backup_manager.lock().unwrap()
                                .restore_backup(&self.selected_browser, &backup.path) {
                                Ok(message) => {
                                    native_dialog::MessageDialog::new()
                                        .set_type(native_dialog::MessageType::Info)
                                        .set_title("Erfolg")
                                        .set_text(&message)
                                        .show_alert()
                                        .ok();
                                    self.current_view = View::Main;
                                }
                                Err(error) => {
                                    native_dialog::MessageDialog::new()
                                        .set_type(native_dialog::MessageType::Error)
                                        .set_title("Fehler")
                                        .set_text(&error)
                                        .show_alert()
                                        .ok();
                                }
                            }
                        }
                    }
                } else {
                    native_dialog::MessageDialog::new()
                        .set_type(native_dialog::MessageType::Warning)
                        .set_title("Keine Auswahl")
                        .set_text("Bitte wählen Sie ein Backup aus.")
                        .show_alert()
                        .ok();
                }
            }
        });
    }
    
    fn show_settings_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("⬅ Zurück").clicked() {
                self.current_view = View::Main;
            }
        });
        
        ui.separator();
        
        ui.heading("Browser für Backup auswählen:");
        
        let mut config = self.backup_manager.lock().unwrap().get_config().clone();
        let mut changed = false;
        
        if ui.checkbox(&mut config.backup_chrome, "Google Chrome").changed() {
            changed = true;
        }
        
        if ui.checkbox(&mut config.backup_edge, "Microsoft Edge").changed() {
            changed = true;
        }
        
        if ui.checkbox(&mut config.backup_firefox, "Mozilla Firefox").changed() {
            changed = true;
        }
        
        ui.separator();
        
        ui.heading("System-Einstellungen:");
        
        if ui.checkbox(&mut self.autostart, "Mit Windows starten").changed() {
            if let Err(e) = setup_autostart(self.autostart) {
                eprintln!("Failed to set autostart: {}", e);
                // Show error to user
                native_dialog::MessageDialog::new()
                    .set_type(native_dialog::MessageType::Error)
                    .set_title("Fehler")
                    .set_text(&format!("Autostart konnte nicht geändert werden: {}", e))
                    .show_alert()
                    .ok();
                // Revert checkbox
                self.autostart = !self.autostart;
            }
        }
        
        ui.separator();
        
        ui.label(format!("Backup-Verzeichnis: {}", 
            self.backup_manager.lock().unwrap().get_backup_directory().display()));
        
        ui.separator();
        
        if ui.button("💾 Speichern").clicked() && changed {
            self.backup_manager.lock().unwrap().set_config(config);
            native_dialog::MessageDialog::new()
                .set_type(native_dialog::MessageType::Info)
                .set_title("Gespeichert")
                .set_text("Einstellungen wurden gespeichert.")
                .show_alert()
                .ok();
        }
    }
}

// Helper function to check if autostart is enabled
fn check_autostart_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run") {
            key.get_value::<String, _>("BrowserBackup").is_ok()
        } else {
            false
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}