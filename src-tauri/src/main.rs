#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CloudDrive {
    pub id: String,
    pub label: String,
    pub remote_name: String,
    pub remote_subpath: String,
    pub provider_type: String,
    pub mount_point: String,
    pub sync_local_dir: String,
    pub sync_direction: String,
    pub conflict_mode: String,
    pub streaming_mode: bool,
    pub auto_mount: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemCheckStatus {
    pub rclone_installed: bool,
    pub fuse_or_winfsp_installed: bool,
    pub message: String,
}

fn get_db_path() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("cloud_manager");
    fs::create_dir_all(&p).ok();
    p.push("drives.db");
    p
}

fn get_db_connection() -> Result<Connection, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS drives (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL,
            remote_name TEXT NOT NULL,
            remote_subpath TEXT NOT NULL DEFAULT '',
            provider_type TEXT NOT NULL,
            mount_point TEXT NOT NULL,
            sync_local_dir TEXT NOT NULL,
            sync_direction TEXT NOT NULL,
            conflict_mode TEXT NOT NULL,
            streaming_mode INTEGER NOT NULL DEFAULT 1,
            auto_mount INTEGER NOT NULL DEFAULT 0
        )",
        [],
    ).map_err(|e| e.to_string())?;

    conn.execute("ALTER TABLE drives ADD COLUMN remote_subpath TEXT NOT NULL DEFAULT ''", []).ok();
    conn.execute("ALTER TABLE drives ADD COLUMN streaming_mode INTEGER NOT NULL DEFAULT 1", []).ok();

    Ok(conn)
}

#[tauri::command]
fn get_available_drive_letters() -> Vec<String> {
    if !cfg!(target_os = "windows") {
        return vec!["/mnt/cloud_drive".into(), "~/Drives/Cloud".into()];
    }

    let mut available = Vec::new();
    // Évalue les lettres de D à Z (en réservant C:)
    for b in b'D'..=b'Z' {
        let letter = format!("{}:", b as char);
        let path_str = format!("{}\\", letter);
        if !Path::new(&path_str).exists() {
            available.push(letter);
        }
    }
    available
}

#[tauri::command]
fn check_system_dependencies() -> SystemCheckStatus {
    let rclone_ok = Command::new("rclone").arg("version").output().is_ok();

    let driver_ok = if cfg!(target_os = "windows") {
        let p1 = PathBuf::from("C:\\Program Files\\WinFsp");
        let p2 = PathBuf::from("C:\\Program Files (x86)\\WinFsp");
        p1.exists() || p2.exists()
    } else {
        Command::new("mountpoint").arg("-q").output().is_ok()
            || Command::new("fusermount3").arg("-V").output().is_ok()
            || Command::new("fusermount").arg("-V").output().is_ok()
    };

    let msg = match (rclone_ok, driver_ok) {
        (true, true) => "Système prêt. Rclone et le pilote de disque sont opérationnels.".into(),
        (false, true) => "Rclone est manquant. Cliquez sur 'Installer Rclone'.".into(),
        (true, false) => "Pilote WinFsp manquant sous Windows pour le montage de disque.".into(),
        (false, false) => "Rclone et WinFsp sont absents sur ce PC.".into(),
    };

    SystemCheckStatus {
        rclone_installed: rclone_ok,
        fuse_or_winfsp_installed: driver_ok,
        message: msg,
    }
}

#[tauri::command]
fn install_rclone() -> Result<String, String> {
    if cfg!(target_os = "windows") {
        let output = Command::new("winget")
            .args([
                "install",
                "--id",
                "Rclone.Rclone",
                "-e",
                "--silent",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ])
            .output()
            .map_err(|e| format!("Erreur winget: {}", e))?;

        if output.status.success() {
            Ok("Rclone a été installé avec succès.".into())
        } else {
            Err(format!("Échec installation: {}", String::from_utf8_lossy(&output.stderr)))
        }
    } else {
        let output = Command::new("sh")
            .arg("-c")
            .arg("curl https://rclone.org/install.sh | sudo bash")
            .output()
            .map_err(|e| format!("Erreur script: {}", e))?;

        if output.status.success() {
            Ok("Rclone installé avec succès.".into())
        } else {
            Err(format!("Erreur: {}", String::from_utf8_lossy(&output.stderr)))
        }
    }
}

#[tauri::command]
fn list_remote_folders(remote_name: String, base_path: String) -> Result<Vec<String>, String> {
    let target = if base_path.is_empty() {
        format!("{}:", remote_name)
    } else {
        format!("{}:{}", remote_name, base_path.trim_matches('/'))
    };

    let output = Command::new("rclone")
        .args(["lsf", &target, "--dirs-only", "--max-depth", "1"])
        .output()
        .map_err(|e| format!("Erreur rclone: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let folders: Vec<String> = text
        .lines()
        .map(|l| l.trim_end_matches('/').to_string())
        .filter(|l| !l.is_empty())
        .collect();

    Ok(folders)
}

#[tauri::command]
fn configure_scaleway_remote(remote_name: String, access_key: String, secret_key: String, region: String, endpoint: String) -> Result<String, String> {
    let output = Command::new("rclone")
        .args([
            "config", "create",
            &remote_name,
            "s3",
            "provider", "Scaleway",
            "access_key_id", &access_key,
            "secret_access_key", &secret_key,
            "region", &region,
            "endpoint", &endpoint,
            "acl", "private"
        ])
        .output()
        .map_err(|e| format!("Échec d'exécution Rclone: {}", e))?;

    if output.status.success() {
        Ok(format!("Le stockage Scaleway S3 '{}' est prêt.", remote_name))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
fn start_google_drive_oauth(remote_name: String) -> Result<String, String> {
    Command::new("rclone")
        .args(["config", "create", &remote_name, "drive", "scope", "drive"])
        .spawn()
        .map_err(|e| format!("Impossible d'ouvrir le navigateur: {}", e))?;

    Ok("Navigateur ouvert pour l'authentification Google. Validez l'accès puis continuez.".into())
}

#[tauri::command]
fn get_drives() -> Result<Vec<CloudDrive>, String> {
    let conn = get_db_connection()?;
    let mut stmt = conn
        .prepare("SELECT id, label, remote_name, remote_subpath, provider_type, mount_point, sync_local_dir, sync_direction, conflict_mode, streaming_mode, auto_mount FROM drives")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let stream_int: i32 = row.get(9)?;
            let auto_int: i32 = row.get(10)?;
            Ok(CloudDrive {
                id: row.get(0)?,
                label: row.get(1)?,
                remote_name: row.get(2)?,
                remote_subpath: row.get(3)?,
                provider_type: row.get(4)?,
                mount_point: row.get(5)?,
                sync_local_dir: row.get(6)?,
                sync_direction: row.get(7)?,
                conflict_mode: row.get(8)?,
                streaming_mode: stream_int != 0,
                auto_mount: auto_int != 0,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for r in rows {
        if let Ok(item) = r {
            result.push(item);
        }
    }
    Ok(result)
}

#[tauri::command]
fn save_drive(drive: CloudDrive) -> Result<(), String> {
    let conn = get_db_connection()?;
    conn.execute(
        "INSERT OR REPLACE INTO drives 
        (id, label, remote_name, remote_subpath, provider_type, mount_point, sync_local_dir, sync_direction, conflict_mode, streaming_mode, auto_mount)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            drive.id,
            drive.label,
            drive.remote_name,
            drive.remote_subpath,
            drive.provider_type,
            drive.mount_point,
            drive.sync_local_dir,
            drive.sync_direction,
            drive.conflict_mode,
            if drive.streaming_mode { 1 } else { 0 },
            if drive.auto_mount { 1 } else { 0 },
        ],
    ).map_err(|e| e.to_string())?;

    sync_system_automount(&drive);
    Ok(())
}

#[tauri::command]
fn delete_drive(id: String) -> Result<(), String> {
    let conn = get_db_connection()?;
    conn.execute("DELETE FROM drives WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn check_mount(mount_point: String) -> bool {
    if cfg!(target_os = "windows") {
        PathBuf::from(&mount_point).exists()
    } else {
        Command::new("mountpoint")
            .arg("-q")
            .arg(&mount_point)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// 1. Montage détaché pour que Rclone reste actif à la fermeture de la fenêtre
#[tauri::command]
fn toggle_mount(drive: CloudDrive) -> Result<bool, String> {
    let currently_mounted = check_mount(drive.mount_point.clone());

    if currently_mounted {
        if cfg!(target_os = "windows") {
            Command::new("taskkill")
                .args(["/IM", "rclone.exe", "/F"])
                .output()
                .map_err(|e| e.to_string())?;
        } else {
            Command::new("fusermount")
                .args(["-u", &drive.mount_point])
                .output()
                .map_err(|e| e.to_string())?;
        }
        Ok(false)
    } else {
        if !cfg!(target_os = "windows") {
            fs::create_dir_all(&drive.mount_point).ok();
        }

        let full_remote = if drive.remote_subpath.is_empty() {
            format!("{}:", drive.remote_name)
        } else {
            format!("{}:{}", drive.remote_name, drive.remote_subpath.trim_matches('/'))
        };

        let mut cmd = Command::new("rclone");
        let mut args = vec![
            "mount".to_string(),
            full_remote,
            drive.mount_point.clone(),
            "--volname".to_string(),
            drive.label.clone(),
        ];

        if drive.streaming_mode {
            args.extend([
                "--vfs-cache-mode".to_string(),
                "off".to_string(),
                "--vfs-read-chunk-size".to_string(),
                "32M".to_string(),
                "--vfs-read-chunk-size-limit".to_string(),
                "off".to_string(),
            ]);
        } else {
            args.extend([
                "--vfs-cache-mode".to_string(),
                "full".to_string(),
            ]);
        }

        cmd.args(&args);

        #[cfg(target_os = "windows")]
        {
            // CREATE_NO_WINDOW (0x08000000) + DETACHED_PROCESS (0x00000008)
            cmd.creation_flags(0x08000000 | 0x00000008);
        }

        #[cfg(not(target_os = "windows"))]
        {
            cmd.arg("--daemon");
        }

        cmd.spawn()
            .map_err(|e| format!("Échec lancement Rclone détaché: {}", e))?;

        Ok(true)
    }
}

// 2. Exportation de la configuration et de la base de données
#[tauri::command]
fn export_database_json() -> Result<String, String> {
    let drives = get_drives()?;
    serde_json::to_string_pretty(&drives).map_err(|e| e.to_string())
}

// Synchronisation avec support de simulation Dry-Run
#[tauri::command]
fn run_sync(drive: CloudDrive, dry_run: bool) -> Result<String, String> {
    fs::create_dir_all(&drive.sync_local_dir).ok();

    let local_path = drive.sync_local_dir.clone();
    let remote_target = if drive.remote_subpath.is_empty() {
        format!("{}:", drive.remote_name)
    } else {
        format!("{}:{}", drive.remote_name, drive.remote_subpath.trim_matches('/'))
    };

    let mut args: Vec<String> = Vec::new();

    if drive.sync_direction == "bidirectional" {
        args.extend([
            "bisync".to_string(),
            local_path,
            remote_target,
            "--verbose".to_string(),
        ]);
        if dry_run {
            args.push("--dry-run".to_string());
        }
    } else {
        let (src, dst) = if drive.sync_direction == "local_to_remote" {
            (local_path, remote_target)
        } else {
            (remote_target, local_path)
        };

        if drive.conflict_mode == "sync" {
            args.push("sync".to_string());
        } else {
            args.push("copy".to_string());
        }
        args.extend([src, dst, "--verbose".to_string()]);
        if dry_run {
            args.push("--dry-run".to_string());
        }
    }

    let output = Command::new("rclone")
        .args(&args)
        .output()
        .map_err(|e| format!("Erreur exécution rclone: {}", e))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    let full_log = format!("{}\n{}", stdout_str, stderr_str);

    if dry_run {
        let simulated_lines: Vec<&str> = full_log
            .lines()
            .filter(|l| {
                l.contains("NOTICE")
                    || l.contains("Transferring")
                    || l.contains("Copied")
                    || l.contains("Deleted")
            })
            .take(20)
            .collect();

        if simulated_lines.is_empty() {
            Ok("Simulation terminée : Aucun fichier à synchroniser (vos dossiers sont déjà parfaitement synchronisés).".into())
        } else {
            Ok(format!(
                "Simulation [Dry Run] réussie :\n{}",
                simulated_lines.join("\n")
            ))
        }
    } else if output.status.success() {
        Ok("Synchronisation terminée avec succès.".into())
    } else {
        Err(stderr_str.to_string())
    }
}

fn sync_system_automount(drive: &CloudDrive) {
    if cfg!(target_os = "windows") {
        let task_name = format!("RcloneMount_{}", drive.id);
        if drive.auto_mount {
            let full_remote = if drive.remote_subpath.is_empty() {
                format!("{}:", drive.remote_name)
            } else {
                format!("{}:{}", drive.remote_name, drive.remote_subpath.trim_matches('/'))
            };

            let cache_flag = if drive.streaming_mode {
                "--vfs-cache-mode off --vfs-read-chunk-size 32M"
            } else {
                "--vfs-cache-mode full"
            };

            let tr = format!(
                "rclone mount {} {} {} --volname \"{}\"",
                full_remote, drive.mount_point, cache_flag, drive.label
            );

            Command::new("schtasks")
                .args(["/Create", "/F", "/TN", &task_name, "/TR", &tr, "/SC", "ONLOGON", "/RL", "HIGHEST"])
                .output()
                .ok();
        } else {
            Command::new("schtasks")
                .args(["/Delete", "/F", "/TN", &task_name])
                .output()
                .ok();
        }
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_available_drive_letters,
            check_system_dependencies,
            install_rclone,
            list_remote_folders,
            configure_scaleway_remote,
            start_google_drive_oauth,
            get_drives,
            save_drive,
            delete_drive,
            check_mount,
            toggle_mount,
            run_sync,
            export_database_json
        ])
        .run(tauri::generate_context!())
        .expect("Erreur lors de l'exécution de l'application");
}