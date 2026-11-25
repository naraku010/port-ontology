"use client";

import { useTerminal } from "@/store/useTerminal";
import { fmt, hoursFrom } from "@/lib/time";

/** 간트 차트에 표시하는 계획 기간(시간). 시드 데이터는 2026-12-01부터 4일. */
const HORIZON_H = 96;
const pct = (h: number) => `${(Math.max(0, Math.min(HORIZON_H, h)) / HORIZON_H) * 100}%`;

export function BerthGantt() {
  const schedule = useTerminal((s) => s.schedule);
  const impact = useTerminal((s) => s.impact);
  const rootVoyage = useTerminal((s) => s.rootVoyage);
  const setScenario = useTerminal((s) => s.setScenario);
  const openEntity = useTerminal((s) => s.openEntity);

  if (!schedule) return <div className="p-4 text-sm text-muted">선석 계획을 불러오는 중…</div>;
  const start = schedule.plan_start;
  const shifts = new Map((impact?.voyage_shifts ?? []).map((s) => [s.voyage_id, s]));
  const missedTargets = new Set((impact?.connection_groups ?? []).map((g) => g.to_voyage));
  const ticks = Array.from({ length: HORIZON_H / 6 }, (_, i) => i * 6);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between px-4 pt-3 pb-2">
        <h2 className="text-sm font-semibold tracking-wide">선석 배정 계획</h2>
        <div className="flex gap-4 text-xs text-muted">
          <Legend className="bg-accent/70" label="계획" />
          <Legend className="bg-root" label="지연 (입력)" />
          <Legend className="bg-shift" label="연쇄 지연 (R2)" />
          <Legend className="border border-dashed border-muted" label="원래 일정" />
          <Legend className="!w-2.5 rounded-full bg-dg" label="환적 화물 못 받음" />
        </div>
      </div>

      <div className="relative min-h-0 flex-1 overflow-auto px-4 pb-3">
        <div className="relative ml-36 h-6 border-b border-line text-[10px] text-muted">
          {ticks.map((h) => (
            <div key={h} className="absolute -translate-x-1/2" style={{ left: pct(h) }}>
              {h % 24 === 0 ? <span className="font-semibold text-ink">12/{String(1 + h / 24).padStart(2, "0")}</span> : `${h % 24}시`}
            </div>
          ))}
        </div>

        {schedule.berths.map(({ berth, calls }) => (
          <div key={berth.id} className="flex h-[52px] items-stretch border-b border-line/60">
            <button onClick={() => openEntity(berth.id)} className="w-36 shrink-0 pr-3 text-left hover:text-accent" title="선석 상세 보기">
              <div className="truncate text-sm font-medium">{berth.name}</div>
              <div className="text-[10px] text-muted">
                {berth.length_m}m · 수심 {berth.depth_m}m · 크레인 {berth.cranes}
              </div>
            </button>
            <div className="relative flex-1">
              {ticks.map((h) => (
                <div key={h} className="absolute inset-y-0 w-px bg-line/40" style={{ left: pct(h) }} />
              ))}
              {calls.map((c) => {
                const shift = shifts.get(c.voyage_id);
                const a = hoursFrom(start, shift?.new_eta ?? c.eta);
                const d = hoursFrom(start, shift?.new_etd ?? c.etd);
                const color = !shift ? "bg-accent/70" : shift.caused_by ? "bg-shift" : "bg-root";
                const selected = c.voyage_id === rootVoyage;
                return (
                  <div key={c.voyage_id}>
                    {shift && (
                      <div
                        className="absolute top-2 bottom-2 rounded border border-dashed border-muted/70"
                        style={{ left: pct(hoursFrom(start, shift.old_eta)), width: `calc(${pct(hoursFrom(start, shift.old_etd))} - ${pct(hoursFrom(start, shift.old_eta))})` }}
                      />
                    )}
                    <button
                      onClick={() => setScenario(c.voyage_id)}
                      onDoubleClick={() => openEntity(c.voyage_id)}
                      title={`${c.vessel_name} ${c.voyage_no} (${c.voyage_id})\n${c.service}\n${fmt(shift?.new_eta ?? c.eta)} → ${fmt(shift?.new_etd ?? c.etd)}${shift ? `\n+${shift.delay_hours}시간 지연` : ""}\n클릭: 시나리오 대상으로 선택 · 더블클릭: 상세 보기`}
                      className={`absolute top-2 bottom-2 overflow-hidden rounded px-1.5 text-left text-[11px] leading-tight text-bg transition-[left,width] duration-500 ${color} ${selected ? "outline-2 outline-offset-1 outline-ink" : ""}`}
                      style={{ left: pct(a), width: `calc(${pct(d)} - ${pct(a)})` }}
                    >
                      {missedTargets.has(c.voyage_id) && <span className="absolute top-1 right-1 h-2 w-2 rounded-full bg-dg" />}
                      <div className="truncate font-semibold">{c.vessel_name}</div>
                      <div className="truncate opacity-80">
                        {c.voyage_id} · {c.voyage_no}
                        {shift ? ` · +${shift.delay_hours}h` : ""}
                      </div>
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function Legend({ className, label }: { className: string; label: string }) {
  return (
    <span className="flex items-center gap-1.5">
      <span className={`inline-block h-2.5 w-4 rounded-sm ${className}`} />
      {label}
    </span>
  );
}
