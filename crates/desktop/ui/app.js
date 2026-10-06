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
const CATEGORIES = [
  { id: "todo", label: "Todo" },
  { id: "flujos", label: "Flujos de trabajo" },
  { id: "desplegar", label: "Desplegar" },
  { id: "desarrollo", label: "Desarrollo" },
  { id: "sistema", label: "Sistema" },
  { id: "guardados", label: "Mis guardados" },
];
const STEP_TYPES = [
  { id: "run", label: "Comando", placeholder: "docker compose up -d" },
  { id: "detach", label: "Abrir app", placeholder: "code ." },
  { id: "terminal", label: "Terminal", placeholder: "comando dentro (opcional)" },
  { id: "open", label: "Abrir enlace", placeholder: "http://localhost:3000" },
];
const ICON_CMD = '<svg viewBox="0 0 24 24" aria-hidden="true"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>';
const ICON_FLOW = '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="6" cy="6" r="2.5"/><circle cx="18" cy="18" r="2.5"/><path d="M8.5 6H15a3 3 0 0 1 0 6H9a3 3 0 0 0 0 6h6.5"/></svg>';

const state = {
  status: null,
  profiles: [],
  catalog: { actions: [], keys: { modifiers: [], keys: [], media: [] } },
  library: [],
  libCat: "todo",
  libQuery: "",
  id: null,
  profile: null,
  dirty: false,
  resolved: {},
  selected: null,
  edType: "keys",
  cmdDraft: null,
  flowDraft: null,
  pendingSwitch: null,
  tplOpen: null,
};

const $ = (sel) => document.querySelector(sel);
const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
const clone = (x) => JSON.parse(JSON.stringify(x));

// ---------- utilidades ----------

let toastTimer;
function toast(text, isError = false) {
  const el = $("#toast");
  el.textContent = text;
  el.classList.toggle("err", isError);
  el.classList.remove("hidden");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.add("hidden"), isError ? 6000 : 3500);
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

function controlLabel(id) {
  return CONTROLS.find((c) => c.id === id)?.label || id;
}

function findAction(value) {
  if (typeof value !== "string") return null;
  const v = value.trim().toLowerCase();
  return state.catalog.actions.find((a) => a.id === v || a.aliases.includes(v)) || null;
}

function isMediaValue(value) {
  if (typeof value !== "string") return false;
  const a = findAction(value);
  if (a) return a.category === "Multimedia" || a.category === "Desplazamiento";
  return state.catalog.keys.media.includes(value.trim().toLowerCase());
}

/** keys | media | command | flow | null */
function kindOf(binding) {
  if (binding === null || binding === undefined) return null;
  if (typeof binding === "string") return isMediaValue(binding) ? "media" : "keys";
  if (binding.flow) return "flow";
  if (binding.command !== undefined) return "command";
  return null;
}

function bindingName(binding) {
  const kind = kindOf(binding);
  if (kind === "command") return binding.label || binding.command;
  if (kind === "flow") return binding.label || `Flujo de ${binding.flow.length} pasos`;
  const a = findAction(binding);
  return a ? a.description : binding || "Sin asignar";
}

function automationsInProfile() {
  if (!state.profile) return 0;
  return Object.values(state.profile.bindings).filter((b) => ["command", "flow"].includes(kindOf(b))).length;
}

function formatTime(iso) {
  const d = new Date(iso);
  if (isNaN(d)) return iso;
  return d.toLocaleString("es", { day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit" });
}

function lastLine(text) {
  return String(text || "").split("\n").map((l) => l.trim()).filter(Boolean).pop() || "";
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

  const cmds = $("#st-cmds");
  cmds.classList.toggle("hidden", !s.automations);
  if (s.automations) {
    const n = s.automations === 1 ? "1 comando" : `${s.automations} comandos`;
    const map = {
      ok: [`Escuchando el pad · ${n}`, "amber"],
      denied: [`${n} sin permiso`, "bad"],
      no_device: [`${n} · pad desconectado`, "muted"],
      unsupported: [`${n} · escucha no disponible aquí`, "muted"],
    };
    const [text, cls] = map[s.input] || [n, "muted"];
    cmds.textContent = text;
    cmds.className = `pill ${cls}`;
    cmds.title = `Perfil activo: ${s.active_profile}`;
  }

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
  renderPermBanner();
}

function renderPermBanner() {
  const s = state.status;
  const wantsKeys = automationsInProfile() > 0 || ["command", "flow"].includes(state.selected && state.edType);
  const banner = $("#perm-banner");
  if (!s || !wantsKeys || s.input === "ok" || s.input === "no_device") {
    banner.classList.add("hidden");
    return;
  }
  banner.classList.remove("hidden");
  if (s.input === "unsupported") {
    $("#perm-title").textContent = "Este sistema aún no escucha las teclas del pad";
    $("#perm-body").textContent = "Puedes preparar y probar los comandos con «Probar»; se ejecutarán desde el pad cuando la escucha esté disponible.";
    $("#btn-grant").classList.add("hidden");
  } else {
    $("#perm-title").textContent = "El agente no puede leer las teclas del pad";
    $("#perm-body").textContent = "Los comandos se guardan, pero no se ejecutarán al pulsar las teclas hasta dar el permiso. Se pide tu contraseña una vez.";
    $("#btn-grant").classList.remove("hidden");
  }
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
  renderLibrary();
  await updatePreview();
}

function setBinding(control, value) {
  state.profile.bindings[control] = value === null || value === "" ? null : value;
  markDirty();
  renderPad();
  renderEditorHeader();
  renderPermBanner();
  renderLibrary();
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
  renderPermBanner();
}

function slotInfo(control) {
  const binding = state.profile.bindings[control.id];
  const kind = kindOf(binding);
  const auto = kind === "command" || kind === "flow";
  let sub;
  if (kind === "flow") sub = `flujo · ${binding.flow.length} ${binding.flow.length === 1 ? "paso" : "pasos"}`;
  else if (kind === "command") sub = `comando · ${binding.command}`;
  else sub = binding ? (state.resolved[control.id] || binding) : "—";
  return { kind, auto, title: bindingName(binding), sub, empty: !binding };
}

function renderPad() {
  $("#keys").innerHTML = CONTROLS.filter((c) => c.id.startsWith("key_")).map((c) => {
    const s = slotInfo(c);
    const tag = s.kind === "flow" ? `${ICON_FLOW}${c.label}` : s.kind === "command" ? `${ICON_CMD}${c.label}` : esc(c.label);
    return `<button class="key ${s.auto ? "auto" : ""} ${state.selected === c.id ? "selected" : ""}" data-control="${c.id}">
      <span class="slot-label">${tag}</span>
      <span class="slot-action">${esc(s.title)}</span>
      <span class="slot-value ${s.empty ? "empty" : ""}">${esc(s.sub)}</span>
    </button>`;
  }).join("");
  document.querySelectorAll(".knob-area [data-control]").forEach((el) => {
    const c = CONTROLS.find((x) => x.id === el.dataset.control);
    const s = slotInfo(c);
    el.classList.toggle("selected", state.selected === c.id);
    el.classList.toggle("auto", s.auto);
    const v = el.querySelector(".slot-value");
    v.textContent = s.empty ? "—" : s.auto ? s.title : s.sub;
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
  // Los controles sin asignar van en una sola línea.
  const unassigned = preview.warnings.filter((w) => w.endsWith("no tiene acción asignada")).map((w) => controlLabel(w.split(" ")[0]).toLowerCase());
  if (unassigned.length) msgs.push(`<div class="msg info">Sin asignar: ${esc(unassigned.join(", "))}.</div>`);
  for (const w of preview.warnings.filter((w) => !w.endsWith("no tiene acción asignada"))) msgs.push(`<div class="msg warn">${esc(w)}</div>`);
  if (state.dirty && !preview.error) msgs.push('<div class="msg info">Cambios sin guardar. Los comandos se activan al guardar.</div>');
  $("#messages").innerHTML = msgs.join("");
  $("#btn-save").disabled = !!preview.error;
  $("#btn-apply").disabled = !!preview.error;
  renderPad();
  renderEditorHeader();
}

async function saveCurrent() {
  const saved = await call("save_profile", { id: state.id, profile: state.profile }, { busy: $("#btn-save") });
  state.dirty = false;
  state.profiles = await call("list_profiles");
  renderProfiles();
  renderProfile();
  await updatePreview();
  refreshStatus();
  return saved;
}

// ---------- panel de edición ----------

function defaultCommand() {
  return { command: "", label: "", cwd: "", vars: {}, notify: true, terminal: false, single: true, confirm: false };
}

function defaultFlow() {
  return { flow: [{ terminal: "" }], label: "", cwd: "", vars: {}, notify: true, single: true, confirm: false, on_connect: false };
}

function openEditor(control) {
  state.selected = control;
  const binding = state.profile.bindings[control];
  const kind = kindOf(binding);
  state.edType = kind || "keys";
  state.cmdDraft = kind === "command" ? { ...defaultCommand(), ...clone(binding) } : defaultCommand();
  state.flowDraft = kind === "flow" ? { ...defaultFlow(), ...clone(binding) } : defaultFlow();
  $("#editor").classList.remove("hidden");
  $("#cmd-result").classList.add("hidden");
  $("#flow-result").classList.add("hidden");
  renderPad();
  renderEditor();
}

function closeEditor() {
  state.selected = null;
  $("#editor").classList.add("hidden");
  renderPad();
  renderPermBanner();
}

function setType(type) {
  state.edType = type;
  renderEditor();
  renderPermBanner();
}

function renderEditorHeader() {
  const control = CONTROLS.find((c) => c.id === state.selected);
  if (!control || !state.profile) return;
  const value = state.profile.bindings[control.id];
  const kind = kindOf(value);
  $("#ed-eyebrow").textContent = control.label;
  const kindLabel = { command: "comando", flow: "flujo" }[kind];
  $("#ed-current").textContent = kindLabel ? `${kindLabel}: ${bindingName(value)}` : value || "sin asignar";
  const resolved = state.resolved[control.id];
  $("#ed-resolved").textContent = kindLabel && resolved ? `→ envía ${resolved}` : resolved && resolved !== value ? `→ ${resolved}` : "";
  $("#ed-save-lib").classList.toggle("hidden", !kindLabel);
}

function renderEditor() {
  const control = CONTROLS.find((c) => c.id === state.selected);
  if (!control || !state.profile) return;
  renderEditorHeader();
  document.querySelectorAll("#ed-types .type").forEach((b) => b.setAttribute("aria-checked", String(b.dataset.type === state.edType)));
  document.querySelectorAll(".sec").forEach((s) => s.classList.toggle("hidden", s.dataset.sec !== state.edType));
  ({ keys: renderKeysSec, media: renderMediaSec, command: renderCommandSec, flow: renderFlowSec })[state.edType]();
}

function renderKeysSec() {
  const value = state.profile.bindings[state.selected];
  const platform = targetPlatform();
  const current = findAction(value);
  const groups = {};
  for (const a of state.catalog.actions) {
    if (a.category === "Multimedia" || a.category === "Desplazamiento") continue;
    (groups[a.category] ||= []).push(a);
  }
  $("#ed-common").innerHTML = Object.entries(groups).map(([cat, list]) => `
    <div class="group-title">${esc(cat)}</div>
    <div class="chips">${list.map((a) => `
      <button class="chip ${current?.id === a.id ? "active" : ""}" data-action="${esc(a.id)}" title="${esc(a.id)}">
        ${esc(a.description)} <code>${esc(a[platform])}</code>
      </button>`).join("")}
    </div>`).join("");

  // Precarga la combinación y el valor directo con lo asignado.
  const raw = typeof value === "string" ? (state.resolved[state.selected] || value) : "";
  const parts = raw.includes(",") ? [] : raw.split("-");
  const key = parts.length ? parts[parts.length - 1] : "";
  const mods = parts.slice(0, -1);
  $("#ed-mods").innerHTML = MODIFIERS.map((m) => `
    <label><input type="checkbox" value="${m}" ${mods.includes(m) || (m === "cmd" && mods.includes("win")) ? "checked" : ""}>
    ${esc(MOD_LABEL[platform]?.[m] || m)}</label>`).join("");
  $("#ed-key").innerHTML = '<option value="">—</option>' +
    state.catalog.keys.keys.map((k) => `<option value="${esc(k)}" ${k === key ? "selected" : ""}>${esc(k)}</option>`).join("");
  $("#ed-raw").value = typeof value === "string" ? value : "";
  updateCombo();
}

function renderMediaSec() {
  const value = state.profile.bindings[state.selected];
  const current = findAction(value);
  const actions = state.catalog.actions.filter((a) => a.category === "Multimedia" || a.category === "Desplazamiento");
  const known = new Set(actions.map((a) => a.linux));
  const extra = state.catalog.keys.media.filter((m) => !known.has(m));
  $("#ed-media").innerHTML = `
    <div class="chips">${actions.map((a) => `
      <button class="chip ${current?.id === a.id ? "active" : ""}" data-action="${esc(a.id)}">${esc(a.description)}</button>`).join("")}
      ${extra.map((m) => `<button class="chip ${value === m ? "active" : ""}" data-media="${esc(m)}">${esc(m)}</button>`).join("")}
    </div>`;
}

function renderCommandSec() {
  const d = state.cmdDraft;
  $("#cmd-label").value = d.label || "";
  $("#cmd-command").value = d.command || "";
  $("#cmd-cwd").value = d.cwd || "";
  $("#cmd-notify").checked = d.notify;
  $("#cmd-terminal").checked = d.terminal;
  $("#cmd-single").checked = d.single;
  $("#cmd-confirm").checked = d.confirm;
  const quick = state.library.filter((t) => t.category === "sistema" && t.binding.command).slice(0, 4);
  $("#cmd-quick").innerHTML = quick.map((t) =>
    `<button class="chip" data-quick="${esc(t.id)}" title="${esc(t.binding.command)}">${esc(t.title)}</button>`).join("") +
    '<button class="chip" data-goto-lib="1">Ver biblioteca…</button>';
}

function stepType(step) {
  if (step.open !== undefined && step.open !== null) return "open";
  if (step.terminal !== undefined && step.terminal !== null) return "terminal";
  return step.detach ? "detach" : "run";
}

function stepValue(step) {
  return step.open ?? step.terminal ?? step.run ?? "";
}

function makeStep(type, value, label, delay) {
  const step = {};
  if (label) step.label = label;
  if (type === "open") step.open = value;
  else if (type === "terminal") step.terminal = value;
  else {
    step.run = value;
    if (type === "detach") step.detach = true;
  }
  if (delay) step.delay_ms = delay;
  return step;
}

function renderFlowSec() {
  const d = state.flowDraft;
  $("#flow-label").value = d.label || "";
  $("#flow-cwd").value = d.cwd || "";
  $("#flow-connect").checked = d.on_connect;
  $("#flow-notify").checked = d.notify;
  $("#flow-single").checked = d.single;
  $("#flow-confirm").checked = d.confirm;
  const vars = Object.entries(d.vars || {});
  $("#flow-vars").innerHTML = vars.length ? `<div class="field"><span>Variables</span>${vars.map(([k, v]) => `
    <label class="field"><span class="mono">$${esc(k)}</span><input class="mono" data-var="${esc(k)}" value="${esc(v)}"></label>`).join("")}</div>` : "";
  renderSteps();
}

function renderSteps() {
  const steps = state.flowDraft.flow;
  $("#flow-steps").innerHTML = steps.map((step, i) => {
    const type = stepType(step);
    const meta = STEP_TYPES.find((t) => t.id === type);
    return `<li class="step" data-i="${i}">
      <span class="step-num">${i + 1}</span>
      <div class="step-body">
        <div class="step-top">
          <select data-field="type" aria-label="Tipo del paso ${i + 1}">${STEP_TYPES.map((t) =>
            `<option value="${t.id}" ${t.id === type ? "selected" : ""}>${t.label}</option>`).join("")}</select>
          <input data-field="label" placeholder="Nombre (opcional)" value="${esc(step.label || "")}" aria-label="Nombre del paso ${i + 1}">
        </div>
        <input class="mono" data-field="value" placeholder="${esc(meta.placeholder)}" value="${esc(stepValue(step))}" aria-label="Valor del paso ${i + 1}">
        <div class="step-tools">
          <button class="ghost" data-move="-1" ${i === 0 ? "disabled" : ""} aria-label="Subir paso ${i + 1}">↑</button>
          <button class="ghost" data-move="1" ${i === steps.length - 1 ? "disabled" : ""} aria-label="Bajar paso ${i + 1}">↓</button>
          <button class="ghost danger" data-remove="1" aria-label="Quitar paso ${i + 1}">✕</button>
        </div>
      </div>
    </li>`;
  }).join("");
}

/** Comando del borrador sin campos vacíos, listo para el perfil. */
function buildCommand() {
  const d = state.cmdDraft;
  const b = { command: d.command.trim() };
  if (d.label.trim()) b.label = d.label.trim();
  if (d.cwd.trim()) b.cwd = d.cwd.trim();
  if (Object.keys(d.vars || {}).length) b.vars = d.vars;
  for (const k of ["notify", "terminal", "single", "confirm"]) b[k] = d[k];
  return b;
}

function buildFlow() {
  const d = state.flowDraft;
  const b = { flow: d.flow.map((s) => clone(s)) };
  if (d.label.trim()) b.label = d.label.trim();
  if (d.cwd.trim()) b.cwd = d.cwd.trim();
  if (Object.keys(d.vars || {}).length) b.vars = d.vars;
  for (const k of ["notify", "single", "confirm", "on_connect"]) b[k] = d[k];
  return b;
}

function renderResult(el, outcome) {
  el.classList.remove("hidden");
  if (outcome === "running") {
    el.innerHTML = '<div class="result-head run"><span>Ejecutando…</span></div>';
    return;
  }
  const secs = (outcome.duration_ms / 1000).toFixed(1);
  const head = outcome.ok
    ? `✓ Terminó en ${secs} s${outcome.code !== null ? ` · código ${outcome.code}` : ""}`
    : `✗ Falló${outcome.code !== null ? ` · código ${outcome.code}` : ""} · ${secs} s`;
  el.innerHTML = `<div class="result-head ${outcome.ok ? "ok" : "bad"}"><span>${esc(head)}</span><span>ahora</span></div>` +
    (outcome.output ? `<pre>${esc(outcome.output)}</pre>` : "");
}

async function testDraft(binding, resultEl, button) {
  renderResult(resultEl, "running");
  try {
    const outcome = await call("test_binding", { profileId: state.id, control: state.selected, binding }, { busy: button });
    renderResult(resultEl, outcome);
  } catch {
    resultEl.classList.add("hidden");
  }
}

function updateCombo() {
  const v = comboValue();
  $("#ed-combo").textContent = v || "—";
  $("#ed-combo-use").disabled = !v;
}

function comboValue() {
  const key = $("#ed-key").value;
  if (!key) return "";
  const mods = [...document.querySelectorAll("#ed-mods input:checked")].map((i) => i.value);
  return [...mods, key].join("-");
}

// ---------- biblioteca ----------

async function loadLibrary() {
  try {
    state.library = await invoke("list_library");
  } catch (e) {
    state.library = [];
    toast(`Biblioteca: ${e}`, true);
  }
  renderLibrary();
}

/** Control del perfil que ya usa esta plantilla (misma etiqueta y tipo). */
function usedBy(template) {
  if (!state.profile) return null;
  const label = template.binding.label;
  const kind = kindOf(template.binding);
  return Object.entries(state.profile.bindings).find(([, b]) => kindOf(b) === kind && b.label && b.label === label)?.[0] || null;
}

function templateMatches(t) {
  const q = state.libQuery.trim().toLowerCase();
  if (!q) return true;
  const text = [t.title, t.description, t.binding.command, ...(t.binding.flow || []).map((s) => `${s.label || ""} ${stepValue(s)}`)].join(" ").toLowerCase();
  return text.includes(q);
}

function stepsHtml(binding) {
  if (binding.flow) {
    return `<ol>${binding.flow.map((s) => `<li>${esc(s.label || stepValue(s) || STEP_TYPES.find((t) => t.id === stepType(s)).label)}</li>`).join("")}</ol>`;
  }
  return `<code class="cmd">${esc(binding.command)}</code>`;
}

function cardHtml(t) {
  const used = usedBy(t);
  const badges = [];
  if (used) badges.push(`<span class="badge amber">En ${esc(controlLabel(used).toLowerCase())}</span>`);
  if (t.binding.confirm) badges.push('<span class="badge red">Pide confirmación</span>');
  if (t.binding.terminal) badges.push('<span class="badge">En terminal</span>');
  const needs = t.needs?.length ? `<div class="needs">Pide: ${esc(t.needs.map((n) => n.label.toLowerCase()).join(", "))}</div>` : "";
  const actions = used
    ? `<button data-edit="${esc(used)}">Editar</button>`
    : `<button class="primary" data-use="${esc(t.id)}">Usar en una tecla</button>`;
  const del = t.user ? `<button class="ghost danger" data-del="${esc(t.id)}" aria-label="Eliminar ${esc(t.title)}">Eliminar</button>` : "";
  return `<article class="card ${used ? "used" : ""}">
    <div class="card-head"><h3>${esc(t.title)}</h3><span>${badges.join(" ")}</span></div>
    ${t.description ? `<p>${esc(t.description)}</p>` : ""}
    ${stepsHtml(t.binding)}
    ${needs}
    <div class="card-actions">${actions}${del}</div>
  </article>`;
}

function renderLibrary() {
  const saved = state.library.filter((t) => t.category === "guardados").length;
  $("#lib-cats").innerHTML = CATEGORIES.filter((c) => c.id !== "guardados" || saved).map((c) =>
    `<button class="chip" role="tab" data-cat="${c.id}" aria-selected="${c.id === state.libCat}">${esc(c.label)}${c.id === "guardados" ? ` · ${saved}` : ""}</button>`).join("");
  const list = state.library.filter(templateMatches);
  const cats = state.libCat === "todo" ? CATEGORIES.filter((c) => c.id !== "todo") : CATEGORIES.filter((c) => c.id === state.libCat);
  const html = cats.map((c) => {
    const items = list.filter((t) => t.category === c.id);
    if (!items.length) return "";
    return `<section class="lib-section"><h2>${esc(c.label)}</h2><div class="lib-grid">${items.map(cardHtml).join("")}</div></section>`;
  }).join("");
  $("#lib-list").innerHTML = html || '<p class="lib-empty">No hay nada que coincida con la búsqueda.</p>';
}

function firstFreeControl() {
  const keys = CONTROLS.map((c) => c.id);
  return keys.find((k) => !state.profile.bindings[k]) || state.selected || "key_6";
}

function openTemplate(id) {
  const t = state.library.find((x) => x.id === id);
  if (!t) return;
  state.tplOpen = t;
  $("#tpl-cat").textContent = CATEGORIES.find((c) => c.id === t.category)?.label || "Biblioteca";
  $("#tpl-title").textContent = t.title;
  $("#tpl-desc").textContent = t.description || "";
  $("#tpl-preview").innerHTML = stepsHtml(t.binding);
  const target = state.selected || firstFreeControl();
  $("#tpl-control").innerHTML = CONTROLS.map((c) => {
    const b = state.profile.bindings[c.id];
    const now = b ? ` — ahora: ${bindingName(b)}` : " — libre";
    return `<option value="${c.id}" ${c.id === target ? "selected" : ""}>${esc(c.label + now)}</option>`;
  }).join("");
  $("#tpl-needs").innerHTML = (t.needs || []).map((n) => `
    <label class="field">${esc(n.label)}
      <input data-need="${esc(n.key)}" class="${n.kind === "folder" ? "mono" : ""}" value="${esc(n.default)}" required>
    </label>`).join("");
  $("#tpl-overlay").classList.remove("hidden");
  ($("#tpl-needs input") || $("#tpl-control")).focus();
}

function closeTemplate() {
  state.tplOpen = null;
  $("#tpl-overlay").classList.add("hidden");
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
  const mark = { ok: ["✓", "ok"], error: ["✗", "bad"], ignorada: ["–", ""] };
  $("#diag-runs").innerHTML = d.runs.map((r) => {
    const [m, cls] = mark[r.result] || ["?", ""];
    return `<tr><td class="${cls}">${m}</td><td>${esc(formatTime(r.timestamp))}</td><td>${esc(controlLabel(r.control))}</td>
      <td>${esc(r.label)}</td><td>${esc(r.trigger)}</td><td>${r.result === "ignorada" ? "—" : `${(r.duration_ms / 1000).toFixed(1)} s`}</td>
      <td class="${cls}">${esc(lastLine(r.output))}</td></tr>`;
  }).join("") || '<tr><td colspan="7" class="muted">Todavía no se ha ejecutado ningún comando.</td></tr>';
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

function showTab(name) {
  document.querySelectorAll(".tab").forEach((x) => x.classList.toggle("active", x.dataset.tab === name));
  for (const t of ["editor", "library", "diagnose"]) $(`#tab-${t}`).classList.toggle("hidden", t !== name);
  if (name !== "editor") closeEditor();
  if (name === "diagnose") runDiagnose();
  if (name === "library") renderLibrary();
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
    renderEditor();
  });

  // Panel: tipo y atajos
  $("#ed-close").addEventListener("click", closeEditor);
  $("#ed-types").addEventListener("click", (e) => {
    const b = e.target.closest("[data-type]");
    if (b) setType(b.dataset.type);
  });
  document.querySelector('[data-sec="keys"]').addEventListener("click", (e) => {
    const b = e.target.closest("[data-action]");
    if (b) { setBinding(state.selected, b.dataset.action); renderKeysSec(); }
  });
  $("#ed-media").addEventListener("click", (e) => {
    const b = e.target.closest("[data-action],[data-media]");
    if (b) { setBinding(state.selected, b.dataset.action || b.dataset.media); renderMediaSec(); }
  });
  $("#ed-mods").addEventListener("change", updateCombo);
  $("#ed-key").addEventListener("change", updateCombo);
  $("#ed-combo-use").addEventListener("click", () => setBinding(state.selected, comboValue()));
  $("#ed-raw-use").addEventListener("click", () => setBinding(state.selected, $("#ed-raw").value.trim()));
  $("#ed-raw").addEventListener("keydown", (e) => {
    if (e.key === "Enter") setBinding(state.selected, $("#ed-raw").value.trim());
  });
  $("#ed-clear").addEventListener("click", () => {
    setBinding(state.selected, null);
    openEditor(state.selected);
  });
  $("#ed-save-lib").addEventListener("click", async () => {
    const b = state.profile.bindings[state.selected];
    const t = await call("save_to_library", { title: bindingName(b), description: "", binding: b }, { busy: $("#ed-save-lib") });
    toast(`«${t.title}» guardado en la biblioteca`);
    loadLibrary();
  });

  // Panel: comando
  const cmdFields = { "#cmd-label": "label", "#cmd-command": "command", "#cmd-cwd": "cwd" };
  for (const [sel, key] of Object.entries(cmdFields)) {
    $(sel).addEventListener("input", (e) => { state.cmdDraft[key] = e.target.value; });
  }
  for (const k of ["notify", "terminal", "single", "confirm"]) {
    $(`#cmd-${k}`).addEventListener("change", (e) => { state.cmdDraft[k] = e.target.checked; });
  }
  $("#cmd-test").addEventListener("click", () => {
    if (!state.cmdDraft.command.trim()) return toast("Escribe el comando primero", true);
    testDraft(buildCommand(), $("#cmd-result"), $("#cmd-test"));
  });
  $("#cmd-use").addEventListener("click", () => {
    if (!state.cmdDraft.command.trim()) return toast("Escribe el comando primero", true);
    setBinding(state.selected, buildCommand());
    toast(`${controlLabel(state.selected)}: comando asignado. Guarda para activarlo.`);
  });
  $("#cmd-quick").addEventListener("click", (e) => {
    if (e.target.closest("[data-goto-lib]")) return showTab("library");
    const b = e.target.closest("[data-quick]");
    const t = b && state.library.find((x) => x.id === b.dataset.quick);
    if (!t) return;
    state.cmdDraft = { ...defaultCommand(), ...clone(t.binding) };
    renderCommandSec();
  });

  // Panel: flujo
  $("#flow-label").addEventListener("input", (e) => { state.flowDraft.label = e.target.value; });
  $("#flow-cwd").addEventListener("input", (e) => { state.flowDraft.cwd = e.target.value; });
  const flowChecks = { connect: "on_connect", notify: "notify", single: "single", confirm: "confirm" };
  for (const [id, key] of Object.entries(flowChecks)) {
    $(`#flow-${id}`).addEventListener("change", (e) => { state.flowDraft[key] = e.target.checked; });
  }
  $("#flow-vars").addEventListener("input", (e) => {
    const k = e.target.dataset.var;
    if (k) state.flowDraft.vars[k] = e.target.value;
  });
  $("#flow-steps").addEventListener("input", (e) => {
    const li = e.target.closest(".step");
    if (!li) return;
    const i = Number(li.dataset.i);
    const step = state.flowDraft.flow[i];
    const field = e.target.dataset.field;
    if (field === "label") {
      if (e.target.value) step.label = e.target.value; else delete step.label;
    } else if (field === "value") {
      state.flowDraft.flow[i] = makeStep(stepType(step), e.target.value, step.label, step.delay_ms);
    }
  });
  $("#flow-steps").addEventListener("change", (e) => {
    if (e.target.dataset.field !== "type") return;
    const i = Number(e.target.closest(".step").dataset.i);
    const step = state.flowDraft.flow[i];
    state.flowDraft.flow[i] = makeStep(e.target.value, stepValue(step), step.label, step.delay_ms);
    renderSteps();
  });
  $("#flow-steps").addEventListener("click", (e) => {
    const li = e.target.closest(".step");
    if (!li) return;
    const i = Number(li.dataset.i);
    const steps = state.flowDraft.flow;
    const move = e.target.closest("[data-move]");
    if (move) {
      const j = i + Number(move.dataset.move);
      [steps[i], steps[j]] = [steps[j], steps[i]];
      renderSteps();
    } else if (e.target.closest("[data-remove]")) {
      steps.splice(i, 1);
      renderSteps();
    }
  });
  $("#flow-add").addEventListener("click", () => {
    state.flowDraft.flow.push({ run: "" });
    renderSteps();
    const inputs = document.querySelectorAll('#flow-steps [data-field="value"]');
    inputs[inputs.length - 1]?.focus();
  });
  $("#flow-lib").addEventListener("click", () => {
    state.libCat = "flujos";
    showTab("library");
  });
  const flowProblem = () => {
    const steps = state.flowDraft.flow;
    if (!steps.length) return "Agrega al menos un paso";
    const empty = steps.findIndex((s) => stepType(s) !== "terminal" && !stepValue(s).trim());
    return empty >= 0 ? `El paso ${empty + 1} está vacío` : null;
  };
  $("#flow-test").addEventListener("click", () => {
    const problem = flowProblem();
    if (problem) return toast(problem, true);
    testDraft(buildFlow(), $("#flow-result"), $("#flow-test"));
  });
  $("#flow-use").addEventListener("click", () => {
    const problem = flowProblem();
    if (problem) return toast(problem, true);
    setBinding(state.selected, buildFlow());
    toast(`${controlLabel(state.selected)}: flujo asignado. Guarda para activarlo.`);
  });

  // Permiso
  $("#btn-grant").addEventListener("click", async () => {
    await call("grant_input_permission", undefined, { busy: $("#btn-grant") });
    try { await invoke("restart_agent"); } catch { /* el agente puede no estar instalado */ }
    toast("Permiso concedido. El agente ya escucha el pad.");
    setTimeout(refreshStatus, 1500);
  });

  // Acciones del perfil
  $("#btn-save").addEventListener("click", async () => {
    const saved = await saveCurrent();
    if (saved.reapplied) toast("Guardado y teclado reprogramado: las teclas ya envían sus nuevas señales");
    else if (saved.reapply_error) toast(`Guardado. Falta reprogramar el teclado: ${saved.reapply_error}`, true);
    else toast("Perfil guardado");
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
    const profile = { ...clone(state.profile), name: $("#new-name").value.trim() };
    await call("save_profile", { id, profile });
    $("#new-form").classList.add("hidden");
    state.dirty = false;
    await loadProfiles(id);
    toast("Perfil creado");
  });

  // Biblioteca
  $("#lib-search").addEventListener("input", (e) => { state.libQuery = e.target.value; renderLibrary(); });
  $("#lib-cats").addEventListener("click", (e) => {
    const b = e.target.closest("[data-cat]");
    if (b) { state.libCat = b.dataset.cat; renderLibrary(); }
  });
  $("#lib-list").addEventListener("click", async (e) => {
    const use = e.target.closest("[data-use]");
    const edit = e.target.closest("[data-edit]");
    const del = e.target.closest("[data-del]");
    if (use) openTemplate(use.dataset.use);
    else if (edit) { showTab("editor"); openEditor(edit.dataset.edit); }
    else if (del) {
      await call("delete_from_library", { id: del.dataset.del });
      toast("Eliminado de la biblioteca");
      loadLibrary();
    }
  });
  $("#tpl-close").addEventListener("click", closeTemplate);
  $("#tpl-cancel").addEventListener("click", closeTemplate);
  $("#tpl-overlay").addEventListener("click", (e) => { if (e.target.id === "tpl-overlay") closeTemplate(); });
  $("#tpl-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const t = state.tplOpen;
    const values = {};
    document.querySelectorAll("#tpl-needs [data-need]").forEach((i) => { values[i.dataset.need] = i.value.trim(); });
    const binding = await call("use_template", { id: t.id, values });
    const control = $("#tpl-control").value;
    closeTemplate();
    setBinding(control, binding);
    showTab("editor");
    openEditor(control);
    toast(`«${t.title}» asignado a ${controlLabel(control).toLowerCase()}. Guarda para activarlo.`);
  });

  // Pestañas y diagnóstico
  document.querySelectorAll(".tab").forEach((t) => t.addEventListener("click", () => showTab(t.dataset.tab)));
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
    if (e.key === "Escape") {
      if (state.tplOpen) closeTemplate(); else closeEditor();
    }
    const tabKeys = { 1: "editor", 2: "library", 3: "diagnose" };
    if ((e.ctrlKey || e.metaKey) && tabKeys[e.key]) {
      e.preventDefault();
      showTab(tabKeys[e.key]);
    }
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
  await loadLibrary();
  await loadProfiles();
  setInterval(refreshStatus, 3000);
}

init();
