// backup_manager.rs - Fixed version
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use html_escape::encode_text;
use std::thread;
use std::time::Duration;
use rusqlite::{Connection, Result as SqlResult};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
enum BackupError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    BrowserNotFound(String),
}

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
        
        manager.ensure_backup_dir().ok();
        manager.load_config();
        manager
    }
    
    fn get_default_backup_dir() -> PathBuf {
        let user_profile = std::env::var("USERPROFILE")
            .unwrap_or_else(|_| dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "C:\\".to_string()));
        
        PathBuf::from(user_profile)
            .join("Work Folders")
            .join("Benutzerdatensicherung")
            .join("Bookmarks")
    }
    
    fn ensure_backup_dir(&self) -> Result<(), std::io::Error> {
        if !self.backup_dir.exists() {
            fs::create_dir_all(&self.backup_dir)?;
        }
        Ok(())
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
        if let Err(e) = fs::create_dir_all(&browser_backup_dir) {
            return BackupResult {
                browser: browser.to_string(),
                success: false,
                message: format!("Fehler beim Erstellen des Verzeichnisses: {}", e),
            };
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
        let user_profile = std::env::var("USERPROFILE")
            .map_err(|_| "USERPROFILE environment variable not found".to_string())?;
        
        let target_path = match browser {
            "Chrome" => PathBuf::from(&user_profile)
                .join("AppData")
                .join("Local")
                .join("Google")
                .join("Chrome")
                .join("User Data")
                .join("Default")
                .join("Bookmarks"),
            "Edge" => PathBuf::from(&user_profile)
                .join("AppData")
                .join("Local")
                .join("Microsoft")
                .join("Edge")
                .join("User Data")
                .join("Default")
                .join("Bookmarks"),
            "Firefox" => {
                let profiles_path = PathBuf::from(&user_profile)
                    .join("AppData")
                    .join("Roaming")
                    .join("Mozilla")
                    .join("Firefox")
                    .join("Profiles");
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

    // Static method for scheduling that doesn't create new instances
    pub fn start_scheduled_backups(backup_manager: Arc<Mutex<BackupManager>>, interval_hours: u64) {
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(interval_hours * 3600));
                
                if let Ok(manager) = backup_manager.lock() {
                    let results = manager.backup_all();
                    
                    println!("Automatisches Backup durchgeführt: {:?}", results);
                    
                    for result in &results {
                        if result.success {
                            println!("✓ {} backup successful: {}", result.browser, result.message);
                        } else {
                            eprintln!("✗ {} backup failed: {}", result.browser, result.message);
                        }
                    }
                }
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
        
        match browser {
            "Chrome" | "Edge" => {
                let content = fs::read_to_string(&latest_backup.path)
                    .map_err(|e| format!("Fehler beim Lesen: {}", e))?;
                
                // Parse JSON und konvertiere zu HTML
                let bookmarks: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| format!("JSON Parse Fehler: {}", e))?;
                
                let html = self.json_to_html(&bookmarks);
                fs::write(output_path, html)
                    .map_err(|e| format!("Fehler beim Schreiben: {}", e))?;
            }
            "Firefox" => {
                // Firefox SQLite to HTML conversion
                let html = self.firefox_sqlite_to_html(&latest_backup.path)?;
                fs::write(output_path, html)
                    .map_err(|e| format!("Fehler beim Schreiben: {}", e))?;
            }
            _ => return Err("Unbekannter Browser".to_string()),
        }
        
        Ok(())
    }
 
    fn firefox_sqlite_to_html(&self, db_path: &Path) -> Result<String, String> {
        // Open the SQLite database
        let conn = Connection::open(db_path)
            .map_err(|e| format!("Fehler beim Öffnen der Firefox-Datenbank: {}", e))?;
        
        // Query to get bookmarks with folder structure
        let query = r#"
            WITH RECURSIVE
            bookmark_tree(id, parent, title, url, position, level, path) AS (
                -- Root folders
                SELECT 
                    b.id,
                    b.parent,
                    b.title,
                    p.url,
                    b.position,
                    0 as level,
                    b.title as path
                FROM moz_bookmarks b
                LEFT JOIN moz_places p ON b.fk = p.id
                WHERE b.parent IN (1, 2, 3, 4, 5)  -- Standard Firefox root folders
                
                UNION ALL
                
                -- Recursive part
                SELECT 
                    b.id,
                    b.parent,
                    b.title,
                    p.url,
                    b.position,
                    bt.level + 1,
                    bt.path || ' > ' || b.title
                FROM moz_bookmarks b
                LEFT JOIN moz_places p ON b.fk = p.id
                JOIN bookmark_tree bt ON b.parent = bt.id
            )
            SELECT id, parent, title, url, position, level, path
            FROM bookmark_tree
            WHERE title IS NOT NULL
            ORDER BY parent, position
        "#;
        
        let mut stmt = conn.prepare(query)
            .map_err(|e| format!("Fehler beim Vorbereiten der SQL-Abfrage: {}", e))?;
        
        #[derive(Debug)]
        struct Bookmark {
            id: i64,
            parent: i64,
            title: String,
            url: Option<String>,
            position: i32,
            level: i32,
        }
        
        let bookmarks_iter = stmt.query_map([], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                parent: row.get(1)?,
                title: row.get(2)?,
                url: row.get(3)?,
                position: row.get(4)?,
                level: row.get(5)?,
            })
        }).map_err(|e| format!("Fehler beim Ausführen der SQL-Abfrage: {}", e))?;
        
        let mut bookmarks: Vec<Bookmark> = Vec::new();
        for bookmark_result in bookmarks_iter {
            bookmarks.push(bookmark_result.map_err(|e| format!("Fehler beim Lesen der Lesezeichen: {}", e))?);
        }
        
        // Build HTML
        let mut html = String::from(
            "<!DOCTYPE html>\n\
            <html>\n\
            <head>\n\
                <meta charset=\"UTF-8\">\n\
                <title>Firefox Favoriten</title>\n\
                <style>\n\
                    body { font-family: Arial, sans-serif; margin: 20px; }\n\
                    ul { list-style-type: none; padding-left: 20px; }\n\
                    li { margin: 5px 0; }\n\
                    a { text-decoration: none; color: #0066cc; }\n\
                    a:hover { text-decoration: underline; }\n\
                    .folder { font-weight: bold; margin: 10px 0; }\n\
                    .root { margin-left: 0; padding-left: 0; }\n\
                </style>\n\
            </head>\n\
            <body>\n\
                <h1>Firefox Favoriten</h1>\n"
        );
        
        // Group bookmarks by parent
        use std::collections::HashMap;
        let mut children_map: HashMap<i64, Vec<&Bookmark>> = HashMap::new();
        for bookmark in &bookmarks {
            children_map.entry(bookmark.parent).or_insert_with(Vec::new).push(bookmark);
        }
        
        // Recursive function to build HTML
        fn build_html_tree(
            parent_id: i64,
            children_map: &HashMap<i64, Vec<&Bookmark>>,
            level: usize
        ) -> String {
            let mut result = String::new();
            
            if let Some(children) = children_map.get(&parent_id) {
                let indent = "    ".repeat(level);
                result.push_str(&format!("{}<ul{}>\n", 
                    indent, 
                    if level == 0 { " class=\"root\"" } else { "" }
                ));
                
                for child in children {
                    if child.url.is_some() {
                        // It's a bookmark
                        result.push_str(&format!(
                            "{}    <li><a href=\"{}\">{}</a></li>\n",
                            indent,
                            encode_text(child.url.as_ref().unwrap()).as_ref(),
                            encode_text(&child.title).as_ref()
                        ));
                    } else {
                        // It's a folder
                        result.push_str(&format!(
                            "{}    <li class=\"folder\">{}\n",
                            indent,
                            encode_text(&child.title).as_ref()
                        ));
                        
                        // Recursively add children
                        result.push_str(&build_html_tree(child.id, children_map, level + 2));
                        
                        result.push_str(&format!("{}    </li>\n", indent));
                    }
                }
                
                result.push_str(&format!("{}</ul>\n", indent));
            }
            
            result
        }
        
        // Start with root folders (IDs 1-5 are standard Firefox roots)
        for root_id in 1..=5 {
            html.push_str(&build_html_tree(root_id, &children_map, 0));
        }
        
        html.push_str("</body>\n</html>");
        
        Ok(html)
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
                    result.push_str(&format!("{}<div class=\"folder\">{}</div>\n", indent, encode_text(name)));
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
                                        indent,    
                                        encode_text(url).as_ref(),
                                        encode_text(name).as_ref()
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