#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CloudDrive {
    pub id: String,
    pub label: String,
    pub remote_name: String,
    pub provider_type: String,
    pub mount_point: String,
    pub sync_local_dir: String,
    pub sync_direction: String,
    pub conflict_mode: String,
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
            provider_type TEXT NOT NULL,
            mount_point TEXT NOT NULL,
            sync_local_dir TEXT NOT NULL,
            sync_direction TEXT NOT NULL,
            conflict_mode TEXT NOT NULL,
            auto_mount INTEGER NOT NULL DEFAULT 0
        )",
        [],
    ).map_err(|e| e.to_string())?;
    Ok(conn)
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
        (true, true) => "Système opérationnel : Rclone et WinFsp/FUSE sont détectés.".into(),
        (false, true) => "Attention : Rclone est introuvable dans le PATH système.".into(),
        (true, false) => "Attention : WinFsp (pilote de disque FUSE) n'est pas installé sur ce PC.".into(),
        (false, false) => "Rclone et WinFsp sont tous les deux manquants sur cette machine.".into(),
    };

    SystemCheckStatus {
        rclone_installed: rclone_ok,
        fuse_or_winfsp_installed: driver_ok,
        message: msg,
    }
}

#[tauri::command]
fn get_drives() -> Result<Vec<CloudDrive>, String> {
    let conn = get_db_connection()?;
    let mut stmt = conn
        .prepare("SELECT id, label, remote_name, provider_type, mount_point, sync_local_dir, sync_direction, conflict_mode, auto_mount FROM drives")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let auto_int: i32 = row.get(8)?;
            Ok(CloudDrive {
                id: row.get(0)?,
                label: row.get(1)?,
                remote_name: row.get(2)?,
                provider_type: row.get(3)?,
                mount_point: row.get(4)?,
                sync_local_dir: row.get(5)?,
                sync_direction: row.get(6)?,
                conflict_mode: row.get(7)?,
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
        (id, label, remote_name, provider_type, mount_point, sync_local_dir, sync_direction, conflict_mode, auto_mount)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            drive.id,
            drive.label,
            drive.remote_name,
            drive.provider_type,
            drive.mount_point,
            drive.sync_local_dir,
            drive.sync_direction,
            drive.conflict_mode,
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

        let mut args = vec![
            "mount".to_string(),
            format!("{}:", drive.remote_name),
            drive.mount_point.clone(),
            "--vfs-cache-mode".to_string(),
            "full".to_string(),
            "--volname".to_string(),
            drive.label.clone(),
        ];

        if !cfg!(target_os = "windows") {
            args.push("--daemon".to_string());
        }

        Command::new("rclone")
            .args(&args)
            .spawn()
            .map_err(|e| format!("Échec du lancement Rclone: {}", e))?;

        Ok(true)
    }
}

#[tauri::command]
fn run_sync(drive: CloudDrive) -> Result<String, String> {
    fs::create_dir_all(&drive.sync_local_dir).ok();
    let src = if drive.sync_direction == "local_to_remote" {
        drive.sync_local_dir.clone()
    } else {
        format!("{}:", drive.remote_name)
    };

    let dst = if drive.sync_direction == "local_to_remote" {
        format!("{}:", drive.remote_name)
    } else {
        drive.sync_local_dir.clone()
    };

    let mut args = vec!["rclone".to_string()];
    match drive.conflict_mode.as_str() {
        "sync" => args.extend(["sync".to_string(), src, dst]),
        "backup" => {
            let backup_dir = if drive.sync_direction == "local_to_remote" {
                format!("{}/_backup", dst)
            } else {
                format!("{}/_backup", drive.sync_local_dir)
            };
            args.extend(["sync".to_string(), src, dst, "--backup-dir".to_string(), backup_dir]);
        }
        _ => args.extend(["copy".to_string(), src, dst]),
    }

    let output = Command::new(&args[0])
        .args(&args[1..])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok("Synchronisation terminée avec succès.".to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn sync_system_automount(drive: &CloudDrive) {
    if cfg!(target_os = "windows") {
        let task_name = format!("RcloneMount_{}", drive.id);
        if drive.auto_mount {
            let tr = format!(
                "rclone mount {}: {} --vfs-cache-mode full --volname \"{}\"",
                drive.remote_name, drive.mount_point, drive.label
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
            check_system_dependencies,
            get_drives,
            save_drive,
            delete_drive,
            check_mount,
            toggle_mount,
            run_sync
        ])
        .run(tauri::generate_context!())
        .expect("Erreur lors de l'exécution de l'application");
}