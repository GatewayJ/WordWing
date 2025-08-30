/**
 * 与后端 RFC3339（UTC）互转，展示/编辑使用本地日历日 + 24 小时制时间。
 * 避免 WebKitGTK 下原生 `datetime-local` 日历弹层导致整页无法操作的问题。
 */

export function isoToDateTimeParts(iso: string | null | undefined): { date: string; time: string } {
  if (!iso) return { date: "", time: "" };
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return { date: "", time: "" };
  const pad = (n: number) => String(n).padStart(2, "0");
  return {
    date: `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`,
    time: `${pad(d.getHours())}:${pad(d.getMinutes())}`,
  };
}

/** 日期必填；时间可空，视为当日 00:00（本地）。返回 ISO 字符串或 null（格式非法）。 */
export function dateTimePartsToIso(date: string, time: string): string | null {
  const dStr = date.trim();
  if (!dStr) return null;
  const dm = dStr.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (!dm) return null;
  const y = Number(dm[1]);
  const mo = Number(dm[2]);
  const day = Number(dm[3]);
  if (mo < 1 || mo > 12 || day < 1 || day > 31) return null;

  let h = 0;
  let min = 0;
  const tRaw = time.trim();
  if (tRaw) {
    const tm = tRaw.match(/^(\d{1,2}):(\d{2})$/);
    if (!tm) return null;
    h = Number(tm[1]);
    min = Number(tm[2]);
    if (h > 23 || min > 59) return null;
  }

  const d = new Date(y, mo - 1, day, h, min, 0, 0);
  if (
    Number.isNaN(d.getTime()) ||
    d.getFullYear() !== y ||
    d.getMonth() !== mo - 1 ||
    d.getDate() !== day
  ) {
    return null;
  }
  return d.toISOString();
}

export function validateDatePart(date: string): boolean {
  return /^(\d{4})-(\d{2})-(\d{2})$/.test(date.trim()) && dateTimePartsToIso(date, "00:00") !== null;
}

export function validateTimePart(time: string): boolean {
  const t = time.trim();
  if (!t) return true;
  return /^([01]?\d|2[0-3]):[0-5]\d$/.test(t);
}
