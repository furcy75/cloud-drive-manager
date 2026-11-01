# ☁️ Cloud Drive Manager

**Cloud Drive Manager** est une application de bureau open source, ultra-légère et multiplateforme (Linux & Windows), conçue avec **Rust** et **Tauri**. 

Elle permet de monter plusieurs stockages cloud (**Google Drive**, **Scaleway Object Storage S3**, etc.) sous forme de disques durs externes ou lecteurs réseau virtuels, avec un contrôle total sur l'automatisation système et la synchronisation locale.

---

## ✨ Fonctionnalités principales

- **Multi-Cloud universel** : Compatible avec Google Drive, Scaleway S3, AWS S3, Dropbox, OneDrive, etc. (propulsé par le moteur open source Rclone).
- **Nommage personnalisé des volumes** :
  - **Sous Windows** : Affiche le nom personnalisé directement dans l'Explorateur (`--volname`, ex: `MonStockagePro (G:)`).
  - **Sous Linux** : Monte le disque dans un répertoire dédié (`~/Drives/MonStockagePro`).
- **Montage On / Off en un clic** : Montez et démontez vos disques à la volée via l'interface graphique.
- **Démarrage sans interface (Services natifs)** :
  - Option pour monter les disques automatiquement au démarrage de la machine sans ouvrir le logiciel.
  - Utilise les **services utilisateurs systemd** sous Linux et le **Planificateur de tâches** (`schtasks`) sous Windows.
- **Synchronisation manuelle sur mesure** :
  - Choix du dossier local cible (ex: sur le Bureau).
  - Choix du sens de synchronisation (*Local ➜ Cloud* ou *Cloud ➜ Local*).
  - 3 politiques de résolution de conflits :
    1. **Copie simple** (`copy`) : met à jour sans supprimer les fichiers locaux/distants orphelins.
    2. **Miroir strict** (`sync`) : reflète rigoureusement la source en supprimant le superflu sur la cible.
    3. **Sauvegarde des conflits** (`backup`) : déplace les versions écrasées ou supprimées dans un sous-dossier horodaté `_backup`.
- **Persistance SQLite intégrée** : Base de données locale autonome stockant l'ensemble de vos configurations sans dépendance externe.

---

## 📥 Téléchargement des exécutables

Les binaires précompilés sont générés automatiquement par **GitHub Actions** à chaque mise à jour :

1. Rendez-vous dans l'onglet [**Actions**](../../actions) de ce dépôt.
2. Cliquez sur la dernière exécution terminée (avec une coche verte).
3. Rendez-vous tout en bas dans la section **Artifacts** :
   - **`binaries-windows-latest`** : contient l'exécutable portable Windows (`.exe`).
   - **`binaries-ubuntu-22.04`** : contient le paquet d'installation Linux (`.deb` et `.AppImage`).

---

## ⚙️ Prérequis sur la machine cible

L'application s'appuie sur le moteur **Rclone** et les couches de montage de systèmes de fichiers virtuels :

### Sous Windows
1. **Rclone** : Téléchargez [rclone.exe](https://rclone.org/downloads/) et ajoutez-le au `PATH` Windows (ou placez-le dans le même dossier que l'exécutable).
2. **WinFsp** *(Indispensable)* : Installez le pilote FUSE pour Windows via [WinFsp](https://winfsp.dev/).

### Sous Linux
Vérifiez la présence de `rclone` et `fuse3` via votre gestionnaire de paquets :
```bash
# Debian / Ubuntu / Linux Mint
sudo apt update && sudo apt install -y rclone fuse3

# Fedora
sudo dnf install -y rclone fuse3

# Arch Linux
sudo pacman -S --noconfirm rclone fuse3