//! Windows resource file with app icon
//! 
//! Este arquivo é necessário quando o build não consegue processar o icon.ico.
//! Usa o ícone padrão de aplicativo 48x48 do sistema.

// This is a minimal rsrc file that doesn't embed an icon.
// Tauri will fall back to a default icon if the icon.ico cannot be processed.
//
// To embed a custom icon, you can use a resource script (.rc) or ico file.
// The icon.ico file should be a valid 32-bit ICO format.

use tauri::ResourceBundle;