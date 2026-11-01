// Modèle de base pour Tauri v2 sans plugins superflus
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("erreur lors du lancement de l'application");
}