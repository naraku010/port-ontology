"use client";

import { useEffect, useMemo, useState } from "react";
import { useTerminal } from "@/store/useTerminal";
import { callTool } from "@/lib/api";
import { PROPERTY, label } from "@/lib/labels";
import { CLASS_COLOR, SchemaDiagram } from "./SchemaDiagram";

interface Hit {
  type: string;
  id: string;
  label: string;
}

/**
 * 온톨로지 화면.
 * 왼쪽: 스키마 다이어그램(클래스=노드, 관계=화살표)과 규칙 목록. 규칙에 마우스를 올리면 관련 클래스만 강조.
 * 오른쪽: 선택한 클래스의 속성, 관계, 적용 규칙, 인스턴스 목록.
 */
export function OntologyView() {
  const schema = useTerminal((s) => s.schema);
  const [selected, setSelected] = useState("Voyage");
  const [hoverRule, setHoverRule] = useState<string | null>(null);

  if (!schema) return null;
  const highlight = hoverRule ? new Set(schema.rules.find((r) => r.id === hoverRule)?.classes ?? []) : null;

  return (
    <div className="grid h-full grid-cols-[1fr_420px] overflow-hidden">
      <div className="flex min-h-0 flex-col">
        <div className="flex items-baseline justify-between px-6 pt-5">
          <div>
            <h2 className="text-base font-semibold">터미널 온톨로지</h2>
            <p className="mt-0.5 text-xs text-muted">
              클래스 {schema.classes.length}개 · 관계 {schema.relations.length}개 · 규칙 {schema.rules.length}개. 클래스를 누르면 오른쪽에 상세가 나옵니다.
            </p>
          </div>
        </div>

        <div className="min-h-0 flex-1 px-6 py-3">
          <SchemaDiagram schema={schema} selected={selected} highlight={highlight} onSelect={setSelected} />
        </div>

        <div className="border-t border-line px-6 py-4">
          <div className="mb-2 flex items-baseline gap-2">
            <h3 className="text-sm font-semibold">추론 규칙</h3>
            <span className="text-xs text-muted">영향 분석과 제안 검증은 모두 이 규칙으로 계산합니다. 마우스를 올리면 관련 클래스가 강조됩니다.</span>
          </div>
          <div className="grid grid-cols-3 gap-2">
            {schema.rules.map((r) => (
              <div
                key={r.id}
                onMouseEnter={() => setHoverRule(r.id)}
                onMouseLeave={() => setHoverRule(null)}
                title={r.statement}
                className={`cursor-default rounded-lg border px-3 py-2 transition-colors ${hoverRule === r.id ? "border-shift bg-shift/10" : "border-line bg-panel"}`}
              >
                <div className="flex items-center gap-2">
                  <span className="font-mono text-xs font-semibold text-shift">{r.id}</span>
                  <span className="text-sm font-medium">{r.label}</span>
                </div>
                <div className="mt-0.5 truncate text-xs text-muted">{r.statement}</div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* key로 클래스가 바뀔 때마다 검색어 등 내부 상태를 초기화한다. */}
      <ClassDetail key={selected} name={selected} onSelect={setSelected} />
    </div>
  );
}

function ClassDetail({ name, onSelect }: { name: string; onSelect: (n: string) => void }) {
  const schema = useTerminal((s) => s.schema)!;
  const openEntity = useTerminal((s) => s.openEntity);
  const [filter, setFilter] = useState("");
  const [hits, setHits] = useState<{ total: number; results: Hit[] } | null>(null);

  const cls = schema.classes.find((c) => c.name === name)!;
  const byName = useMemo(() => new Map(schema.classes.map((c) => [c.name, c])), [schema]);
  const outgoing = schema.relations.filter((r) => r.domain === name);
  const incoming = schema.relations.filter((r) => r.range === name);
  const rules = schema.rules.filter((r) => r.classes.includes(name));
  const color = CLASS_COLOR[name];

  // 클래스나 검색어가 바뀌면 인스턴스 목록을 다시 불러온다(빈 검색어 = 전체).
  useEffect(() => {
    let alive = true;
    const t = setTimeout(async () => {
      const r = await callTool<{ total: number; results: Hit[] }>("search_entities", { query: filter, type: name, limit: 50 });
      if (alive) setHits(r);
    }, 150);
    return () => {
      alive = false;
      clearTimeout(t);
    };
  }, [name, filter]);

  return (
    <aside className="flex min-h-0 flex-col border-l border-line bg-panel">
      <div className="border-b border-line px-5 py-4">
        <div className="flex items-center gap-2">
          <span className="h-3 w-3 rounded-sm" style={{ background: color }} />
          <h2 className="text-lg font-semibold">{cls.label}</h2>
          <span className="font-mono text-xs text-muted">{cls.name}</span>
          <span className="ml-auto rounded-full bg-panel-2 px-2 py-0.5 text-xs tabular-nums text-muted">인스턴스 {cls.instance_count}개</span>
        </div>
        <p className="mt-1 text-sm text-muted">{cls.description}</p>
      </div>

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-5 py-4">
        <Section title="속성">
          <div className="flex flex-wrap gap-1.5">
            {cls.properties.map((p) => (
              <span key={p} className="rounded-md border border-line bg-panel-2 px-2 py-1 text-xs">
                {label(PROPERTY, p)} <span className="ml-1 font-mono text-[10px] text-muted">{p}</span>
              </span>
            ))}
          </div>
        </Section>

        <Section title="관계">
          <ul className="space-y-1">
            {outgoing.map((r) => (
              <RelationRow key={r.name} dir="→" relLabel={r.label} relName={r.name} other={byName.get(r.range)?.label ?? r.range} onClick={() => onSelect(r.range)} />
            ))}
            {incoming.map((r) => (
              <RelationRow key={r.name} dir="←" relLabel={r.label} relName={r.name} other={byName.get(r.domain)?.label ?? r.domain} onClick={() => onSelect(r.domain)} />
            ))}
          </ul>
        </Section>

        {rules.length > 0 && (
          <Section title="적용 규칙">
            <ul className="space-y-2">
              {rules.map((r) => (
                <li key={r.id} className="rounded-lg border border-line bg-panel-2 px-3 py-2">
                  <div className="flex items-center gap-2 text-sm">
                    <span className="font-mono text-xs font-semibold text-shift">{r.id}</span>
                    <span className="font-medium">{r.label}</span>
                  </div>
                  <p className="mt-1 text-xs leading-relaxed text-muted">{r.statement}</p>
                </li>
              ))}
            </ul>
          </Section>
        )}

        <Section title="인스턴스" aside={hits ? `${hits.total}개` : undefined}>
          <input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder={`${cls.label} ID나 이름으로 찾기`}
            className="mb-2 w-full rounded-md border border-line bg-panel-2 px-2.5 py-1.5 text-sm outline-none focus:border-accent"
          />
          <ul className="divide-y divide-line/60 rounded-md border border-line">
            {hits?.results.map((h) => (
              <li key={h.id}>
                <button onClick={() => openEntity(h.id)} className="flex w-full items-baseline gap-3 px-3 py-1.5 text-left hover:bg-panel-2">
                  <span className="shrink-0 font-mono text-xs">{h.id}</span>
                  <span className="truncate text-xs text-muted">{h.label}</span>
                </button>
              </li>
            ))}
            {hits && hits.results.length === 0 && <li className="px-3 py-2 text-xs text-muted">일치하는 인스턴스가 없습니다.</li>}
          </ul>
          {hits && hits.total > hits.results.length && <p className="mt-1 text-xs text-muted">상위 {hits.results.length}개만 표시합니다. 검색어로 좁혀 보세요.</p>}
        </Section>
      </div>
    </aside>
  );
}

function Section({ title, aside, children }: { title: string; aside?: string; children: React.ReactNode }) {
  return (
    <section>
      <div className="mb-2 flex items-baseline justify-between">
        <h3 className="text-xs font-semibold tracking-wider text-muted">{title}</h3>
        {aside && <span className="text-xs text-muted tabular-nums">{aside}</span>}
      </div>
      {children}
    </section>
  );
}

function RelationRow({ dir, relLabel, relName, other, onClick }: { dir: string; relLabel: string; relName: string; other: string; onClick: () => void }) {
  return (
    <li>
      <button onClick={onClick} className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-panel-2">
        <span className="w-4 text-center text-muted">{dir}</span>
        <span>{relLabel}</span>
        <span className="font-mono text-[10px] text-muted">{relName}</span>
        <span className="ml-auto font-medium text-accent">{other}</span>
      </button>
    </li>
  );
}
