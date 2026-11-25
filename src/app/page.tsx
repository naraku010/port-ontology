"use client";

import { useEffect } from "react";
import { useTerminal, type Tab } from "@/store/useTerminal";
import { inTauri } from "@/lib/api";
import { BerthGantt } from "@/components/BerthGantt";
import { ImpactPanel } from "@/components/ImpactPanel";
import { ScenarioPanel } from "@/components/ScenarioPanel";
import { ProposalBar } from "@/components/ProposalBar";
import { EntityInspector } from "@/components/EntityInspector";
import { OntologyView } from "@/components/OntologyView";
import { McpView } from "@/components/McpView";

const TABS: { id: Tab; label: string }[] = [
  { id: "operations", label: "운영" },
  { id: "ontology", label: "온톨로지" },
  { id: "mcp", label: "MCP / LLM" },
];

export default function Home() {
  const { tab, setTab, load, error, clearError, loading } = useTerminal();

  useEffect(() => {
    load();
  }, [load]);

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center gap-6 border-b border-line bg-panel px-4 py-2">
        <div>
          <div className="text-sm font-semibold">Port Ontology</div>
          <div className="text-[10px] text-muted">가상의 컨테이너 터미널 · 계획 기간 2026-12-01 ~ 12-04</div>
        </div>
        <nav className="flex gap-1">
          {TABS.map((t) => (
            <button
              key={t.id}
              onClick={() => setTab(t.id)}
              className={`rounded px-3 py-1.5 text-sm ${tab === t.id ? "bg-panel-2 text-ink" : "text-muted hover:text-ink"}`}
            >
              {t.label}
            </button>
          ))}
        </nav>
        <div className="ml-auto text-xs text-muted">{loading ? "처리 중…" : ""}</div>
      </header>

      {error && (
        <div className="flex items-center justify-between bg-root/15 px-4 py-2 text-sm text-root">
          <span>{error}</span>
          {inTauri() && (
            <button onClick={clearError} className="text-xs underline">
              닫기
            </button>
          )}
        </div>
      )}

      <main className="relative min-h-0 flex-1">
        {tab === "operations" && (
          <div className="flex h-full">
            <ScenarioPanel />
            <div className="flex min-w-0 flex-1 flex-col">
              <div className="h-[46%] min-h-0 border-b border-line">
                <BerthGantt />
              </div>
              <div className="min-h-0 flex-1">
                <ImpactPanel />
              </div>
              <ProposalBar />
            </div>
          </div>
        )}
        {tab === "ontology" && <OntologyView />}
        {tab === "mcp" && <McpView />}
        <EntityInspector />
      </main>
    </div>
  );
}
