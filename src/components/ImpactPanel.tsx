"use client";

import { useState } from "react";
import { useTerminal } from "@/store/useTerminal";
import { ImpactGraph } from "./ImpactGraph";

type View = "graph" | "evidence" | "containers" | "mitigation";

export function ImpactPanel() {
  const impact = useTerminal((s) => s.impact);
  const [view, setView] = useState<View>("graph");
  const tabs: { id: View; label: string; count?: number }[] = [
    { id: "graph", label: "영향 그래프" },
    { id: "evidence", label: "근거 경로", count: impact?.connection_groups.length },
    { id: "containers", label: "환적 실패 컨테이너", count: impact?.missed_connections.length },
    { id: "mitigation", label: "대응 방안", count: impact?.berth_options.length },
  ];

  return (
    <div className="flex h-full flex-col">
      <div className="flex gap-1 border-b border-line px-3">
        {tabs.map((t) => (
          <button
            key={t.id}
            onClick={() => setView(t.id)}
            className={`-mb-px border-b-2 px-3 py-2 text-sm ${view === t.id ? "border-accent text-ink" : "border-transparent text-muted hover:text-ink"}`}
          >
            {t.label}
            {t.count !== undefined && <span className="ml-1.5 rounded bg-panel-2 px-1.5 text-xs">{t.count}</span>}
          </button>
        ))}
      </div>
      <div className="min-h-0 flex-1">
        {view === "graph" && <ImpactGraph />}
        {view === "evidence" && <Evidence />}
        {view === "containers" && <Containers />}
        {view === "mitigation" && <Mitigation />}
      </div>
    </div>
  );
}

function Evidence() {
  const impact = useTerminal((s) => s.impact);
  if (!impact) return <NoScenario />;
  return (
    <div className="h-full space-y-2 overflow-auto p-3">
      <p className="text-xs text-muted">각 환적 실패가 어떤 관계와 규칙을 따라 발생했는지 보여줍니다. LLM도 이 경로를 근거로 인용합니다.</p>
      {impact.connection_groups.map((g) => (
        <div key={`${g.from_voyage}>${g.to_voyage}`} className="rounded border border-line bg-panel p-3">
          <div className="mb-2 flex items-center gap-3 text-xs">
            <span className="font-mono font-semibold">
              {g.from_voyage} → {g.to_voyage}
            </span>
            <span className="text-muted">컨테이너 {g.containers}개</span>
            {g.reefer > 0 && <span className="text-reefer">❄ 냉동 {g.reefer}</span>}
            {g.dangerous_goods > 0 && <span className="text-dg">⚠ 위험물 {g.dangerous_goods}</span>}
            {g.high_priority > 0 && <span className="text-root">우선 처리 {g.high_priority}</span>}
          </div>
          <ol className="flex flex-wrap items-center gap-1 text-xs">
            {g.evidence_path.map((step, i) => (
              <li key={i} className="flex items-center gap-1">
                {i > 0 && <span className="text-muted">→</span>}
                <span className={`rounded px-1.5 py-0.5 ${step.includes("(R") ? "bg-shift/15 text-shift" : "bg-panel-2"}`}>{step}</span>
              </li>
            ))}
          </ol>
        </div>
      ))}
      {impact.connection_groups.length === 0 && <p className="text-sm text-muted">이 시나리오에서는 끊기는 환적 연결이 없습니다.</p>}
    </div>
  );
}

function Containers() {
  const impact = useTerminal((s) => s.impact);
  const proposeRebooking = useTerminal((s) => s.proposeRebooking);
  const openEntity = useTerminal((s) => s.openEntity);
  if (!impact) return <NoScenario />;
  return (
    <div className="h-full overflow-auto">
      <table className="w-full text-xs">
        <thead className="sticky top-0 bg-panel text-left text-muted">
          <tr>
            {["컨테이너", "환적 연결", "목적지", "부족 시간", "우선순위", "대체 항차", ""].map((h) => (
              <th key={h} className="px-3 py-2 font-medium">
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {impact.missed_connections.map((m) => (
            <tr key={m.container_id} className="border-t border-line/60 hover:bg-panel">
              <td className="px-3 py-1.5 font-mono">
                <button onClick={() => openEntity(m.container_id)} className="hover:text-accent">
                  {m.container_id}
                </button>
                {m.reefer && <span className="ml-1 text-reefer">❄</span>}
                {m.imdg_class && <span className="ml-1 text-dg">⚠{m.imdg_class}</span>}
              </td>
              <td className="px-3 py-1.5 font-mono">
                {m.from_voyage} → {m.to_voyage}
              </td>
              <td className="px-3 py-1.5">{m.destination}</td>
              <td className="px-3 py-1.5 tabular-nums">{m.short_by_hours}시간</td>
              <td className="px-3 py-1.5">
                <span className={m.priority === "HIGH" ? "text-root" : "text-muted"}>{m.priority === "HIGH" ? "높음" : "보통"}</span>
                {m.priority_reasons.length > 0 && <span className="ml-1 text-muted">({m.priority_reasons.join(", ")})</span>}
              </td>
              <td className="px-3 py-1.5 font-mono">{m.alternative_voyage ?? <span className="text-muted">계획 기간 내 없음</span>}</td>
              <td className="px-3 py-1.5 text-right">
                {m.alternative_voyage && (
                  <button onClick={() => proposeRebooking(m.container_id, m.alternative_voyage!)} className="rounded border border-line px-2 py-0.5 hover:border-accent hover:text-accent">
                    항차 변경 제안
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Mitigation() {
  const impact = useTerminal((s) => s.impact);
  const proposeBerth = useTerminal((s) => s.proposeBerth);
  if (!impact) return <NoScenario />;
  return (
    <div className="h-full space-y-2 overflow-auto p-3">
      <p className="text-xs text-muted">선석 점유(R2) 때문에 밀린 항차 중, 제원(R1)이 맞고 원래 시간대가 비어 있는 다른 선석으로 옮길 수 있는 경우입니다.</p>
      {impact.berth_options.map((o) => (
        <div key={`${o.voyage_id}-${o.berth_id}`} className="flex items-center justify-between rounded border border-line bg-panel p-3">
          <div>
            <div className="text-sm font-semibold">
              {o.voyage_id} → {o.berth_id} 선석 변경 <span className="ml-2 text-ok">{o.saves_hours}시간 단축</span>
            </div>
            <div className="text-xs text-muted">{o.note}</div>
          </div>
          <button onClick={() => proposeBerth(o.voyage_id, o.berth_id)} className="rounded bg-accent px-3 py-1.5 text-sm font-semibold text-bg hover:brightness-110">
            변경 제안
          </button>
        </div>
      ))}
      {impact.berth_options.length === 0 && <p className="text-sm text-muted">이 시나리오에서 가능한 선석 변경이 없습니다.</p>}
    </div>
  );
}

function NoScenario() {
  return <div className="flex h-full items-center justify-center text-sm text-muted">먼저 왼쪽에서 [영향 분석]을 실행하세요.</div>;
}
