"use client";

import { useTerminal } from "@/store/useTerminal";

/** 모든 액션의 두 번째 단계: 검증된 계획을 사용자가 확인하고 적용한다. */
export function ProposalBar() {
  const p = useTerminal((s) => s.lastProposal);
  const applyPlan = useTerminal((s) => s.applyPlan);
  const loading = useTerminal((s) => s.loading);
  if (!p) return null;
  const dismiss = () => useTerminal.setState({ lastProposal: null });

  return (
    <div className={`flex items-center gap-4 border-t px-4 py-3 ${p.feasible ? "border-ok/50 bg-ok/10" : "border-root/50 bg-root/10"}`}>
      <div className="min-w-0 flex-1 text-sm">
        {p.feasible && p.plan ? (
          <>
            <span className="mr-2 font-mono text-xs text-ok">{p.plan.plan_id}</span>
            {p.plan.description}
            <span className="ml-2 text-xs text-muted">규칙 검증을 통과했습니다. 아직 반영되지 않았습니다.</span>
          </>
        ) : (
          <>
            <span className="mr-2 font-semibold text-root">불가</span>
            {p.violations?.map((v, i) => (
              <span key={i} className="mr-3">
                <span className="font-mono text-xs text-root">{v.rule}</span> {v.message}
              </span>
            ))}
          </>
        )}
      </div>
      {p.feasible && p.plan && (
        <button onClick={() => applyPlan(p.plan!.plan_id)} disabled={loading} className="rounded bg-ok px-3 py-1.5 text-sm font-semibold text-bg hover:brightness-110 disabled:opacity-50">
          계획 적용
        </button>
      )}
      <button onClick={dismiss} className="text-sm text-muted hover:text-ink">
        닫기
      </button>
    </div>
  );
}
