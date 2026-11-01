// Récupération sécurisée de l'API Tauri
function getTauriInvoke() {
  if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
    return window.__TAURI__.core.invoke;
  }
  if (window.__TAURI__ && window.__TAURI__.invoke) {
    return window.__TAURI__.invoke;
  }
  return null;
}

async function checkDependencies(invoke) {
  const alertBox = document.getElementById("system-alert");
  try {
    const status = await invoke("check_system_dependencies");
    alertBox.style.display = "block";
    alertBox.textContent = status.message;

    if (status.rclone_installed && status.fuse_or_winfsp_installed) {
      alertBox.className = "alert-banner alert-success";
    } else {
      alertBox.className = "alert-banner alert-warning";
    }
  } catch (e) {
    alertBox.style.display = "block";
    alertBox.className = "alert-banner alert-warning";
    alertBox.textContent = "Impossible de vérifier les dépendances système: " + e;
  }
}

async function loadDrives(invoke) {
  const container = document.getElementById("drive-list");
  try {
    const drives = await invoke("get_drives");
    if (!drives || drives.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          Aucun disque configuré pour le moment.<br>
          Cliquez sur <b>"+ Ajouter un Disque"</b> pour commencer.
        </div>`;
      return;
    }

    container.innerHTML = "";
    for (const d of drives) {
      const isMounted = await invoke("check_mount", { mountPoint: d.mount_point });
      const card = document.createElement("div");
      card.className = "card";
      card.innerHTML = `
        <div>
          <strong style="font-size: 16px;">${d.label}</strong> 
          <code style="background:#eee; padding:2px 6px; border-radius:4px;">${d.mount_point}</code>
          <br><small style="color:#666;">${d.provider_type} (${d.remote_name}) ${d.auto_mount ? '• <i>Automount au boot</i>' : ''}</small>
        </div>
        <div>
          <button class="${isMounted ? 'btn-green' : 'btn-red'}" id="mount-btn-${d.id}">
            ${isMounted ? 'Démonter (ON)' : 'Monter (OFF)'}
          </button>
          <button class="btn-blue" id="sync-btn-${d.id}">Synchroniser</button>
          <button id="del-btn-${d.id}">Supprimer</button>
        </div>
      `;
      container.appendChild(card);

      document.getElementById(`mount-btn-${d.id}`).onclick = async () => {
        await invoke("toggle_mount", { drive: d });
        loadDrives(invoke);
      };

      document.getElementById(`sync-btn-${d.id}`).onclick = async () => {
        alert("Lancement de la synchronisation...");
        try {
          const res = await invoke("run_sync", { drive: d });
          alert(res);
        } catch (err) {
          alert("Erreur de synchronisation: " + err);
        }
      };

      document.getElementById(`del-btn-${d.id}`).onclick = async () => {
        if (confirm(`Supprimer le disque "${d.label}" ?`)) {
          await invoke("delete_drive", { id: d.id });
          loadDrives(invoke);
        }
      };
    }
  } catch (err) {
    container.innerHTML = `<div class="empty-state" style="color:#c62828;">Erreur lors du chargement : ${err}</div>`;
  }
}

document.addEventListener("DOMContentLoaded", () => {
  const modal = document.getElementById("modal");
  const btnOpen = document.getElementById("btn-open-modal");
  const btnCancel = document.getElementById("btn-cancel");
  const btnSubmit = document.getElementById("btn-submit");

  // Rendre l'ouverture et fermeture du modal toujours fonctionnelle
  btnOpen.onclick = () => { modal.style.display = "flex"; };
  btnCancel.onclick = () => { modal.style.display = "none"; };

  const invoke = getTauriInvoke();
  if (!invoke) {
    const alertBox = document.getElementById("system-alert");
    alertBox.style.display = "block";
    alertBox.className = "alert-banner alert-warning";
    alertBox.textContent = "Attention : L'API Tauri n'est pas encore prête ou withGlobalTauri est désactivé.";
    return;
  }

  // Vérification système et liste des disques
  checkDependencies(invoke);
  loadDrives(invoke);

  // Enregistrement d'un nouveau disque
  btnSubmit.onclick = async () => {
    const label = document.getElementById("label").value.trim();
    const remote_name = document.getElementById("remote_name").value.trim();
    const mount_point = document.getElementById("mount_point").value.trim();

    if (!label || !remote_name || !mount_point) {
      alert("Veuillez renseigner au minimum le nom, le remote Rclone et le point de montage.");
      return;
    }

    const drive = {
      id: "d_" + Date.now().toString(36),
      label: label,
      remote_name: remote_name,
      provider_type: document.getElementById("provider_type").value,
      mount_point: mount_point,
      sync_local_dir: document.getElementById("sync_dir").value.trim(),
      sync_direction: document.getElementById("sync_direction").value,
      conflict_mode: document.getElementById("conflict_mode").value,
      auto_mount: document.getElementById("auto_mount").checked
    };

    try {
      await invoke("save_drive", { drive });
      modal.style.display = "none";
      // Réinitialiser les champs
      document.getElementById("label").value = "";
      document.getElementById("remote_name").value = "";
      document.getElementById("mount_point").value = "";
      loadDrives(invoke);
    } catch (e) {
      alert("Erreur lors de l'enregistrement: " + e);
    }
  };
});