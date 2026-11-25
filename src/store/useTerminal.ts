"use client";

import { create } from "zustand";
import { callTool, listTools } from "@/lib/api";
import type { BerthSchedule, Entity, ImpactReport, Plan, Schema, ToolDef, Violation } from "@/lib/types";

export type Tab = "operations" | "ontology" | "mcp";

interface ProposalResult {
  feasible: boolean;
  plan?: Plan;
  violations?: Violation[];
}

interface TerminalState {
  tab: Tab;
  schedule: BerthSchedule | null;
  schema: Schema | null;
  tools: ToolDef[];
  rootVoyage: string;
  delayHours: number;
  impact: ImpactReport | null;
  plans: Plan[];
  auditLog: string[];
  lastProposal: (ProposalResult & { key: string }) | null;
  entity: Entity | null;
  loading: boolean;
  error: string | null;

  setTab: (t: Tab) => void;
  setScenario: (voyage: string, delay?: number) => void;
  load: () => Promise<void>;
  runImpact: () => Promise<void>;
  proposeBerth: (voyageId: string, berthId: string) => Promise<void>;
  proposeRebooking: (containerId: string, voyageId: string) => Promise<void>;
  applyPlan: (planId: string) => Promise<void>;
  reset: () => Promise<void>;
  openEntity: (id: string) => Promise<void>;
  closeEntity: () => void;
  clearError: () => void;
}

export const useTerminal = create<TerminalState>((set, get) => {
  // Wraps a backend call with loading/error handling.
  const run = async <T,>(fn: () => Promise<T>): Promise<T | undefined> => {
    set({ loading: true, error: null });
    try {
      return await fn();
    } catch (e) {
      set({ error: String(e) });
      return undefined;
    } finally {
      set({ loading: false });
    }
  };

  const refresh = async () => {
    const [schedule, plans] = await Promise.all([
      callTool<BerthSchedule>("get_berth_schedule"),
      callTool<{ plans: Plan[]; audit_log: string[] }>("list_plans"),
    ]);
    set({ schedule, plans: plans.plans, auditLog: plans.audit_log });
  };

  return {
    tab: "operations",
    schedule: null,
    schema: null,
    tools: [],
    rootVoyage: "V01",
    delayHours: 18,
    impact: null,
    plans: [],
    auditLog: [],
    lastProposal: null,
    entity: null,
    loading: false,
    error: null,

    setTab: (tab) => set({ tab }),
    setScenario: (rootVoyage, delay) => set((s) => ({ rootVoyage, delayHours: delay ?? s.delayHours })),

    load: () =>
      run(async () => {
        const [schema, tools] = await Promise.all([callTool<Schema>("describe_schema"), listTools()]);
        set({ schema, tools });
        await refresh();
      }),

    runImpact: () =>
      run(async () => {
        const { rootVoyage, delayHours } = get();
        const impact = await callTool<ImpactReport>("trace_delay_impact", { voyage_id: rootVoyage, delay_hours: delayHours });
        set({ impact, lastProposal: null });
      }),

    proposeBerth: (voyageId, berthId) =>
      run(async () => {
        const r = await callTool<ProposalResult>("propose_berth_reassignment", { voyage_id: voyageId, berth_id: berthId });
        set({ lastProposal: { ...r, key: `berth:${voyageId}:${berthId}` } });
        await refresh();
      }),

    proposeRebooking: (containerId, voyageId) =>
      run(async () => {
        const r = await callTool<ProposalResult>("propose_transshipment_rebooking", { container_id: containerId, voyage_id: voyageId });
        set({ lastProposal: { ...r, key: `rebook:${containerId}` } });
        await refresh();
      }),

    applyPlan: (planId) =>
      run(async () => {
        await callTool("apply_plan", { plan_id: planId });
        await refresh();
        // Re-run the scenario so the effect of the applied plan is visible.
        if (get().impact) {
          const { rootVoyage, delayHours } = get();
          set({ impact: await callTool<ImpactReport>("trace_delay_impact", { voyage_id: rootVoyage, delay_hours: delayHours }) });
        }
        set({ lastProposal: null });
      }),

    reset: () =>
      run(async () => {
        await callTool("reset_data");
        set({ impact: null, lastProposal: null, entity: null });
        await refresh();
      }),

    openEntity: (id) =>
      run(async () => {
        set({ entity: await callTool<Entity>("get_entity", { id }) });
      }),
    closeEntity: () => set({ entity: null }),
    clearError: () => set({ error: null }),
  };
});
