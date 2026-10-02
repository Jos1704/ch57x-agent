"use strict";

const invoke = window.__TAURI__.core.invoke;

const CONTROLS = [
  { id: "key_1", label: "Tecla 1" },
  { id: "key_2", label: "Tecla 2" },
  { id: "key_3", label: "Tecla 3" },
  { id: "key_4", label: "Tecla 4" },
  { id: "key_5", label: "Tecla 5" },
  { id: "key_6", label: "Tecla 6" },
  { id: "knob_left", label: "Perilla: girar a la izquierda" },
  { id: "knob_press", label: "Perilla: pulsar" },
  { id: "knob_right", label: "Perilla: girar a la derecha" },
];
const MODIFIERS = ["ctrl", "shift", "alt", "cmd"];
const MOD_LABEL = { linux: { cmd: "super" }, macos: { alt: "opt" } };

const state = {
  status: null,
  profiles: [],
  catalog: { actions: [], keys: { modifiers: [], keys: [], media: [] } },
  id: null,
  profile: null,
  dirty: false,
  resolved: {},
  selected: null,
  pendingSwitch: null,
};

const $ = (sel) => document.querySelector(sel);
const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

// ---------- utilidades ----------

let toastTimer;
function toast(text, isError = false) {
  const el = $("#toast");
  el.textContent = text;
  el.classList.toggle("err", isError);
  el.classList.remove("hidden");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.add("hidden"), isError ? 6000 : 3000);
}

async function call(cmd, args, { busy } = {}) {
  if (busy) busy.disabled = true;
  try {
    return await invoke(cmd, args);
  } catch (e) {
    toast(String(e), true);
    throw e;
  } finally {
    if (busy) busy.disabled = false;
  }
}

/** Botón que pide un segundo clic para confirmar. */
function confirmClick(button, label, action) {
  button.addEventListener("click", async () => {
    if (!button.classList.contains("confirm")) {
      const original = button.textContent;
      button.classList.add("confirm");
      button.textContent = label;
      setTimeout(() => {
        button.classList.remove("confirm");
        button.textContent = original;
      }, 3000);
      return;
    }
    button.classList.remove("confirm");
    await action();
  });
}

function targetPlatform() {
  const p = state.profile?.platform;
  if (p === "linux" || p === "macos") return p;
  return state.status?.platform === "macos" ? "macos" : "linux";
}

function findAction(value) {
  if (!value) return null;
  const v = value.trim().toLowerCase();
  return state.catalog.actions.find((a) => a.id === v || a.aliases.includes(v)) || null;
}

// ---------- estado ----------

async function refreshStatus() {
  let s;
  try {
    s = await invoke("get_status");
  } catch (e) {
    $("#st-device").textContent = "Error de estado";
    $("#st-device").className = "pill bad";
    return;
  }
  const prevAuto = state.status?.auto_profile;
  state.status = s;

  const dev = $("#st-device");
  if (!s.connected) {
    dev.textContent = "Macro pad desconectado";
    dev.className = "pill warn";
  } else if (s.access === "ok") {
    dev.textContent = "Macro pad conectado";
    dev.className = "pill ok";
  } else {
    dev.textContent = s.access === "denied" ? "Macro pad sin permisos" : "Macro pad con error";
    dev.className = "pill bad";
    dev.title = s.access === "denied" ? "Falta la regla udev" : s.access;
  }

  const agent = $("#st-agent");
  agent.textContent = s.agent_active === true ? "Agente activo" : s.agent_active === false ? "Agente detenido" : "Agente: desconocido";
  agent.className = "pill " + (s.agent_active ? "ok" : "warn");

  const last = $("#st-last");
  if (s.last_failure && (!s.last_applied || s.last_failure.timestamp > s.last_applied.timestamp)) {
    last.textContent = `Falló «${s.last_failure.profile}»`;
    last.className = "pill bad";
    last.title = s.last_failure.message;
  } else if (s.last_applied) {
    last.textContent = `Aplicado: ${s.last_applied.profile} · ${formatTime(s.last_applied.timestamp)}`;
    last.className = "pill muted";
    last.title = `${s.last_applied.trigger} — ${s.last_applied.timestamp}`;
  } else {
    last.textContent = "Ningún perfil aplicado";
    last.className = "pill muted";
  }

  if (prevAuto !== s.auto_profile) renderProfiles();
}

function formatTime(iso) {
  const d = new Date(iso);
  if (isNaN(d)) return iso;
  return d.toLocaleString("es", { day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit" });
}

// ---------- perfiles ----------

async function loadProfiles(selectId) {
  state.profiles = await call("list_profiles");
  const auto = state.status?.auto_profile;
  const id = selectId || state.id || auto || state.profiles[0]?.id;
  renderProfiles();
  if (id) await openProfile(id, true);
}

function renderProfiles() {
  const auto = state.status?.auto_profile;
  const platLabel = { linux: "Linux", macos: "macOS", any: "Cualquiera" };
  $("#profiles").innerHTML = state.profiles.map((p) => {
    const badges = [];
    if (p.id === auto) badges.push('<span class="badge auto">automático</span>');
    if (p.error) badges.push('<span class="badge err">error</span>');
    else if (p.modified) badges.push('<span class="badge">modificado</span>');
    else if (p.builtin) badges.push('<span class="badge">incluido</span>');
    return `<li data-id="${esc(p.id)}" class="${p.id === state.id ? "active" : ""}" title="${esc(p.error || p.id)}">
      <div class="p-name">${esc(p.name)}</div>
      <div class="p-meta">${esc(platLabel[p.platform] || "—")} ${badges.join(" ")}</div>
    </li>`;
  }).join("");
}

async function openProfile(id, force = false) {
  if (state.dirty && !force && state.pendingSwitch !== id) {
    state.pendingSwitch = id;
    toast("Hay cambios sin guardar. Haz clic de nuevo para descartarlos.");
    setTimeout(() => { if (state.pendingSwitch === id) state.pendingSwitch = null; }, 3000);
    return;
  }
  state.pendingSwitch = null;
  const profile = await call("get_profile", { id });
  state.id = id;
  state.profile = profile;
  state.dirty = false;
  closeEditor();
  renderProfiles();
  renderProfile();
  await updatePreview();
}

function setBinding(control, value) {
  state.profile.bindings[control] = value === null || value === "" ? null : value;
  markDirty();
  renderPad();
  renderEditor();
  schedulePreview();
}

function markDirty() {
  state.dirty = true;
  $("#btn-save").textContent = "Guardar cambios";
  $("#btn-apply").textContent = "Guardar y aplicar";
}

function renderProfile() {
  const p = state.profile;
  const meta = state.profiles.find((x) => x.id === state.id);
  $("#p-name").value = p.name;
  $("#p-platform").value = p.platform;
  $("#p-id").textContent = state.id;
  $("#btn-save").textContent = "Guardar";
  $("#btn-apply").textContent = "Aplicar ahora";
  const deletable = meta && (!meta.builtin || meta.modified);
  $("#btn-delete").classList.toggle("hidden", !deletable);
  $("#btn-delete").textContent = meta?.modified ? "Descartar modificaciones" : "Eliminar";
  $("#btn-auto").disabled = state.status?.auto_profile === state.id;
  renderPad();
}

function slotHtml(control) {
  const value = state.profile.bindings[control.id];
  const action = findAction(value);
  const resolved = state.resolved[control.id];
  const short = control.id.startsWith("key_") ? control.label : null;
  const title = action ? action.description : value || "Sin asignar";
  const code = value ? (resolved || value) : "—";
  return { short, title, code, empty: !value };
}

function renderPad() {
  $("#keys").innerHTML = CONTROLS.filter((c) => c.id.startsWith("key_")).map((c) => {
    const s = slotHtml(c);
    return `<button class="key ${state.selected === c.id ? "selected" : ""}" data-control="${c.id}">
      <span class="slot-label">${esc(s.short)}</span>
      <span class="slot-action">${esc(s.title)}</span>
      <span class="slot-value ${s.empty ? "empty" : ""}">${esc(s.code)}</span>
    </button>`;
  }).join("");
  document.querySelectorAll(".knob-area [data-control]").forEach((el) => {
    const c = CONTROLS.find((x) => x.id === el.dataset.control);
    const s = slotHtml(c);
    el.classList.toggle("selected", state.selected === c.id);
    const v = el.querySelector(".slot-value");
    v.textContent = s.empty ? "—" : s.code;
    v.classList.toggle("empty", s.empty);
    el.title = `${c.label}: ${s.title}`;
  });
}

let previewTimer;
function schedulePreview() {
  clearTimeout(previewTimer);
  previewTimer = setTimeout(updatePreview, 250);
}

async function updatePreview() {
  if (!state.profile) return;
  let preview;
  try {
    preview = await invoke("preview_profile", { profile: state.profile });
  } catch (e) {
    preview = { resolved: [], warnings: [], error: String(e) };
  }
  state.resolved = Object.fromEntries(preview.resolved);
  const msgs = [];
  if (preview.error) msgs.push(`<div class="msg err">${esc(preview.error)}</div>`);
  for (const w of preview.warnings) msgs.push(`<div class="msg warn">${esc(w)}</div>`);
  if (state.dirty && !preview.error) msgs.push('<div class="msg info">Cambios sin guardar.</div>');
  $("#messages").innerHTML = msgs.join("");
  $("#btn-save").disabled = !!preview.error;
  $("#btn-apply").disabled = !!preview.error;
  renderPad();
  renderEditor();
}

async function saveCurrent() {
  await call("save_profile", { id: state.id, profile: state.profile }, { busy: $("#btn-save") });
  state.dirty = false;
  state.profiles = await call("list_profiles");
  renderProfiles();
  renderProfile();
  await updatePreview();
}

// ---------- panel de edición ----------

function openEditor(control) {
  state.selected = control;
  $("#editor").classList.remove("hidden");
  renderPad();
  renderEditor(true);
}

function closeEditor() {
  state.selected = null;
  $("#editor").classList.add("hidden");
  renderPad();
}

function renderEditor(reset = false) {
  const control = CONTROLS.find((c) => c.id === state.selected);
  if (!control || !state.profile) return;
  const value = state.profile.bindings[control.id];
  const platform = targetPlatform();
  const isKnob = control.id.startsWith("knob");

  $("#ed-eyebrow").textContent = isKnob ? "Perilla" : "Tecla";
  $("#ed-title").textContent = control.label;
  $("#ed-current").textContent = value || "sin asignar";
  const resolved = state.resolved[control.id];
  $("#ed-resolved").textContent = resolved && resolved !== value ? `→ ${resolved}` : "";

  // Acciones comunes agrupadas por categoría.
  const groups = {};
  for (const a of state.catalog.actions) (groups[a.category] ||= []).push(a);
  const current = findAction(value);
  $("#ed-common").innerHTML = Object.entries(groups).map(([cat, list]) => `
    <div class="group-title">${esc(cat)}</div>
    <div class="chips">${list.map((a) => `
      <button class="chip ${current?.id === a.id ? "active" : ""}" data-action="${esc(a.id)}" title="${esc(a.id)}">
        ${esc(a.description)} <code>${esc(a[platform])}</code>
      </button>`).join("")}
    </div>`).join("");

  $("#ed-media").innerHTML = state.catalog.keys.media.map((m) =>
    `<button class="chip ${value === m ? "active" : ""}" data-media="${esc(m)}">${esc(m)}</button>`).join("");

  if (reset) {
    // Precarga la combinación y el valor directo con lo asignado.
    const raw = resolved || value || "";
    const parts = raw.includes(",") ? [] : raw.split("-");
    const key = parts.length ? parts[parts.length - 1] : "";
    const mods = parts.slice(0, -1);
    $("#ed-mods").innerHTML = MODIFIERS.map((m) => `
      <label><input type="checkbox" value="${m}" ${mods.includes(m) || (m === "cmd" && mods.includes("win")) ? "checked" : ""}>
      ${esc(MOD_LABEL[platform]?.[m] || m)}</label>`).join("");
    const keys = state.catalog.keys.keys;
    $("#ed-key").innerHTML = '<option value="">—</option>' +
      keys.map((k) => `<option value="${esc(k)}" ${k === key ? "selected" : ""}>${esc(k)}</option>`).join("");
    $("#ed-raw").value = value || "";
    updateCombo();
  }
}

function comboValue() {
  const key = $("#ed-key").value;
  if (!key) return "";
  const mods = [...document.querySelectorAll("#ed-mods input:checked")].map((i) => i.value);
  return [...mods, key].join("-");
}

function updateCombo() {
  $("#ed-combo").textContent = comboValue() || "—";
  $("#ed-combo-use").disabled = !comboValue();
}

// ---------- diagnóstico ----------

let diagnosisText = "";
async function runDiagnose() {
  const d = await call("run_diagnose", undefined, { busy: $("#btn-diag") });
  diagnosisText = d.text;
  let section = "";
  const icon = { ok: "✓", error: "✗", info: "·" };
  $("#diag-checks").innerHTML = d.report.checks.map((c) => {
    const head = c.section !== section ? `<div class="check-section">${esc((section = c.section))}</div>` : "";
    return `${head}<div class="check ${c.level}"><span class="icon">${icon[c.level]}</span><span>${esc(c.message)}</span></div>`;
  }).join("");
  $("#diag-history").innerHTML = d.history.map((r) => `
    <tr><td class="${r.ok ? "ok" : "bad"}">${r.ok ? "✓" : "✗"}</td>
    <td>${esc(formatTime(r.timestamp))}</td><td>${esc(r.profile)}</td><td>${esc(r.trigger)}</td><td>${esc(r.message)}</td></tr>`).join("")
    || '<tr><td colspan="5" class="muted">Sin intentos registrados.</td></tr>';
}

async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const ta = document.createElement("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    ta.remove();
  }
}

// ---------- eventos ----------

function bind() {
  $("#profiles").addEventListener("click", (e) => {
    const li = e.target.closest("li[data-id]");
    if (li && li.dataset.id !== state.id) openProfile(li.dataset.id);
  });

  $("#pad").addEventListener("click", (e) => {
    const el = e.target.closest("[data-control]");
    if (el) openEditor(el.dataset.control);
  });

  $("#p-name").addEventListener("input", (e) => {
    state.profile.name = e.target.value;
    markDirty();
    schedulePreview();
  });
  $("#p-platform").addEventListener("change", (e) => {
    state.profile.platform = e.target.value;
    markDirty();
    schedulePreview();
    renderEditor(true);
  });

  $("#ed-close").addEventListener("click", closeEditor);
  $("#ed-tabs").addEventListener("click", (e) => {
    const b = e.target.closest(".subtab");
    if (!b) return;
    document.querySelectorAll(".subtab").forEach((x) => x.classList.toggle("active", x === b));
    document.querySelectorAll(".sub").forEach((x) => x.classList.toggle("hidden", x.dataset.sub !== b.dataset.sub));
  });
  $("#ed-common").addEventListener("click", (e) => {
    const b = e.target.closest("[data-action]");
    if (b) setBinding(state.selected, b.dataset.action);
  });
  $("#ed-media").addEventListener("click", (e) => {
    const b = e.target.closest("[data-media]");
    if (b) setBinding(state.selected, b.dataset.media);
  });
  $("#ed-mods").addEventListener("change", updateCombo);
  $("#ed-key").addEventListener("change", updateCombo);
  $("#ed-combo-use").addEventListener("click", () => setBinding(state.selected, comboValue()));
  $("#ed-raw-use").addEventListener("click", () => setBinding(state.selected, $("#ed-raw").value.trim()));
  $("#ed-raw").addEventListener("keydown", (e) => {
    if (e.key === "Enter") setBinding(state.selected, $("#ed-raw").value.trim());
  });
  $("#ed-clear").addEventListener("click", () => setBinding(state.selected, null));

  $("#btn-save").addEventListener("click", async () => {
    await saveCurrent();
    toast("Perfil guardado");
  });

  $("#btn-apply").addEventListener("click", async () => {
    if (state.dirty) await saveCurrent();
    const name = await call("apply_profile", { id: state.id }, { busy: $("#btn-apply") });
    toast(`Perfil «${name}» aplicado`);
    refreshStatus();
  });

  $("#btn-auto").addEventListener("click", async () => {
    if (state.dirty) await saveCurrent();
    await call("set_auto_profile", { id: state.id }, { busy: $("#btn-auto") });
    toast("El agente aplicará este perfil al conectar el macro pad");
    await refreshStatus();
    renderProfile();
  });

  confirmClick($("#btn-restore"), "¿Restaurar? Clic de nuevo", async () => {
    const msg = await call("restore_development", undefined, { busy: $("#btn-restore") });
    toast(msg);
    await refreshStatus();
    await loadProfiles(state.status.auto_profile);
  });

  confirmClick($("#btn-delete"), "¿Seguro? Clic de nuevo", async () => {
    const meta = state.profiles.find((x) => x.id === state.id);
    await call("delete_profile", { id: state.id });
    toast(meta?.modified ? "Se restauró la versión incluida" : "Perfil eliminado");
    await loadProfiles(meta?.modified ? state.id : state.status?.auto_profile);
  });

  $("#btn-new").addEventListener("click", () => {
    $("#new-form").classList.remove("hidden");
    $("#new-name").value = state.profile ? `${state.profile.name} (copia)` : "";
    $("#new-id").value = "";
    $("#new-id").focus();
  });
  $("#new-cancel").addEventListener("click", () => $("#new-form").classList.add("hidden"));
  $("#new-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const id = $("#new-id").value.trim();
    if (state.profiles.some((p) => p.id === id)) {
      toast(`Ya existe un perfil «${id}»`, true);
      return;
    }
    const profile = { ...structuredClone(state.profile), name: $("#new-name").value.trim() };
    await call("save_profile", { id, profile });
    $("#new-form").classList.add("hidden");
    state.dirty = false;
    await loadProfiles(id);
    toast("Perfil creado");
  });

  document.querySelectorAll(".tab").forEach((t) => t.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((x) => x.classList.toggle("active", x === t));
    $("#tab-editor").classList.toggle("hidden", t.dataset.tab !== "editor");
    $("#tab-diagnose").classList.toggle("hidden", t.dataset.tab !== "diagnose");
    if (t.dataset.tab === "diagnose") { closeEditor(); runDiagnose(); }
  }));
  $("#btn-diag").addEventListener("click", runDiagnose);
  $("#btn-copy").addEventListener("click", async () => {
    await copyText(diagnosisText);
    toast("Diagnóstico copiado");
  });
  $("#btn-agent").addEventListener("click", async () => {
    await call("restart_agent", undefined, { busy: $("#btn-agent") });
    toast("Agente reiniciado");
    setTimeout(refreshStatus, 1000);
  });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") closeEditor();
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      if (!$("#btn-save").disabled) $("#btn-save").click();
    }
  });
}

async function init() {
  bind();
  state.catalog = await call("get_catalog");
  await refreshStatus();
  await loadProfiles();
  setInterval(refreshStatus, 3000);
}

init();
