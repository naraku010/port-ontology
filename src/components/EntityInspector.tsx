"use client";

import { useTerminal } from "@/store/useTerminal";
import type { EntityRelation } from "@/lib/types";
import { ENTITY_TYPE, PROPERTY, RELATION, VALUE, label } from "@/lib/labels";

// get_entity가 많은 관계를 요약할 때 쓰는 키 (engine.rs get_entity 참고)
const SUMMARY_KEY: Record<string, string> = { count: "개수", transshipment: "환적", sample: "예시", reefer: "냉동", imdg_classes: "위험물 등급" };

/** 온톨로지 인스턴스 하나와 그 관계를 보여주는 패널. 관계 대상을 누르면 이동한다. */
export function EntityInspector() {
  const entity = useTerminal((s) => s.entity);
  const openEntity = useTerminal((s) => s.openEntity);
  const close = useTerminal((s) => s.closeEntity);
  if (!entity) return null;

  const show = (v: unknown) => {
    if (v === null || v === undefined) return "—";
    if (Array.isArray(v)) return v.join(", ");
    if (typeof v === "object") return JSON.stringify(v);
    return label(VALUE, String(v));
  };

  const Rel = ({ r, dir }: { r: EntityRelation; dir: "out" | "in" }) => {
    const { relation, target, ...rest } = r;
    const name = `${label(RELATION, relation)} (${relation})`;
    return (
      <li className="flex items-baseline gap-2 text-xs">
        <span className="shrink-0 text-muted">{dir === "out" ? `${name} →` : `← ${name}`}</span>
        {target ? (
          <button onClick={() => openEntity(target)} className="font-mono text-accent hover:underline">
            {target}
          </button>
        ) : (
          <span className="font-mono text-ink">
            {Object.entries(rest)
              .map(([k, v]) => `${label(SUMMARY_KEY, k)}: ${Array.isArray(v) ? v.join(", ") : String(v)}`)
              .join(" · ")}
          </span>
        )}
      </li>
    );
  };

  return (
    <div className="absolute inset-y-0 right-0 z-20 flex w-96 flex-col border-l border-line bg-panel shadow-2xl">
      <div className="flex items-center justify-between border-b border-line px-4 py-3">
        <div>
          <div className="text-[10px] tracking-widest text-muted">
            {label(ENTITY_TYPE, entity.type)} ({entity.type})
          </div>
          <div className="font-mono text-sm font-semibold">{entity.id}</div>
        </div>
        <button onClick={close} className="text-muted hover:text-ink" aria-label="닫기">
          ✕
        </button>
      </div>
      <div className="flex-1 space-y-4 overflow-y-auto p-4">
        <section>
          <h3 className="mb-2 text-xs font-semibold text-muted">속성</h3>
          <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
            {Object.entries(entity.properties)
              .filter(([k]) => k !== "id")
              .map(([k, v]) => (
                <div key={k} className="contents">
                  <dt className="text-muted">{label(PROPERTY, k)}</dt>
                  <dd className="font-mono break-all">{show(v)}</dd>
                </div>
              ))}
          </dl>
        </section>
        <section>
          <h3 className="mb-2 text-xs font-semibold text-muted">나가는 관계</h3>
          <ul className="space-y-1">{entity.outgoing.map((r, i) => <Rel key={i} r={r} dir="out" />)}</ul>
          {entity.outgoing.length === 0 && <p className="text-xs text-muted">없음</p>}
        </section>
        <section>
          <h3 className="mb-2 text-xs font-semibold text-muted">들어오는 관계</h3>
          <ul className="space-y-1">{entity.incoming.map((r, i) => <Rel key={i} r={r} dir="in" />)}</ul>
          {entity.incoming.length === 0 && <p className="text-xs text-muted">없음</p>}
        </section>
      </div>
    </div>
  );
}
