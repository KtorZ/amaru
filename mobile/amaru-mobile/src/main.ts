import type { BleDevice } from "@mnlphlp/plugin-blec";
import { decode } from "cbor-x";

import { bytes, count, dataRate, duration, percent, rate, uptime } from "./format";
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

type Connection = "disconnected" | "scanning" | "connecting" | "awaiting" | "connected";
type Bluetooth = typeof import("@mnlphlp/plugin-blec");

const app = element("#app");
const amaruLogo = new URL("../../../amaru.svg", import.meta.url).href;
const hasTauriRuntime = "__TAURI_INTERNALS__" in window;
const SIGNAL_TIMEOUT_MS = 2_000;

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
let poweringOff = false;

void initialise();
render();
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
    await bluetooth.getConnectionUpdates((connected) => {
      if (!connected) onDisconnect();
    });
    await bluetooth.getScanningUpdates((scanning) => {
      if (connection === "disconnected" || connection === "scanning") {
        connection = scanning ? "scanning" : "disconnected";
        render();
      }
    });
  } catch (cause) {
    error = message(cause);
    render();
  }
}

async function scan(): Promise<void> {
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
    await ble.startScan((found) => {
      if (foundAmaru) return;

      const device = found.find(isAmaru);
      if (device === undefined) return;

      foundAmaru = true;
      devices.set(device.address, device);
      render();
      void stopScanAfterDiscovery(ble);
    }, 10_000);
  } catch (cause) {
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

  try {
    const ble = bluetoothApi();
    connection = "connecting";
    selectedAddress = address;
    stream.reset();
    error = null;
    render();
    await ble.stopScan();
    await ble.connect(address, onDisconnect);
    await ble.subscribe(STREAM_UUID, SERVICE_UUID, receiveNotification);
    connection = "awaiting";
  } catch (cause) {
    connection = "disconnected";
    error = message(cause);
  }
  render();
}

function beginPowerOff(): void {
  confirmingPowerOff = true;
  render();
}

function cancelPowerOff(): void {
  confirmingPowerOff = false;
  render();
}

async function powerOff(): Promise<void> {
  try {
    poweringOff = true;
    error = null;
    render();
    await bluetoothApi().send(POWER_OFF_UUID, [...POWER_OFF_COMMAND], "withoutResponse", SERVICE_UUID);
  } catch (cause) {
    confirmingPowerOff = false;
    error = message(cause);
  } finally {
    poweringOff = false;
    render();
  }
}

function onDisconnect(): void {
  connection = "disconnected";
  snapshot = null;
  lastPayloadAt = null;
  lastTipHash = null;
  lastTipUpdateAt = null;
  stream.reset();
  selectedAddress = null;
  confirmingPowerOff = false;
  poweringOff = false;
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
      connection = "connected";
      error = null;
      render();
    }
  } catch (cause) {
    error = message(cause);
    render();
  }
}

function render(): void {
  app.innerHTML = snapshot === null ? setupView() : dashboardView(snapshot);
  bindActions();
}

function setupView(): string {
  const waitingForTelemetry = connection === "connecting" || connection === "awaiting";
  const nodes = [...devices.values()]
    .sort((left, right) => right.rssi - left.rssi)
    .map(
      (device) => `
        <button class="device" data-connect="${escape(device.address)}">
          <span class="device__mark"></span>
          <span>
            <strong>${escape(device.name || "Amaru node")}</strong>
            <small>${escape(device.address)} · ${device.rssi} dBm</small>
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
        <p>Connect to a nearby Amaru node over Bluetooth.</p>
      </div>
      ${waitingForTelemetry ? `<p class="loading"><i></i>${connection === "connecting" ? "Connecting to Amaru..." : "Waiting for telemetry..."}</p>` : `
        <button class="primary" data-scan ${busy || unavailable ? "disabled" : ""}>
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
    <section class="shell dashboard">
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
            <thead><tr><th>Status</th><th>Name</th><th>RTT</th><th>Direction</th></tr></thead>
            <tbody>${peers}</tbody>
          </table>
        </div>
      </section>
      ${error === null ? "" : `<p class="error">${escape(error)}</p>`}
    </section>`;
}

function powerOffControl(): string {
  if (!confirmingPowerOff) {
    return '<button class="power-button" data-power-off>Power off</button>';
  }

  return `<span class="power-confirmation">
    <span class="power-confirmation__prompt">Power off this node?</span>
    <button class="text-button" data-cancel-power-off ${poweringOff ? "disabled" : ""}>Cancel</button>
    <button class="power-button" data-confirm-power-off ${poweringOff ? "disabled" : ""}>${poweringOff ? "Requesting..." : "Confirm"}</button>
  </span>`;
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
    <td><i class="peer-state ${peer.connected ? "online" : "offline"}"></i></td>
    <td>${escape(peer.address)}</td>
    <td>${duration(peer.rttMicros)}</td>
    <td>${direction}</td>
  </tr>`;
}

function bindActions(): void {
  app.querySelector<HTMLButtonElement>("[data-scan]")?.addEventListener("click", () => void scan());
  app.querySelector<HTMLButtonElement>("[data-power-off]")?.addEventListener("click", beginPowerOff);
  app.querySelector<HTMLButtonElement>("[data-cancel-power-off]")?.addEventListener("click", cancelPowerOff);
  app.querySelector<HTMLButtonElement>("[data-confirm-power-off]")?.addEventListener("click", () => void powerOff());
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
