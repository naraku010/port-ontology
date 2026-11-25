"use client";

import { useTerminal } from "@/store/useTerminal";

export function ScenarioPanel() {
  const s = useTerminal();
  const voyages = (s.schedule?.berths ?? []).flatMap((b) => b.calls.map((c) => ({ ...c, berth: b.berth.id })));
  voyages.sort((a, b) => a.voyage_id.localeCompare(b.voyage_id));
  const high = s.impact?.missed_connections.filter((m) => m.priority === "HIGH").length ?? 0;

  return (
    <aside className="flex w-80 shrink-0 flex-col gap-4 overflow-y-auto border-r border-line bg-panel p-4">
      <section>
        <h2 className="mb-3 text-xs font-semibold tracking-widest text-muted">지연 시나리오</h2>
        <label className="mb-1 block text-xs text-muted">지연되는 항차</label>
        <select value={s.rootVoyage} onChange={(e) => s.setScenario(e.target.value)} className="mb-3 w-full rounded border border-line bg-panel-2 px-2 py-1.5 text-sm">
          {voyages.map((v) => (
            <option key={v.voyage_id} value={v.voyage_id}>
              {v.voyage_id} · {v.vessel_name} {v.voyage_no} ({v.berth})
            </option>
          ))}
        </select>
        <label className="mb-1 flex justify-between text-xs text-muted">
          <span>지연 시간</span>
          <span className="font-mono text-ink">+{s.delayHours}시간</span>
        </label>
        <input
          type="range"
          min={1}
          max={72}
          value={s.delayHours}
          onChange={(e) => s.setScenario(s.rootVoyage, Number(e.target.value))}
          className="mb-3 w-full accent-[var(--color-root)]"
        />
        <div className="flex gap-2">
          <button onClick={s.runImpact} disabled={s.loading} className="flex-1 rounded bg-accent px-3 py-2 text-sm font-semibold text-bg hover:brightness-110 disabled:opacity-50">
            영향 분석
          </button>
          <button onClick={s.reset} disabled={s.loading} className="rounded border border-line px-3 py-2 text-sm text-muted hover:text-ink" title="시드 데이터로 되돌리기">
            초기화
          </button>
        </div>
      </section>

      {s.impact && (
        <section className="rounded border border-line bg-panel-2 p-3">
          <div className="mb-2 grid grid-cols-3 gap-2 text-center">
            <Stat value={s.impact.voyage_shifts.length - 1} label="연쇄 지연 항차" tone="text-shift" />
            <Stat value={s.impact.missed_connections.length} label="환적 실패" tone="text-dg" />
            <Stat value={high} label="우선 처리" tone="text-root" />
          </div>
          <p className="text-xs leading-relaxed text-muted">{s.impact.summary}</p>
        </section>
      )}

      {s.impact && s.impact.shippers.length > 0 && (
        <section>
          <h3 className="mb-2 text-xs font-semibold tracking-widest text-muted">영향받는 화주</h3>
          <ul className="space-y-1">
            {s.impact.shippers.map((sh) => (
              <li key={sh.shipper_id}>
                <button onClick={() => s.openEntity(sh.shipper_id)} className="flex w-full items-center justify-between rounded px-2 py-1 text-left text-sm hover:bg-panel-2">
                  <span className="truncate">
                    {sh.name}
                    {sh.tier === "KEY" && <span className="ml-1.5 rounded bg-ok/20 px-1 text-[10px] text-ok">핵심</span>}
                  </span>
                  <span className="flex shrink-0 gap-2 font-mono text-xs">
                    {sh.reefer > 0 && <span className="text-reefer" title="냉동">❄{sh.reefer}</span>}
                    {sh.dangerous_goods > 0 && <span className="text-dg" title="위험물">⚠{sh.dangerous_goods}</span>}
                    <span title="환적 실패 컨테이너">{sh.containers}</span>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}

      <section className="mt-auto">
        <h3 className="mb-2 text-xs font-semibold tracking-widest text-muted">감사 로그</h3>
        {s.auditLog.length === 0 ? (
          <p className="text-xs text-muted">아직 적용된 계획이 없습니다.</p>
        ) : (
          <ul className="space-y-1 text-xs">
            {s.auditLog.map((l, i) => (
              <li key={i} className="rounded bg-panel-2 px-2 py-1 text-muted">
                {l}
              </li>
            ))}
          </ul>
        )}
      </section>
    </aside>
  );
}

function Stat({ value, label, tone }: { value: number; label: string; tone: string }) {
  return (
    <div>
      <div className={`text-2xl font-semibold tabular-nums ${tone}`}>{value}</div>
      <div className="text-[10px] text-muted">{label}</div>
    </div>
  );
}
