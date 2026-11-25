// Mirrors the JSON returned by the Rust tool dispatcher (src-tauri/src/engine.rs).

export interface Berth {
  id: string;
  name: string;
  length_m: number;
  depth_m: number;
  cranes: number;
}

export interface BerthCall {
  voyage_id: string;
  vessel_id: string;
  vessel_name: string;
  service: string;
  voyage_no: string;
  eta: string;
  etd: string;
  loa_m: number;
  draft_m: number;
  next_ports: string[];
}

export interface BerthSchedule {
  plan_start: string;
  berths: { berth: Berth; calls: BerthCall[] }[];
}

export interface Violation {
  rule: string;
  message: string;
}

export interface VoyageShift {
  voyage_id: string;
  vessel_name: string;
  berth_id: string;
  old_eta: string;
  old_etd: string;
  new_eta: string;
  new_etd: string;
  delay_hours: number;
  caused_by: string | null;
  rule: string;
}

export interface MissedConnection {
  container_id: string;
  shipper_id: string;
  from_voyage: string;
  to_voyage: string;
  destination: string;
  short_by_hours: number;
  reefer: boolean;
  imdg_class: number | null;
  priority: "HIGH" | "NORMAL";
  priority_reasons: string[];
  alternative_voyage: string | null;
}

export interface ConnectionGroup {
  from_voyage: string;
  to_voyage: string;
  containers: number;
  reefer: number;
  dangerous_goods: number;
  high_priority: number;
  evidence_path: string[];
}

export interface ShipperImpact {
  shipper_id: string;
  name: string;
  tier: string;
  containers: number;
  reefer: number;
  dangerous_goods: number;
}

export interface BerthOption {
  voyage_id: string;
  berth_id: string;
  saves_hours: number;
  note: string;
}

export interface GraphNode {
  id: string;
  label: string;
  kind: "Event" | "Voyage" | "Berth" | "ConnectionGroup" | "Shipper";
  level: number;
}

export interface GraphEdge {
  from: string;
  to: string;
  label: string;
}

export interface ImpactReport {
  root_voyage: string;
  delay_hours: number;
  summary: string;
  voyage_shifts: VoyageShift[];
  connection_groups: ConnectionGroup[];
  missed_connections: MissedConnection[];
  shippers: ShipperImpact[];
  berth_options: BerthOption[];
  graph: { nodes: GraphNode[]; edges: GraphEdge[] };
}

export interface Plan {
  plan_id: string;
  description: string;
  applied: boolean;
  action: Record<string, unknown> & { type: string };
}

export interface Schema {
  classes: { name: string; label: string; description: string; properties: string[]; instance_count: number }[];
  relations: { name: string; label: string; domain: string; range: string }[];
  rules: { id: string; name: string; label: string; classes: string[]; statement: string }[];
}

export interface ToolDef {
  name: string;
  description: string;
  inputSchema: { properties: Record<string, { type: string; description?: string }>; required: string[] };
  annotations: { readOnlyHint: boolean };
}

export interface EntityRelation {
  relation: string;
  target?: string;
  count?: number;
  [k: string]: unknown;
}

export interface Entity {
  id: string;
  type: string;
  properties: Record<string, unknown>;
  outgoing: EntityRelation[];
  incoming: EntityRelation[];
}
