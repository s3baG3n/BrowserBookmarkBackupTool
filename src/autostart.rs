#[cfg(target_os = "windows")]
pub fn setup_autostart(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
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

#[cfg(not(target_os = "windows"))]
pub fn setup_autostart(_enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Not implemented for non-Windows platforms
    Ok(())
}