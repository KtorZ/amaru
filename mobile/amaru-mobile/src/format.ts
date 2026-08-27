export function bytes(value: number): string {
  if (value < 1_024) return `${value.toFixed(0)} B`;
  if (value < 1_024 ** 2) return `${(value / 1_024).toFixed(1)} KiB`;
  if (value < 1_024 ** 3) return `${(value / 1_024 ** 2).toFixed(1)} MiB`;
  return `${(value / 1_024 ** 3).toFixed(2)} GiB`;
}

export function rate(value: number, unit: string): string {
    return `${value.toFixed(value >= 100 ? 0 : 1)} ${unit}`;
}

export function dataRate(value: number): string {
  return `${bytes(value)}/s`;
}

export function count(value: number): string {
  return new Intl.NumberFormat("en-US").format(value);
}

export function duration(micros: number | null): string {
  if (micros === null) return "-";
  if (micros < 1_000) return `${micros.toFixed(0)} us`;
  if (micros < 1_000_000) return `${(micros / 1_000).toFixed(1)} ms`;
  return `${(micros / 1_000_000).toFixed(2)} s`;
}

export function uptime(seconds: number | null): string {
  if (seconds === null) return "-";
  const hours = Math.floor(seconds / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);
  return hours === 0 ? `${minutes} min` : `${hours} h ${minutes} min`;
}

export function percent(value: number, total: number): number | null {
  return total === 0 ? null : (value / total) * 100;
}
