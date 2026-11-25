"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { useTerminal } from "@/store/useTerminal";
import type { GraphEdge, GraphNode } from "@/lib/types";
import { ENTITY_TYPE, label } from "@/lib/labels";

const NODE_W = 168;
const NODE_H = 46;
const COL_GAP = 70;
const ROW_GAP = 14;
const PAD = 16;

const KIND_STYLE: Record<GraphNode["kind"], { fill: string; stroke: string }> = {
  Event: { fill: "var(--color-root)", stroke: "var(--color-root)" },
  Voyage: { fill: "var(--color-panel-2)", stroke: "var(--color-accent)" },
  Berth: { fill: "var(--color-panel)", stroke: "var(--color-muted)" },
  ConnectionGroup: { fill: "var(--color-panel-2)", stroke: "var(--color-dg)" },
  Shipper: { fill: "var(--color-panel)", stroke: "var(--color-ok)" },
};

/** Layered layout with one barycenter pass to reduce edge crossings. */
function layout(nodes: GraphNode[], edges: GraphEdge[]) {
  const levels = new Map<number, GraphNode[]>();
  for (const n of nodes) levels.set(n.level, [...(levels.get(n.level) ?? []), n]);
  const sorted = [...levels.keys()].sort((a, b) => a - b);
  const maxRows = Math.max(1, ...[...levels.values()].map((r) => r.length));
  const pos = new Map<string, { x: number; y: number }>();
  sorted.forEach((lvl, col) => {
    const row = levels.get(lvl)!;
    if (col > 0) {
      const bary = (n: GraphNode) => {
        const ys = edges.filter((e) => e.to === n.id && pos.has(e.from)).map((e) => pos.get(e.from)!.y);
        return ys.length ? ys.reduce((a, b) => a + b, 0) / ys.length : Number.MAX_SAFE_INTEGER;
      };
      row.sort((a, b) => bary(a) - bary(b));
    }
    // Centre each column vertically against the tallest one.
    const offset = ((maxRows - row.length) * (NODE_H + ROW_GAP)) / 2;
    row.forEach((n, i) => pos.set(n.id, { x: PAD + col * (NODE_W + COL_GAP), y: PAD + offset + i * (NODE_H + ROW_GAP) }));
  });
  return {
    pos,
    width: PAD * 2 + sorted.length * NODE_W + (sorted.length - 1) * COL_GAP,
    height: PAD * 2 + maxRows * (NODE_H + ROW_GAP),
  };
}

export function ImpactGraph() {
  const impact = useTerminal((s) => s.impact);
  const openEntity = useTerminal((s) => s.openEntity);
  const [hover, setHover] = useState<string | null>(null);

  const g = impact?.graph;
  const { pos, width, height } = useMemo(() => layout(g?.nodes ?? [], g?.edges ?? []), [g]);
  const scroller = useRef<HTMLDivElement>(null);

  // 열을 세로 가운데 정렬하므로, 새 그래프가 그려지면 가운데(연쇄 경로가 있는 높이)로 스크롤한다.
  useEffect(() => {
    const el = scroller.current;
    if (el) el.scrollTo({ top: Math.max(0, (el.scrollHeight - el.clientHeight) / 2), left: 0 });
  }, [g]);

  if (!g) return <Empty />;

  const linked = (e: GraphEdge) => hover !== null && (e.from === hover || e.to === hover);

  return (
    <div ref={scroller} className="h-full overflow-auto">
      <svg width={width} height={height} className="block">
        <defs>
          <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
            <path d="M0,0 L10,5 L0,10 z" fill="var(--color-muted)" />
          </marker>
        </defs>
        {g.edges.map((e, i) => {
          const a = pos.get(e.from);
          const b = pos.get(e.to);
          if (!a || !b) return null;
          const x1 = a.x + NODE_W;
          const y1 = a.y + NODE_H / 2;
          const x2 = b.x;
          const y2 = b.y + NODE_H / 2;
          const mx = (x1 + x2) / 2;
          const on = linked(e);
          return (
            <g key={i} opacity={hover && !on ? 0.15 : 1}>
              <path
                d={`M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`}
                fill="none"
                stroke={on ? "var(--color-ink)" : "var(--color-line)"}
                strokeWidth={on ? 2 : 1.3}
                markerEnd="url(#arrow)"
              />
              {on && (
                <text x={mx} y={(y1 + y2) / 2 - 4} textAnchor="middle" fontSize="10" fill="var(--color-ink)">
                  {e.label}
                </text>
              )}
            </g>
          );
        })}
        {g.nodes.map((n) => {
          const p = pos.get(n.id)!;
          const st = KIND_STYLE[n.kind];
          const lines = n.label.split("\n");
          const entityId = n.kind === "Voyage" || n.kind === "Shipper" ? n.id : null;
          return (
            <g
              key={n.id}
              transform={`translate(${p.x},${p.y})`}
              onMouseEnter={() => setHover(n.id)}
              onMouseLeave={() => setHover(null)}
              onClick={() => entityId && openEntity(entityId)}
              className={entityId ? "cursor-pointer" : ""}
            >
              <rect width={NODE_W} height={NODE_H} rx={6} fill={st.fill} stroke={st.stroke} strokeWidth={hover === n.id ? 2.5 : 1.5} />
              <text x={8} y={13} fontSize="9" fill={n.kind === "Event" ? "var(--color-bg)" : "var(--color-muted)"}>
                {label(ENTITY_TYPE, n.kind)}
              </text>
              {lines.slice(0, 2).map((l, i) => (
                <text key={i} x={8} y={27 + i * 12} fontSize="11" fontWeight={i === 0 ? 600 : 400} fill={n.kind === "Event" ? "var(--color-bg)" : "var(--color-ink)"}>
                  {l.length > 26 ? `${l.slice(0, 25)}…` : l}
                </text>
              ))}
              <title>{n.label}</title>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

function Empty() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-muted">
      선석 계획에서 항차를 고르고 지연 시간을 정한 뒤 [영향 분석]을 누르세요. 노드에 마우스를 올리면 관계가 표시됩니다.
    </div>
  );
}
