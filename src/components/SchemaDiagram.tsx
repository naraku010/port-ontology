"use client";

import type { Schema } from "@/lib/types";

/** 다이어그램 좌표계(viewBox) 크기. */
const W = 760;
const H = 640;
const NODE_W = 172;
const NODE_H = 62;

/**
 * 클래스 배치. 위에서 아래로 "선사 → 선박 → 항차 → 컨테이너 → 화주/블록" 흐름이 보이도록 손으로 정했다.
 * 클래스를 추가하면 여기에 좌표를 넣어야 다이어그램에 나타난다.
 */
const POS: Record<string, { x: number; y: number }> = {
  Operator: { x: 190, y: 70 },
  Vessel: { x: 190, y: 205 },
  Berth: { x: 570, y: 205 },
  Voyage: { x: 380, y: 335 },
  Container: { x: 380, y: 475 },
  Shipper: { x: 190, y: 590 },
  YardBlock: { x: 570, y: 590 },
};

/** 클래스별 강조색(테마 토큰). 인프라=녹색, 운항=청색, 일정=황색, 화물=보라. */
export const CLASS_COLOR: Record<string, string> = {
  Operator: "var(--color-accent)",
  Vessel: "var(--color-accent)",
  Berth: "var(--color-ok)",
  YardBlock: "var(--color-ok)",
  Voyage: "var(--color-shift)",
  Container: "var(--color-dg)",
  Shipper: "var(--color-reefer)",
};

/** 사각형 중심 c에서 방향 (dx,dy)로 나갈 때 테두리와 만나는 점. */
function edgePoint(c: { x: number; y: number }, dx: number, dy: number) {
  const t = Math.min(NODE_W / 2 / Math.abs(dx || 1e-9), NODE_H / 2 / Math.abs(dy || 1e-9));
  return { x: c.x + dx * t, y: c.y + dy * t };
}

interface Props {
  schema: Schema;
  selected: string;
  highlight: Set<string> | null; // 규칙에 마우스를 올렸을 때 강조할 클래스
  onSelect: (name: string) => void;
}

export function SchemaDiagram({ schema, selected, highlight, onSelect }: Props) {
  const dim = (name: string) => (highlight ? !highlight.has(name) : false);

  // 같은 두 클래스 사이의 관계가 여러 개면 나란히 띄워서 그린다.
  const pairCount = new Map<string, number>();
  const pairIndex = new Map<string, number>();
  const key = (a: string, b: string) => [a, b].sort().join("|");
  for (const r of schema.relations) pairCount.set(key(r.domain, r.range), (pairCount.get(key(r.domain, r.range)) ?? 0) + 1);

  const edges = schema.relations.flatMap((r) => {
    const a = POS[r.domain];
    const b = POS[r.range];
    if (!a || !b) return [];
    const k = key(r.domain, r.range);
    const n = pairCount.get(k)!;
    const i = pairIndex.get(k) ?? 0;
    pairIndex.set(k, i + 1);
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const len = Math.hypot(dx, dy);
    // 수직 방향으로 띄우는 거리
    const off = (i - (n - 1) / 2) * 104;
    const ox = (-dy / len) * off;
    const oy = (dx / len) * off;
    const p1 = edgePoint({ x: a.x + ox, y: a.y + oy }, dx, dy);
    const p2 = edgePoint({ x: b.x + ox, y: b.y + oy }, -dx, -dy);
    return [{ r, p1, p2, mid: { x: (p1.x + p2.x) / 2, y: (p1.y + p2.y) / 2 } }];
  });

  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="h-full w-full" role="img" aria-label="온톨로지 스키마 다이어그램">
      <defs>
        <marker id="schema-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
          <path d="M0,1 L10,5 L0,9 z" fill="var(--color-muted)" />
        </marker>
        <marker id="schema-arrow-on" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
          <path d="M0,1 L10,5 L0,9 z" fill="var(--color-ink)" />
        </marker>
      </defs>

      {edges.map(({ r, p1, p2 }) => {
        const on = r.domain === selected || r.range === selected;
        const faded = highlight ? !(highlight.has(r.domain) && highlight.has(r.range)) : false;
        return (
          <line
            key={r.name}
            x1={p1.x}
            y1={p1.y}
            x2={p2.x}
            y2={p2.y}
            stroke={on ? "var(--color-ink)" : "var(--color-line)"}
            strokeWidth={on ? 1.8 : 1.4}
            markerEnd={`url(#${on ? "schema-arrow-on" : "schema-arrow"})`}
            opacity={faded ? 0.2 : 1}
            className="transition-opacity"
          />
        );
      })}

      {edges.map(({ r, mid }) => {
        const on = r.domain === selected || r.range === selected;
        const faded = highlight ? !(highlight.has(r.domain) && highlight.has(r.range)) : false;
        const w = r.label.length * 12 + 18;
        return (
          <g key={`${r.name}-label`} transform={`translate(${mid.x},${mid.y})`} opacity={faded ? 0.2 : 1} className="transition-opacity">
            <title>{`${r.domain} —${r.name}→ ${r.range}`}</title>
            <rect x={-w / 2} y={-11} width={w} height={22} rx={11} fill="var(--color-bg)" stroke={on ? "var(--color-ink)" : "var(--color-line)"} />
            <text textAnchor="middle" y={4} fontSize={11.5} fill={on ? "var(--color-ink)" : "var(--color-muted)"}>
              {r.label}
            </text>
          </g>
        );
      })}

      {schema.classes.map((c) => {
        const p = POS[c.name];
        if (!p) return null;
        const color = CLASS_COLOR[c.name] ?? "var(--color-accent)";
        const isSel = c.name === selected;
        return (
          <g
            key={c.name}
            transform={`translate(${p.x - NODE_W / 2},${p.y - NODE_H / 2})`}
            onClick={() => onSelect(c.name)}
            className="cursor-pointer transition-opacity"
            opacity={dim(c.name) ? 0.25 : 1}
          >
            <rect width={NODE_W} height={NODE_H} rx={12} fill="var(--color-panel-2)" stroke={isSel ? color : "var(--color-line)"} strokeWidth={isSel ? 2.5 : 1.2} />
            <rect x={0} y={10} width={4} height={NODE_H - 20} rx={2} fill={color} />
            <text x={18} y={27} fontSize={16} fontWeight={700} fill="var(--color-ink)">
              {c.label}
            </text>
            <text x={18} y={46} fontSize={11} fill="var(--color-muted)" className="font-mono">
              {c.name}
            </text>
            <text x={NODE_W - 14} y={27} textAnchor="end" className="tabular-nums">
              <tspan fontSize={14} fontWeight={700} fill={color}>
                {c.instance_count}
              </tspan>
              <tspan fontSize={11} fill="var(--color-muted)" dx={2}>
                개
              </tspan>
            </text>
          </g>
        );
      })}
    </svg>
  );
}
