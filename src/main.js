const { invoke } = window.__TAURI__.core;

async function loadDrives() {
  const container = document.getElementById("drive-list");
  container.innerHTML = "Chargement...";
  const drives = await invoke("get_drives");
  container.innerHTML = "";

  for (const d of drives) {
    const isMounted = await invoke("check_mount", { mountPoint: d.mount_point });
    const div = document.createElement("div");
    div.className = "card";
    div.innerHTML = `
      <div>
        <strong>${d.label}</strong> [<code>${d.mount_point}</code>]
        <br><small>${d.provider_type} (${d.remote_name})</small>
      </div>
      <div>
        <button class="${isMounted ? 'btn-green' : 'btn-red'}" onclick='toggleMount(${JSON.stringify(d)})'>
          ${isMounted ? 'Démonter (ON)' : 'Monter (OFF)'}
        </button>
        <button class="btn-blue" onclick='syncDrive(${JSON.stringify(d)})'>Synchroniser</button>
        <button onclick='deleteDrive("${d.id}")'>Supprimer</button>
      </div>
    `;
    container.appendChild(div);
  }
}

window.toggleMount = async (drive) => {
  await invoke("toggle_mount", { drive });
  loadDrives();
};

window.syncDrive = async (drive) => {
  try {
    alert("Démarrage de la synchronisation...");
    const res = await invoke("run_sync", { drive });
    alert(res);
  } catch (err) {
    alert("Erreur: " + err);
  }
};

window.deleteDrive = async (id) => {
  if (confirm("Supprimer ce disque ?")) {
    await invoke("delete_drive", { id });
    loadDrives();
  }
};

window.openModal = () => document.getElementById("modal").style.display = "flex";
window.closeModal = () => document.getElementById("modal").style.display = "none";

window.submitDrive = async () => {
  const drive = {
    id: crypto.randomUUID().slice(0, 8),
    label: document.getElementById("label").value,
    remote_name: document.getElementById("remote_name").value,
    provider_type: "Cloud",
    mount_point: document.getElementById("mount_point").value,
    sync_local_dir: document.getElementById("sync_dir").value,
    sync_direction: document.getElementById("sync_direction").value,
    conflict_mode: document.getElementById("conflict_mode").value,
    auto_mount: document.getElementById("auto_mount").checked
  };
  await invoke("save_drive", { drive });
  closeModal();
  loadDrives();
};

window.addEventListener("DOMContentLoaded", loadDrives);