// Backend times are naive local datetimes ("2026-12-01T06:00:00"), so parse
// them uniformly and only ever compare them with each other.
export const hoursFrom = (start: string, t: string) => (Date.parse(t) - Date.parse(start)) / 3_600_000;

export const fmt = (t: string) => {
  const d = new Date(t);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
};
