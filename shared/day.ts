export const EPOCH = "2026-10-03";

const DAY_MS = 86_400_000;
const ISO_DATE = /^(\d{4})-(\d{2})-(\d{2})$/;

export function parseIsoDate(s: string): number | null {
  const m = ISO_DATE.exec(s);
  if (!m) return null;
  const y = Number(m[1]);
  const mo = Number(m[2]) - 1;
  const d = Number(m[3]);
  const t = Date.UTC(y, mo, d);
  const back = new Date(t);
  const roundTrips =
    back.getUTCFullYear() === y && back.getUTCMonth() === mo && back.getUTCDate() === d;
  return roundTrips ? t : null;
}

export function puzzleNumber(date: string, epoch = EPOCH): number | null {
  const t = parseIsoDate(date);
  const e = parseIsoDate(epoch);
  if (t === null || e === null) return null;
  return Math.round((t - e) / DAY_MS) + 1;
}

// A client's local date can sit one calendar day either side of UTC.
export function dateWithinWindow(date: string, nowMs: number): boolean {
  const t = parseIsoDate(date);
  if (t === null) return false;
  const todayUtc = Math.floor(nowMs / DAY_MS) * DAY_MS;
  return Math.abs(t - todayUtc) <= DAY_MS;
}

export function latestAllowedNumber(nowMs: number, epoch = EPOCH): number {
  const e = parseIsoDate(epoch) ?? 0;
  const todayUtc = Math.floor(nowMs / DAY_MS) * DAY_MS;
  return Math.round((todayUtc - e) / DAY_MS) + 2;
}

export function localIsoDate(now = new Date()): string {
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, "0");
  const d = String(now.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

export function msUntilLocalMidnight(now = new Date()): number {
  const next = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
  return next.getTime() - now.getTime();
}
