import type { BleDevice } from "@mnlphlp/plugin-blec";
import { invoke } from "@tauri-apps/api/core";
import { decode } from "cbor-x";

import { bytes, count, dataRate, duration, percent, rate, temperatures, uptime } from "./format";
import {
  POWER_OFF_COMMAND,
  POWER_OFF_UUID,
  SERVICE_UUID,
  STREAM_UUID,
  SnapshotStream,
  decodeSnapshot,
  type Snapshot,
} from "./protocol";
import "./style.css";

type Connection = "disconnected" | "scanning" | "connecting" | "awaiting" | "connected" | "shutting_down";
type Bluetooth = typeof import("@mnlphlp/plugin-blec");

const app = element("#app");
const amaruLogo = new URL("../../../amaru.svg", import.meta.url).href;
const hasTauriRuntime = "__TAURI_INTERNALS__" in window;
const RESET_TIMEOUT_MS = 1_000;
const SCAN_TIMEOUT_MS = 10_000;
const SIGNAL_TIMEOUT_MS = 2_000;
const POWER_OFF_SLIDE_DISTANCE_PX = 32;
const POWER_OFF_HANDLE_START = [48, 228, 161];
const POWER_OFF_HANDLE_END = [49, 130, 243];

const devices = new Map<string, BleDevice>();
const stream = new SnapshotStream();
let bluetooth: Bluetooth | null = null;
let connection: Connection = "disconnected";
let snapshot: Snapshot | null = null;
let lastPayloadAt: number | null = null;
let lastTipHash: string | null = null;
let lastTipUpdateAt: number | null = null;
let selectedAddress: string | null = null;
let error: string | null = null;
let confirmingPowerOff = false;
let powerOffSlideStartX: number | null = null;
let powerOffSlideMoved = false;
let initialDiscoveryPending = hasTauriRuntime;
let resuming = false;
let connectionAttempt = 0;

void initialise();
render();
document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") void resume();
});
window.setInterval(() => {
  if (snapshot !== null) render();
}, 1_000);

async function initialise(): Promise<void> {
  if (!hasTauriRuntime) {
    render();
    return;
  }

  try {
    bluetooth = await import("@mnlphlp/plugin-blec");
    await bluetooth.getScanningUpdates((scanning) => {
      if (connection === "disconnected" || connection === "scanning") {
        connection = scanning ? "scanning" : "disconnected";
        render();
      }
    });
    void discover(true);
  } catch (cause) {
    initialDiscoveryPending = false;
    error = message(cause);
    render();
  }
}

async function discover(connectFirst: boolean): Promise<void> {
  if (connection === "scanning" || connection === "connecting" || connection === "awaiting" || connection === "connected") {
    return;
  }

  try {
    const ble = bluetoothApi();
    error = null;
    if (!(await ble.checkPermissions(true))) {
      throw new Error("Bluetooth permission is required to find Amaru nodes");
    }
    devices.clear();
    connection = "scanning";
    render();
    let foundAmaru = false;
    let resolveDiscovery = () => {};
    const discovery = new Promise<void>((resolve) => {
      resolveDiscovery = resolve;
    });
    await ble.startScan((found) => {
      if (foundAmaru) return;

      const device = found.find(isAmaru);
      if (device === undefined) return;

      foundAmaru = true;
      devices.set(device.address, device);
      initialDiscoveryPending = false;
      resolveDiscovery();
      if (connectFirst) {
        void attach(device.address);
      } else {
        render();
        void stopScanAfterDiscovery(ble);
      }
    }, SCAN_TIMEOUT_MS);

    await Promise.race([discovery, delay(SCAN_TIMEOUT_MS)]);

    if (!foundAmaru) {
      initialDiscoveryPending = false;
      connection = "disconnected";
      render();
    }
  } catch (cause) {
    initialDiscoveryPending = false;
    connection = "disconnected";
    error = message(cause);
    render();
  }
}

async function stopScanAfterDiscovery(ble: Bluetooth): Promise<void> {
  try {
    await ble.stopScan();
  } catch (cause) {
    if (connection !== "scanning") return;

    connection = "disconnected";
    error = message(cause);
    render();
  }
}

async function attach(address: string): Promise<void> {
  if (connection === "connecting" || connection === "awaiting" || connection === "connected") return;

  let ble: Bluetooth | null = null;
  const attempt = ++connectionAttempt;

  try {
    ble = bluetoothApi();
    connection = "connecting";
    selectedAddress = address;
    stream.reset();
    error = null;
    render();

    await ble.stopScan();
    if (attempt !== connectionAttempt) return;
    await ble.connect(address, () => onDisconnect(attempt));
    if (attempt !== connectionAttempt) return;
    await ble.subscribe(STREAM_UUID, SERVICE_UUID, receiveNotification);
    if (attempt !== connectionAttempt) return;
    connection = "awaiting";
  } catch (cause) {
    if (attempt !== connectionAttempt) return;
    if (ble !== null) await resetQuietly();
    connection = "disconnected";
    error = message(cause);
  }
  if (attempt !== connectionAttempt) return;
  render();
}

async function resetQuietly(): Promise<void> {
  try {
    await Promise.race([
      invoke("plugin:blec|reset_connection"),
      new Promise<void>((resolve) => window.setTimeout(resolve, RESET_TIMEOUT_MS)),
    ]);
  } catch {
    // A best-effort cleanup must never prevent a retry.
  }
}

async function resume(): Promise<void> {
  if (resuming || connection === "shutting_down" || bluetooth === null) return;

  resuming = true;
  initialDiscoveryPending = true;
  try {
    connectionAttempt += 1;
    await resetQuietly();
    onDisconnect();
    await discover(true);
  } finally {
    resuming = false;
  }
}

function beginPowerOff(): void {
  confirmingPowerOff = true;
  resetPowerOffSlide();
  render();
}

function cancelPowerOff(): void {
  confirmingPowerOff = false;
  resetPowerOffSlide();
  render();
}

function beginPowerOffSlide(event: PointerEvent): void {
  const slider = event.currentTarget as HTMLInputElement;
  if (slider.valueAsNumber > 0) {
    slider.value = "0";
    updatePowerOffHandleColor(slider);
    return;
  }

  powerOffSlideStartX = event.clientX;
  powerOffSlideMoved = false;
}

function movePowerOffSlide(event: PointerEvent): void {
  if (powerOffSlideStartX !== null && event.clientX - powerOffSlideStartX >= POWER_OFF_SLIDE_DISTANCE_PX) {
    powerOffSlideMoved = true;
  }
}

function updatePowerOffSlide(slider: HTMLInputElement): void {
  updatePowerOffHandleColor(slider);
  if (powerOffSlideMoved && slider.valueAsNumber >= 100) {
    void powerOff();
  }
}

function finishPowerOffSlide(slider: HTMLInputElement): void {
  if (!powerOffSlideMoved || slider.valueAsNumber < 100) {
    slider.value = "0";
    updatePowerOffHandleColor(slider);
  }
  resetPowerOffSlide();
}

function updatePowerOffHandleColor(slider: HTMLInputElement): void {
  const progress = Math.min(1, Math.max(0, slider.valueAsNumber / 100));
  const color = POWER_OFF_HANDLE_START.map((start, index) =>
    Math.round(start + (POWER_OFF_HANDLE_END[index]! - start) * progress),
  );
  slider.style.setProperty("--power-off-handle-color", `rgb(${color.join(" ")})`);
}

function resetPowerOffSlide(): void {
  powerOffSlideStartX = null;
  powerOffSlideMoved = false;
}

async function powerOff(): Promise<void> {
  const previousConnection = connection;

  try {
    connection = "shutting_down";
    confirmingPowerOff = false;
    resetPowerOffSlide();
    error = null;
    render();
    await bluetoothApi().send(POWER_OFF_UUID, [...POWER_OFF_COMMAND], "withResponse", SERVICE_UUID);
  } catch (cause) {
    connection = previousConnection;
    error = message(cause);
    render();
  }
}

function onDisconnect(attempt?: number): void {
  if (attempt !== undefined && attempt !== connectionAttempt) return;

  connection = "disconnected";
  snapshot = null;
  lastPayloadAt = null;
  lastTipHash = null;
  lastTipUpdateAt = null;
  stream.reset();
  selectedAddress = null;
  confirmingPowerOff = false;
  resetPowerOffSlide();
  render();
}

function receiveNotification(notification: number[]): void {
  try {
    const next = stream.push(notification, (payload) => decodeSnapshot(decode(payload)));
    if (next !== null) {
      const now = Date.now();
      snapshot = next;
      lastPayloadAt = now;
      if (next.tip !== null && next.tip.headerHash !== lastTipHash) {
        lastTipHash = next.tip.headerHash;
        lastTipUpdateAt = now;
      }
      if (connection !== "shutting_down") connection = "connected";
      error = null;
      render();
    }
  } catch (cause) {
    error = message(cause);
    render();
  }
}

function render(): void {
  app.innerHTML = connection === "shutting_down" ? shutdownView() : snapshot === null ? setupView() : dashboardView(snapshot);
  bindActions();
}

function shutdownView(): string {
  return `
    <section class="shell setup">
      <header class="masthead">
        <img class="brand-logo" src="${amaruLogo}" alt="" />
        <div><p class="eyebrow">Cardano. Everywhere.</p><h1>Amaru</h1></div>
      </header>
      <p class="loading"><i></i>Shutting down Amaru...</p>
    </section>`;
}

function setupView(): string {
  const waitingForTelemetry = connection === "connecting" || connection === "awaiting";
  const discoveringInitialNode = initialDiscoveryPending && (bluetooth === null || connection === "scanning");
  const nodes = [...devices.values()]
    .sort((left, right) => right.rssi - left.rssi)
    .map(
      (device) => `
        <button class="device" data-connect="${escape(device.address)}">
          <span class="device__mark"></span>
          <span>
            <strong>${escape(device.name || "Amaru node")}</strong>
            <small>${escape(device.address)}</small>
          </span>
          <span class="chevron">›</span>
        </button>`,
    )
    .join("");
  const busy = connection === "scanning" || waitingForTelemetry;
  const unavailable = !hasTauriRuntime;

  return `
    <section class="shell setup">
      <header class="masthead">
        <img class="brand-logo" src="${amaruLogo}" alt="" />
        <div><p class="eyebrow">Cardano. Everywhere.</p><h1>Amaru</h1></div>
      </header>
      <div class="setup-copy">
        <p>Connect to a nearby Amaru node over <i class="accent-cyan">Bluetooth</i>.</p>
      </div>
      ${waitingForTelemetry ? `<p class="loading"><i></i>${connection === "connecting" ? "Connecting to Amaru..." : "Waiting for telemetry..."}</p>` : discoveringInitialNode ? `<p class="loading"><i></i>Searching nearby Amaru nodes...</p>` : `
        <button class="primary${connection === "scanning" ? " primary--scanning" : ""}" data-scan ${busy || unavailable ? "disabled" : ""}>
          ${unavailable ? "Open in Amaru Mobile" : connection === "scanning" ? "Scanning nearby nodes..." : "Find Amaru node"}
        </button>`}
      <section class="found" aria-live="polite">
        ${waitingForTelemetry ? "" : nodes || ""}
      </section>
      ${unavailable ? '<p class="notice">Bluetooth is available only from the native Tauri application. Start it with <code>npm run tauri dev</code> for macOS or <code>npm run tauri ios dev</code> for an iPhone.</p>' : ""}
      ${error === null ? "" : `<p class="error">${escape(error)}</p>`}
    </section>`;
}

function dashboardView(current: Snapshot): string {
  const resource = current.resource;
  const peers = current.peers.map(peerRow).join("") || '<tr><td colspan="4" class="muted">No peer telemetry yet.</td></tr>';
  const tip = current.tip;
  const signalLost = lastPayloadAt === null || Date.now() - lastPayloadAt > SIGNAL_TIMEOUT_MS;

  return `
    <section class="shell dashboard${confirmingPowerOff ? " dashboard--confirming-power-off" : ""}">
      <header class="topbar">
        <img class="brand-logo brand-logo--compact" src="${amaruLogo}" alt="" />
        <div class="node-name"><strong>AMARU</strong><span>${escape(current.node.version.replace(/^amaru\s+/i, ""))}</span></div>
        <span class="connection ${signalLost ? "connection--lost" : ""}"><i></i>${signalLost ? "no signal" : lastTipAge()}</span>
        ${current.powerOffEnabled ? powerOffControl() : ""}
      </header>

      ${detailsCard("Node", [
        ["PID", String(current.node.pid)],
        ["Uptime", uptime(current.node.uptimeSeconds)],
        ["Network", current.node.network],
        [
          "Temperature",
          resource === null
            ? "-"
            : temperatures(resource.averageTemperatureCelsius, resource.maximumTemperatureCelsius),
        ],
      ])}

      <section class="resource-grid">
        ${metric("Memory", resource === null ? "-" : bytes(resource.processMemoryBytes), resource === null ? null : percent(resource.processMemoryBytes, resource.hostMemoryTotalBytes))}
        ${metric("CPU", resource === null ? "-" : `${resource.cpuPercent.toFixed(1)}%`, resource === null ? null : resource.cpuPercent, false)}
        ${metric(
          "Disk read",
          resource === null ? "-" : dataRate(resource.processDiskReadBytes),
          resource === null ? null : percent(resource.processDiskReadBytes, resource.hostDiskReadBytes),
        )}
        ${metric(
          "Disk write",
          resource === null ? "-" : dataRate(resource.processDiskWriteBytes),
          resource === null ? null : percent(resource.processDiskWriteBytes, resource.hostDiskWriteBytes),
        )}
      </section>

      <section class="card-grid">
        ${tipCard(tip)}
        ${detailsCard("Chain quality", [
          ["Density", tip === null ? "-" : `${(tip.density * 100).toFixed(2)}%`],
          ["Rollback depth", current.chainQuality.averageRollbackLength === null ? "-" : current.chainQuality.averageRollbackLength.toFixed(1)],
          ["Rollback frequency", current.chainQuality.rollbackFrequencyPerSecond === null ? "-" : rate(current.chainQuality.rollbackFrequencyPerSecond, "/s")],
        ])}
      </section>

      <section class="card-grid">
        ${detailsCard("Throughput", [
          ["Blocks", count(current.throughput.blocks)],
          ["Block rate", rate(current.throughput.blocksPerSecond, "blocks/s")],
          ["Transactions", count(current.throughput.transactions)],
          ["Transaction rate", rate(current.throughput.transactionsPerSecond, "tx/s")],
        ])}
        ${detailsCard("Mempool", [
          ["Transactions", count(current.mempool.transactions)],
          ["Occupancy", bytes(current.mempool.sizeBytes)],
        ])}
      </section>

      <section class="card peers">
        <div class="card__title">Peers</div>
        <div class="table-wrap">
          <table>
            <thead><tr><th></th><th>Name</th><th>RTT</th></tr></thead>
            <tbody>${peers}</tbody>
          </table>
        </div>
      </section>
      ${confirmingPowerOff ? powerOffSlider() : ""}
      ${error === null ? "" : `<p class="error">${escape(error)}</p>`}
    </section>`;
}

function powerOffControl(): string {
  return `<button class="power-button" data-power-off type="button" aria-label="Power off node" title="Power off node">
    <svg class="power-button__icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 2v10M6.34 5.34a8 8 0 1 0 11.32 0" /></svg>
  </button>`;
}

function powerOffSlider(): string {
  return `<section class="power-off-slider" aria-label="Confirm node power off">
    <div class="power-off-slider__heading">
      <strong>Power off node</strong>
      <button class="text-button" data-cancel-power-off type="button">Cancel</button>
    </div>
    <div class="power-off-slider__rail">
      <span>Slide to power off</span>
      <input class="power-off-slider__input" data-power-slider type="range" min="0" max="100" value="0" aria-label="Slide to power off node" />
    </div>
  </section>`;
}

function metric(label: string, value: string, valueAsPercent: number | null, showPercent = true): string {
  const boundedPercent = valueAsPercent === null ? null : Math.min(100, Math.max(0, valueAsPercent));
  const detail = showPercent && boundedPercent !== null ? `<small>${boundedPercent.toFixed(1)}%</small>` : "";
  const progress = boundedPercent === null ? "" : `<i class="metric__bar"><i style="width:${boundedPercent}%"></i></i>`;
  return `<section class="metric"><span>${label}</span><strong>${escape(value)}</strong>${detail}${progress}</section>`;
}

function tipCard(tip: Snapshot["tip"]): string {
  return detailsCard(
    "Local tip",
    tip === null
      ? [["Status", "Waiting for a tip.update trace."]]
      : [
          ["Epoch", String(tip.epoch)],
          ["Slot", count(tip.slot)],
          ["Height", count(tip.blockHeight)],
          ["Hash", tip.headerHash.slice(0, 16)],
        ],
  );
}

function detailsCard(title: string, entries: [string, string][]): string {
  return `<section class="card"><div class="card__title">${escape(title)}</div><dl class="details">${entries
    .map(([name, value]) => `<div><dt>${escape(name)}</dt><dd>${escape(value)}</dd></div>`)
    .join("")}</dl></section>`;
}

function peerRow(peer: Snapshot["peers"][number]): string {
  const direction = `${peer.outbound ? "↓" : ""}${peer.inbound ? "↑" : ""}` || "-";
  return `<tr>
    <td><i class="peer-state ${peer.connected ? "online" : "offline"}"></i> ${direction}</td>
    <td title="${escape(peer.address)}">${escape(truncate(peer.address, 24))}</td>
    <td>${duration(peer.rttMicros)}</td>
  </tr>`;
}

function truncate(value: string, limit: number): string {
  return value.length <= limit ? value : `${value.slice(0, limit - 1)}…`;
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds));
}

function bindActions(): void {
  app.querySelector<HTMLButtonElement>("[data-scan]")?.addEventListener("click", () => void discover(false));
  app.querySelector<HTMLButtonElement>("[data-power-off]")?.addEventListener("click", beginPowerOff);
  app.querySelector<HTMLButtonElement>("[data-cancel-power-off]")?.addEventListener("click", cancelPowerOff);
  const powerOffSlider = app.querySelector<HTMLInputElement>("[data-power-slider]");
  powerOffSlider?.addEventListener("pointerdown", beginPowerOffSlide);
  powerOffSlider?.addEventListener("pointermove", movePowerOffSlide);
  powerOffSlider?.addEventListener("pointerup", () => finishPowerOffSlide(powerOffSlider));
  powerOffSlider?.addEventListener("pointercancel", () => finishPowerOffSlide(powerOffSlider));
  powerOffSlider?.addEventListener("input", () => updatePowerOffSlide(powerOffSlider));
  for (const element of app.querySelectorAll<HTMLButtonElement>("[data-connect]")) {
    element.addEventListener("click", () => void attach(element.dataset.connect ?? ""));
  }
}

function isAmaru(device: BleDevice): boolean {
  return device.services.some((service) => service.toLowerCase() === SERVICE_UUID) || device.name.toLowerCase().includes("amaru");
}

function bluetoothApi(): Bluetooth {
  if (bluetooth !== null) return bluetooth;
  if (hasTauriRuntime) throw new Error("Bluetooth is still initialising");
  throw new Error("Bluetooth is available only from the native Amaru Mobile application");
}

function lastTipAge(): string {
  if (lastTipUpdateAt === null) return "waiting for tip";
  return `${Math.max(0, Math.floor((Date.now() - lastTipUpdateAt) / 1_000))}s ago`;
}

function message(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function escape(value: string): string {
  return value.replace(/[&<>'"]/g, (character) => {
    const entities: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" };
    return entities[character];
  });
}

function element(selector: string): HTMLElement {
  const selected = document.querySelector<HTMLElement>(selector);
  if (selected === null) throw new Error(`Missing required element: ${selector}`);
  return selected;
}
