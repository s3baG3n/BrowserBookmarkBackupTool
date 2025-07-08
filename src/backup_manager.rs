// backup_manager.rs
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct BackupResult {
    pub browser: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct BackupFile {
    pub name: String,
    pub path: PathBuf,
    pub date: chrono::DateTime<Local>,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BackupConfig {
    pub backup_chrome: bool,
    pub backup_edge: bool,
    pub backup_firefox: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            backup_chrome: true,
            backup_edge: true,
            backup_firefox: true,
        }
    }
}

pub struct BackupManager {
    backup_dir: PathBuf,
    config: BackupConfig,
}

impl BackupManager {
    pub fn new() -> Self {
        let mut manager = Self {
            backup_dir: Self::get_default_backup_dir(),
            config: BackupConfig::default(),
        };
        
        manager.ensure_backup_dir();
        manager.load_config();
        manager
    }
    
    fn get_default_backup_dir() -> PathBuf {
        let user_profile = std::env::var("USERPROFILE")
            .unwrap_or_else(|_| dirs::home_dir().unwrap().to_string_lossy().to_string());
            PathBuf::from(user_profile)
            .join("Work Folders")
            .join("Benutzerdatensicherung")
            .join("Bookmarks")
    }
    
    fn ensure_backup_dir(&self) {
        if !self.backup_dir.exists() {
            fs::create_dir_all(&self.backup_dir).ok();
        }
    }
    
    fn load_config(&mut self) {
        let config_file = self.backup_dir.join("config.json");
        if config_file.exists() {
            if let Ok(content) = fs::read_to_string(&config_file) {
                if let Ok(config) = serde_json::from_str(&content) {
                    self.config = config;
                }
            }
        }
    }
    
    pub fn save_config(&self) {
        let config_file = self.backup_dir.join("config.json");
        if let Ok(content) = serde_json::to_string_pretty(&self.config) {
            fs::write(config_file, content).ok();
        }
    }
    
    pub fn get_config(&self) -> &BackupConfig {
        &self.config
    }
    
    pub fn set_config(&mut self, config: BackupConfig) {
        self.config = config;
        self.save_config();
    }
    
    pub fn get_backup_directory(&self) -> &Path {
        &self.backup_dir
    }
    
    pub fn backup_all(&self) -> Vec<BackupResult> {
        let mut results = Vec::new();
        
        if self.config.backup_chrome {
            results.push(self.backup_chrome());
        }
        
        if self.config.backup_edge {
            results.push(self.backup_edge());
        }
        
        if self.config.backup_firefox {
            results.push(self.backup_firefox());
        }
        
        results
    }
    
    fn backup_chrome(&self) -> BackupResult {
        let user_profile = std::env::var("USERPROFILE").unwrap_or_default();
        let bookmarks_path = PathBuf::from(user_profile)
            .join("AppData")
            .join("Local")
            .join("Google")
            .join("Chrome")
            .join("User Data")
            .join("Default")
            .join("Bookmarks");
        
        self.backup_browser_file("Chrome", &bookmarks_path, "json")
    }
    
    fn backup_edge(&self) -> BackupResult {
        let user_profile = std::env::var("USERPROFILE").unwrap_or_default();
        let bookmarks_path = PathBuf::from(user_profile)
            .join("AppData")
            .join("Local")
            .join("Microsoft")
            .join("Edge")
            .join("User Data")
            .join("Default")
            .join("Bookmarks");
        
        self.backup_browser_file("Edge", &bookmarks_path, "json")
    }
    
    fn backup_firefox(&self) -> BackupResult {
        let user_profile = std::env::var("USERPROFILE").unwrap_or_default();
        let profiles_path = PathBuf::from(user_profile)
            .join("AppData")
            .join("Roaming")
            .join("Mozilla")
            .join("Firefox")
            .join("Profiles");
        
        if let Ok(entries) = fs::read_dir(&profiles_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.to_string_lossy().ends_with(".default-release") {
                    let places_db = path.join("places.sqlite");
                    return self.backup_browser_file("Firefox", &places_db, "sqlite");
                }
            }
        }
        
        BackupResult {
            browser: "Firefox".to_string(),
            success: false,
            message: "Firefox Profil nicht gefunden".to_string(),
        }
    }
    
    fn backup_browser_file(&self, browser: &str, source_path: &Path, extension: &str) -> BackupResult {
        if !source_path.exists() {
            return BackupResult {
                browser: browser.to_string(),
                success: false,
                message: "Favoriten nicht gefunden".to_string(),
            };
        }
        
        let browser_backup_dir = self.backup_dir.join(browser);
        if !browser_backup_dir.exists() {
            fs::create_dir_all(&browser_backup_dir).ok();
        }
        
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_filename = format!("bookmarks_{}.{}", timestamp, extension);
        let backup_path = browser_backup_dir.join(&backup_filename);
        
        match fs::copy(source_path, &backup_path) {
            Ok(_) => BackupResult {
                browser: browser.to_string(),
                success: true,
                message: format!("Gesichert: {}", backup_filename),
            },
            Err(e) => BackupResult {
                browser: browser.to_string(),
                success: false,
                message: format!("Fehler: {}", e),
            },
        }
    }
    
    pub fn get_backup_list(&self, browser: &str) -> Vec<BackupFile> {
        let browser_dir = self.backup_dir.join(browser);
        let mut backups = Vec::new();
        
        if let Ok(entries) = fs::read_dir(&browser_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            let datetime: chrono::DateTime<Local> = modified.into();
                            backups.push(BackupFile {
                                name: entry.file_name().to_string_lossy().to_string(),
                                path,
                                date: datetime,
                                size: metadata.len(),
                            });
                        }
                    }
                }
            }
        }
        
        backups.sort_by(|a, b| b.date.cmp(&a.date));
        backups
    }
    
    pub fn restore_backup(&self, browser: &str, backup_path: &Path) -> Result<String, String> {
        let user_profile = std::env::var("USERPROFILE").unwrap_or_default();
        
        let target_path = match browser {
            "Chrome" => PathBuf::from(&user_profile)
                .join("AppData/Local/Google/Chrome/User Data/Default/Bookmarks"),
            "Edge" => PathBuf::from(&user_profile)
                .join("AppData/Local/Microsoft/Edge/User Data/Default/Bookmarks"),
            "Firefox" => {
                let profiles_path = PathBuf::from(&user_profile)
                    .join("AppData/Roaming/Mozilla/Firefox/Profiles");
                
                if let Ok(entries) = fs::read_dir(&profiles_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.to_string_lossy().ends_with(".default-release") {
                            return Ok(path.join("places.sqlite").to_string_lossy().to_string());
                        }
                    }
                }
                return Err("Firefox Profil nicht gefunden".to_string());
            }
            _ => return Err("Unbekannter Browser".to_string()),
        };
        
        // Backup der aktuellen Datei
        if target_path.exists() {
            let backup_current = target_path.with_extension("bak");
            fs::copy(&target_path, backup_current)
                .map_err(|e| format!("Fehler beim Sichern der aktuellen Datei: {}", e))?;
        }
        
        // Wiederherstellen
        fs::copy(backup_path, &target_path)
            .map_err(|e| format!("Fehler beim Wiederherstellen: {}", e))?;
        
        let mut message = format!("{} Favoriten erfolgreich wiederhergestellt", browser);
        if browser == "Firefox" {
            message.push_str("\n(Firefox muss neu gestartet werden)");
        }
        
        Ok(message)
    }

    // Automatisches Backup nach Zeitplan
   pub fn schedule_backup(manager: Arc<Mutex<Self>>, interval_hours: u64) {
        use std::thread;
        use std::time::Duration;
        
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(interval_hours * 3600));
                
                let results = manager.lock().unwrap().backup_all();
                
                // Log oder Notification
                println!("Automatisches Backup durchgeführt: {:?}", results);
            }
        });
    }
    
    // Alte Backups automatisch löschen
    pub fn cleanup_old_backups(&self, keep_days: i64) -> Result<usize, String> {
        let mut deleted_count = 0;
        let cutoff_date = Local::now() - chrono::Duration::days(keep_days);
        
        for browser in &["Chrome", "Edge", "Firefox"] {
            let browser_dir = self.backup_dir.join(browser);
            if let Ok(entries) = fs::read_dir(&browser_dir) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            let datetime: chrono::DateTime<Local> = modified.into();
                            if datetime < cutoff_date {
                                if fs::remove_file(entry.path()).is_ok() {
                                    deleted_count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(deleted_count)
    }
    
    // Export als ZIP
    pub fn export_backups(&self, export_path: &Path) -> Result<(), String> {
        use zip::write::FileOptions;
        use zip::ZipWriter;
        use std::io::Write;
        
        let file = fs::File::create(export_path)
            .map_err(|e| format!("Fehler beim Erstellen der ZIP-Datei: {}", e))?;
        
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        
        for browser in &["Chrome", "Edge", "Firefox"] {
            let browser_dir = self.backup_dir.join(browser);
            if let Ok(entries) = fs::read_dir(&browser_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let name = format!("{}/{}", browser, entry.file_name().to_string_lossy());
                        zip.start_file(name, options)
                            .map_err(|e| format!("ZIP Fehler: {}", e))?;
                        
                        let mut file = fs::File::open(&path)
                            .map_err(|e| format!("Fehler beim Lesen: {}", e))?;
                        let mut buffer = Vec::new();
                        file.read_to_end(&mut buffer)
                            .map_err(|e| format!("Fehler beim Lesen: {}", e))?;
                        
                        zip.write_all(&buffer)
                            .map_err(|e| format!("Fehler beim Schreiben: {}", e))?;
                    }
                }
            }
        }
        
        zip.finish().map_err(|e| format!("Fehler beim Finalisieren: {}", e))?;
        Ok(())
    }
    
    // Favoriten als HTML exportieren
    pub fn export_as_html(&self, browser: &str, output_path: &Path) -> Result<(), String> {
        let latest_backup = self.get_backup_list(browser)
            .into_iter()
            .next()
            .ok_or("Kein Backup gefunden")?;
        
        let content = fs::read_to_string(&latest_backup.path)
            .map_err(|e| format!("Fehler beim Lesen: {}", e))?;
        
        match browser {
            "Chrome" | "Edge" => {
                // Parse JSON und konvertiere zu HTML
                let bookmarks: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| format!("JSON Parse Fehler: {}", e))?;
                
                let html = self.json_to_html(&bookmarks);
                fs::write(output_path, html)
                    .map_err(|e| format!("Fehler beim Schreiben: {}", e))?;
            }
            "Firefox" => {
                // SQLite zu HTML würde zusätzliche Dependencies benötigen
                return Err("Firefox HTML Export noch nicht implementiert".to_string());
            }
            _ => return Err("Unbekannter Browser".to_string()),
        }
        
        Ok(())
    }
    
    fn json_to_html(&self, bookmarks: &serde_json::Value) -> String {
        let mut html = String::from(
            "<!DOCTYPE html>\n\
            <html>\n\
            <head>\n\
                <meta charset=\"UTF-8\">\n\
                <title>Browser Favoriten</title>\n\
                <style>\n\
                    body { font-family: Arial, sans-serif; margin: 20px; }\n\
                    ul { list-style-type: none; }\n\
                    a { text-decoration: none; color: #0066cc; }\n\
                    a:hover { text-decoration: underline; }\n\
                    .folder { font-weight: bold; margin: 10px 0; }\n\
                </style>\n\
            </head>\n\
            <body>\n\
                <h1>Browser Favoriten</h1>\n"
        );
        
        // Rekursive Funktion zum Parsen der Bookmarks
        fn parse_folder(folder: &serde_json::Value, depth: usize) -> String {
            let mut result = String::new();
            let indent = "    ".repeat(depth);
            
            if let Some(name) = folder.get("name").and_then(|v| v.as_str()) {
                if depth > 0 {
                    result.push_str(&format!("{}<div class=\"folder\">{}</div>\n", indent, name));
                }
            }
            
            if let Some(children) = folder.get("children").and_then(|v| v.as_array()) {
                result.push_str(&format!("{}<ul>\n", indent));
                
                for child in children {
                    if let Some(type_) = child.get("type").and_then(|v| v.as_str()) {
                        match type_ {
                            "folder" => {
                                result.push_str(&parse_folder(child, depth + 1));
                            }
                            "url" => {
                                if let (Some(name), Some(url)) = (
                                    child.get("name").and_then(|v| v.as_str()),
                                    child.get("url").and_then(|v| v.as_str())
                                ) {
                                    result.push_str(&format!(
                                        "{}    <li><a href=\"{}\">{}</a></li>\n",
                                        indent, url, name
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                
                result.push_str(&format!("{}</ul>\n", indent));
            }
            
            result
        }
        
        if let Some(roots) = bookmarks.get("roots").and_then(|v| v.as_object()) {
            for (_, folder) in roots {
                html.push_str(&parse_folder(folder, 0));
            }
        }
        
        html.push_str("</body>\n</html>");
        html
    }
}
