//! Reasoning over the ontology: relation traversal, rule checks, delay
//! impact propagation and two-step (propose -> apply) actions.
//!
//! Everything here is deterministic. The LLM only decides which of these
//! operations to call; it never writes queries or mutates state directly.

use crate::data;
use crate::model::{rules::*, *};
use chrono::Duration;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
pub struct Violation {
    pub rule: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct VoyageShift {
    pub voyage_id: String,
    pub vessel_name: String,
    pub berth_id: String,
    pub old_eta: Time,
    pub old_etd: Time,
    pub new_eta: Time,
    pub new_etd: Time,
    pub delay_hours: i64,
    /// Voyage whose delay caused this one, None for the root.
    pub caused_by: Option<String>,
    pub rule: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct MissedConnection {
    pub container_id: String,
    pub shipper_id: String,
    pub from_voyage: String,
    pub to_voyage: String,
    pub destination: String,
    /// Hours short of the minimum connection time (positive = missed by).
    pub short_by_hours: i64,
    pub reefer: bool,
    pub imdg_class: Option<u8>,
    pub priority: &'static str,
    pub priority_reasons: Vec<String>,
    pub alternative_voyage: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConnectionGroup {
    pub from_voyage: String,
    pub to_voyage: String,
    pub containers: usize,
    pub reefer: usize,
    pub dangerous_goods: usize,
    pub high_priority: usize,
    pub evidence_path: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ShipperImpact {
    pub shipper_id: String,
    pub name: String,
    pub tier: String,
    pub containers: usize,
    pub reefer: usize,
    pub dangerous_goods: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct BerthOption {
    pub voyage_id: String,
    pub berth_id: String,
    /// Delay this voyage avoids if moved, hours.
    pub saves_hours: i64,
    pub note: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub level: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImpactReport {
    pub root_voyage: String,
    pub delay_hours: i64,
    pub summary: String,
    pub voyage_shifts: Vec<VoyageShift>,
    pub connection_groups: Vec<ConnectionGroup>,
    pub missed_connections: Vec<MissedConnection>,
    pub shippers: Vec<ShipperImpact>,
    pub berth_options: Vec<BerthOption>,
    pub graph: Graph,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl Graph {
    fn node(&mut self, id: &str, label: String, kind: &str, level: usize) {
        if !self.nodes.iter().any(|n| n.id == id) {
            self.nodes.push(GraphNode { id: id.to_string(), label, kind: kind.to_string(), level });
        }
    }
    fn edge(&mut self, from: &str, to: &str, label: &str) {
        if !self.edges.iter().any(|e| e.from == from && e.to == to && e.label == label) {
            self.edges.push(GraphEdge { from: from.to_string(), to: to.to_string(), label: label.to_string() });
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum Action {
    ReassignBerth { voyage_id: String, from_berth: String, to_berth: String },
    RebookContainer { container_id: String, from_voyage: Option<String>, to_voyage: String },
}

#[derive(Clone, Debug, Serialize)]
pub struct Plan {
    pub plan_id: String,
    pub action: Action,
    pub description: String,
    pub applied: bool,
}

pub struct Engine {
    pub o: Ontology,
    plans: BTreeMap<String, Plan>,
    next_plan: u32,
    pub audit_log: Vec<String>,
}

type Schedule = BTreeMap<String, (Time, Time)>;

/// propose_* 결과에 붙는 안내. LLM이 사용자 확인 없이 apply_plan을 부르지 않도록 한다.
const NEXT_STEP: &str = "사용자가 확인한 뒤에만 이 plan_id로 apply_plan을 호출하세요.";

impl Engine {
    pub fn new() -> Self {
        Engine { o: data::seed(), plans: BTreeMap::new(), next_plan: 1, audit_log: Vec::new() }
    }

    pub fn reset(&mut self) {
        *self = Engine::new();
    }

    // ---------- lookup ----------

    fn voyage(&self, id: &str) -> Result<&Voyage, String> {
        self.o.voyages.get(id).ok_or_else(|| format!("존재하지 않는 항차 '{id}'"))
    }
    fn vessel_of(&self, v: &Voyage) -> &Vessel {
        &self.o.vessels[&v.vessel_id]
    }
    fn voyage_label(&self, id: &str) -> String {
        match self.o.voyages.get(id) {
            Some(v) => format!("{} {} ({})", self.vessel_of(v).name, v.voyage_no, v.id),
            None => id.to_string(),
        }
    }

    pub fn entity_type(&self, id: &str) -> Option<&'static str> {
        let o = &self.o;
        if o.voyages.contains_key(id) {
            Some("Voyage")
        } else if o.vessels.contains_key(id) {
            Some("Vessel")
        } else if o.berths.contains_key(id) {
            Some("Berth")
        } else if o.containers.contains_key(id) {
            Some("Container")
        } else if o.shippers.contains_key(id) {
            Some("Shipper")
        } else if o.blocks.contains_key(id) {
            Some("YardBlock")
        } else if o.operators.contains_key(id) {
            Some("Operator")
        } else {
            None
        }
    }

    /// Entity properties plus outgoing and incoming relations.
    /// Large fan-in (e.g. containers of a voyage) is summarised.
    pub fn get_entity(&self, id: &str) -> Result<Value, String> {
        let o = &self.o;
        let kind = self.entity_type(id).ok_or_else(|| format!("존재하지 않는 엔티티 '{id}'"))?;
        let mut out: Vec<Value> = Vec::new();
        let mut inc: Vec<Value> = Vec::new();
        let rel = |name: &str, target: &str| json!({"relation": name, "target": target});
        let props = match kind {
            "Voyage" => {
                let v = &o.voyages[id];
                out.push(rel("PERFORMED_BY", &v.vessel_id));
                out.push(rel("BERTHS_AT", &v.berth_id));
                let dis: Vec<&Container> = o.containers.values().filter(|c| c.inbound_voyage_id.as_deref() == Some(id)).collect();
                let load: Vec<&Container> = o.containers.values().filter(|c| c.outbound_voyage_id.as_deref() == Some(id)).collect();
                inc.push(json!({"relation": "DISCHARGED_FROM", "count": dis.len(), "transshipment": dis.iter().filter(|c| c.outbound_voyage_id.is_some()).count()}));
                inc.push(json!({"relation": "LOADED_ONTO", "count": load.len(), "transshipment": load.iter().filter(|c| c.inbound_voyage_id.is_some()).count()}));
                serde_json::to_value(v).unwrap()
            }
            "Vessel" => {
                let v = &o.vessels[id];
                out.push(rel("OPERATED_BY", &v.operator_id));
                for voy in o.voyages.values().filter(|x| x.vessel_id == id) {
                    inc.push(rel("PERFORMED_BY", &voy.id));
                }
                serde_json::to_value(v).unwrap()
            }
            "Berth" => {
                for voy in o.voyages.values().filter(|x| x.berth_id == id) {
                    inc.push(rel("BERTHS_AT", &voy.id));
                }
                serde_json::to_value(&o.berths[id]).unwrap()
            }
            "Container" => {
                let c = &o.containers[id];
                if let Some(v) = &c.inbound_voyage_id {
                    out.push(rel("DISCHARGED_FROM", v));
                }
                if let Some(v) = &c.outbound_voyage_id {
                    out.push(rel("LOADED_ONTO", v));
                }
                out.push(rel("STORED_IN", &c.block_id));
                out.push(rel("OWNED_BY", &c.shipper_id));
                let mut p = serde_json::to_value(c).unwrap();
                p["kind"] = json!(c.kind());
                p
            }
            "Shipper" => {
                let owned: Vec<&Container> = o.containers.values().filter(|c| c.shipper_id == id).collect();
                inc.push(json!({"relation": "OWNED_BY", "count": owned.len(), "sample": owned.iter().take(10).map(|c| &c.id).collect::<Vec<_>>()}));
                serde_json::to_value(&o.shippers[id]).unwrap()
            }
            "YardBlock" => {
                let stored: Vec<&Container> = o.containers.values().filter(|c| c.block_id == id).collect();
                inc.push(json!({"relation": "STORED_IN", "count": stored.len(), "reefer": stored.iter().filter(|c| c.reefer).count(),
                    "imdg_classes": stored.iter().filter_map(|c| c.imdg_class).collect::<BTreeSet<_>>()}));
                serde_json::to_value(&o.blocks[id]).unwrap()
            }
            "Operator" => {
                for v in o.vessels.values().filter(|v| v.operator_id == id) {
                    inc.push(rel("OPERATED_BY", &v.id));
                }
                serde_json::to_value(&o.operators[id]).unwrap()
            }
            _ => unreachable!(),
        };
        Ok(json!({"id": id, "type": kind, "properties": props, "outgoing": out, "incoming": inc}))
    }

    pub fn search(&self, query: &str, kind: Option<&str>, limit: usize) -> Value {
        let q = query.to_lowercase();
        let mut hits: Vec<Value> = Vec::new();
        let mut push = |t: &str, id: &str, label: &str| {
            if kind.is_none_or(|k| k.eq_ignore_ascii_case(t))
                && (id.to_lowercase().contains(&q) || label.to_lowercase().contains(&q))
            {
                hits.push(json!({"type": t, "id": id, "label": label}));
            }
        };
        let o = &self.o;
        for v in o.voyages.values() {
            push("Voyage", &v.id, &format!("{} {} {}", self.vessel_of(v).name, v.voyage_no, v.service));
        }
        for v in o.vessels.values() {
            push("Vessel", &v.id, &v.name);
        }
        for b in o.berths.values() {
            push("Berth", &b.id, &b.name);
        }
        for s in o.shippers.values() {
            push("Shipper", &s.id, &s.name);
        }
        for b in o.blocks.values() {
            push("YardBlock", &b.id, &b.name);
        }
        for op in o.operators.values() {
            push("Operator", &op.id, &op.name);
        }
        for c in o.containers.values() {
            let kind = match c.kind() {
                "IMPORT" => "수입",
                "EXPORT" => "수출",
                "TRANSSHIPMENT" => "환적",
                k => k,
            };
            let flags = format!("{}{}", if c.reefer { " · 냉동" } else { "" }, c.imdg_class.map(|k| format!(" · 위험물 {k}급")).unwrap_or_default());
            push("Container", &c.id, &format!("{kind} · {} · {}{flags}", c.iso_type, c.destination));
        }
        let total = hits.len();
        hits.truncate(limit);
        json!({"total": total, "results": hits})
    }

    pub fn berth_schedule(&self, berth: Option<&str>) -> Result<Value, String> {
        if let Some(b) = berth {
            if !self.o.berths.contains_key(b) {
                return Err(format!("존재하지 않는 선석 '{b}'"));
            }
        }
        let berths: Vec<Value> = self
            .o
            .berths
            .values()
            .filter(|b| berth.is_none_or(|x| x == b.id))
            .map(|b| {
                let mut calls: Vec<&Voyage> = self.o.voyages.values().filter(|v| v.berth_id == b.id).collect();
                calls.sort_by_key(|v| v.eta);
                json!({
                    "berth": b,
                    "calls": calls.iter().map(|v| {
                        let vs = self.vessel_of(v);
                        json!({"voyage_id": v.id, "vessel_id": vs.id, "vessel_name": vs.name, "service": v.service,
                               "voyage_no": v.voyage_no, "eta": v.eta, "etd": v.etd, "loa_m": vs.loa_m, "draft_m": vs.draft_m,
                               "next_ports": v.next_ports})
                    }).collect::<Vec<_>>()
                })
            })
            .collect();
        Ok(json!({"plan_start": data::base_time(), "berths": berths}))
    }

    // ---------- rules ----------

    /// R1 BerthFit.
    pub fn check_berth_fit(&self, vessel_id: &str, berth_id: &str) -> Result<Vec<Violation>, String> {
        let v = self.o.vessels.get(vessel_id).ok_or_else(|| format!("존재하지 않는 선박 '{vessel_id}'"))?;
        let b = self.o.berths.get(berth_id).ok_or_else(|| format!("존재하지 않는 선석 '{berth_id}'"))?;
        let mut out = Vec::new();
        if v.loa_m > b.length_m {
            out.push(Violation { rule: "R1", message: format!("{} 전장 {}m가 {} 길이 {}m를 초과", v.name, v.loa_m, b.name, b.length_m) });
        }
        if v.draft_m + UKC_M > b.depth_m {
            out.push(Violation {
                rule: "R1",
                message: format!("{} 흘수 {}m + 여유수심 {}m가 {} 수심 {}m를 초과", v.name, v.draft_m, UKC_M, b.name, b.depth_m),
            });
        }
        Ok(out)
    }

    /// R2 BerthExclusive: voyages at `berth_id` overlapping [eta, etd] (with buffer).
    fn berth_conflicts(&self, sched: &Schedule, berth_id: &str, eta: Time, etd: Time, exclude: &str) -> Vec<String> {
        let buf = Duration::hours(BERTH_BUFFER_H);
        self.o
            .voyages
            .values()
            .filter(|v| v.berth_id == berth_id && v.id != exclude)
            .filter(|v| {
                let (a, d) = sched[&v.id];
                a < etd + buf && eta < d + buf
            })
            .map(|v| v.id.clone())
            .collect()
    }

    /// R4 ReeferPower + R5 DangerousGoods for placing `container_id` in `block_id`.
    pub fn check_yard_placement(&self, container_id: &str, block_id: &str) -> Result<Vec<Violation>, String> {
        let c = self.o.containers.get(container_id).ok_or_else(|| format!("존재하지 않는 컨테이너 '{container_id}'"))?;
        let b = self.o.blocks.get(block_id).ok_or_else(|| format!("존재하지 않는 장치장 블록 '{block_id}'"))?;
        let others: Vec<&Container> = self.o.containers.values().filter(|x| x.block_id == b.id && x.id != c.id).collect();
        let mut out = Vec::new();
        if c.reefer {
            let used = others.iter().filter(|x| x.reefer).count() as u32;
            if b.reefer_plugs == 0 {
                out.push(Violation { rule: "R4", message: format!("{}에 리퍼 플러그가 없음", b.name) });
            } else if used >= b.reefer_plugs {
                out.push(Violation { rule: "R4", message: format!("{} 리퍼 플러그 부족 ({used}/{})", b.name, b.reefer_plugs) });
            }
        }
        if let Some(class) = c.imdg_class {
            if DIRECT_DELIVERY_IMDG.contains(&class) {
                out.push(Violation { rule: "R5", message: format!("위험물 {class}급은 직반출 대상") });
            }
            if !b.dg_area {
                out.push(Violation { rule: "R5", message: format!("위험물 {class}급은 위험물 구역 필요 ({}는 해당 없음)", b.name) });
            }
            let present: BTreeSet<u8> = others.iter().filter_map(|x| x.imdg_class).collect();
            for &(a, z) in INCOMPATIBLE_IMDG {
                let clash = (class == a && present.contains(&z)) || (class == z && present.contains(&a));
                if clash {
                    let other = if class == a { z } else { a };
                    out.push(Violation { rule: "R5", message: format!("위험물 {class}급은 {}에 이미 있는 {other}급과 격리 필요", b.name) });
                }
            }
        }
        Ok(out)
    }

    /// Audit every container placement and berth assignment against the rules.
    pub fn validate_all(&self) -> Value {
        let mut found: Vec<Value> = Vec::new();
        for v in self.o.voyages.values() {
            for viol in self.check_berth_fit(&v.vessel_id, &v.berth_id).unwrap_or_default() {
                found.push(json!({"entity": v.id, "rule": viol.rule, "message": viol.message}));
            }
        }
        let sched = self.current_schedule();
        for v in self.o.voyages.values() {
            for other in self.berth_conflicts(&sched, &v.berth_id, v.eta, v.etd, &v.id) {
                if v.id < other {
                    found.push(json!({"entity": v.id, "rule": "R2", "message": format!("{}와 {}가 {}에서 겹침", v.id, other, v.berth_id)}));
                }
            }
        }
        for c in self.o.containers.values() {
            if c.reefer || c.imdg_class.is_some() {
                // Capacity and segregation are block-level checks (reported below per block);
                // here only per-container type violations are reported.
                for viol in self.check_yard_placement(&c.id, &c.block_id).unwrap_or_default() {
                    if !viol.message.contains("플러그 부족") && !viol.message.contains("격리 필요") {
                        found.push(json!({"entity": c.id, "rule": viol.rule, "message": viol.message}));
                    }
                }
            }
            if let (Some(i), Some(out)) = (&c.inbound_voyage_id, &c.outbound_voyage_id) {
                let gap = (sched[out].1 - sched[i].0).num_hours();
                if gap < MCT_H {
                    found.push(json!({"entity": c.id, "rule": "R3", "message": format!("환적 {i} → {out} 간격 {gap}시간, 최소 {MCT_H}시간 미달")}));
                }
            }
        }
        for b in self.o.blocks.values() {
            let reefers = self.o.containers.values().filter(|c| c.block_id == b.id && c.reefer).count() as u32;
            if reefers > b.reefer_plugs {
                found.push(json!({"entity": b.id, "rule": "R4", "message": format!("냉동 컨테이너 {reefers}개, 리퍼 플러그 {}개", b.reefer_plugs)}));
            }
        }
        json!({"violations": found.len(), "details": found})
    }

    // ---------- delay impact ----------

    fn current_schedule(&self) -> Schedule {
        self.o.voyages.values().map(|v| (v.id.clone(), (v.eta, v.etd))).collect()
    }

    /// Apply a delay to one voyage and propagate it along each berth (R2).
    fn propagate(&self, root: &str, delay: Duration) -> (Schedule, Vec<VoyageShift>) {
        let base = self.current_schedule();
        let mut sched = base.clone();
        let (a, d) = sched[root];
        sched.insert(root.to_string(), (a + delay, d + delay));
        let mut causes: BTreeMap<String, String> = BTreeMap::new();

        // Push later calls at the same berth until the berth is conflict free.
        // Policy: the planned berthing sequence is kept, so a late vessel does
        // not lose its turn. Berths are independent, so one pass per berth suffices.
        for berth in self.o.berths.keys() {
            let mut calls: Vec<&Voyage> = self.o.voyages.values().filter(|v| &v.berth_id == berth).collect();
            calls.sort_by_key(|v| (base[&v.id].0, v.id.clone()));
            for w in 1..calls.len() {
                let prev = &calls[w - 1].id;
                let cur = &calls[w].id;
                let ready = sched[prev].1 + Duration::hours(BERTH_BUFFER_H);
                let (ca, cd) = sched[cur];
                if ca < ready {
                    let push = ready - ca;
                    sched.insert(cur.clone(), (ca + push, cd + push));
                    causes.insert(cur.clone(), prev.clone());
                }
            }
        }

        let mut shifts = Vec::new();
        for (id, (na, nd)) in &sched {
            let (oa, od) = base[id];
            if *na != oa || *nd != od {
                let v = &self.o.voyages[id];
                shifts.push(VoyageShift {
                    voyage_id: id.clone(),
                    vessel_name: self.vessel_of(v).name.clone(),
                    berth_id: v.berth_id.clone(),
                    old_eta: oa,
                    old_etd: od,
                    new_eta: *na,
                    new_etd: *nd,
                    delay_hours: (*na - oa).num_hours(),
                    caused_by: causes.get(id).cloned(),
                    rule: if id == root { "INPUT" } else { "R2" },
                });
            }
        }
        shifts.sort_by_key(|s| (s.caused_by.is_some(), s.new_eta));
        (sched, shifts)
    }

    fn priority(&self, c: &Container) -> (&'static str, Vec<String>) {
        let mut reasons = Vec::new();
        if c.reefer {
            reasons.push(format!("냉동 {}°C", c.set_temp_c.unwrap_or_default()));
        }
        if let Some(k) = c.imdg_class {
            reasons.push(format!("위험물 {k}급"));
        }
        if self.o.shippers[&c.shipper_id].tier == "KEY" {
            reasons.push("핵심 화주".into());
        }
        (if reasons.is_empty() { "NORMAL" } else { "HIGH" }, reasons)
    }

    /// Earliest voyage calling the container's destination that still makes MCT.
    fn alternative_voyage(&self, sched: &Schedule, c: &Container, exclude: &str) -> Option<String> {
        let inbound = c.inbound_voyage_id.as_ref()?;
        let earliest = sched[inbound].0 + Duration::hours(MCT_H);
        self.o
            .voyages
            .values()
            .filter(|v| v.id != exclude && &v.id != inbound && v.next_ports.contains(&c.destination))
            .filter(|v| sched[&v.id].1 >= earliest)
            .min_by_key(|v| sched[&v.id].1)
            .map(|v| v.id.clone())
    }

    pub fn trace_delay_impact(&self, voyage_id: &str, delay_hours: i64) -> Result<ImpactReport, String> {
        self.voyage(voyage_id)?;
        if !(1..=240).contains(&delay_hours) {
            return Err("delay_hours는 1~240 사이여야 합니다".into());
        }
        let base = self.current_schedule();
        let (sched, shifts) = self.propagate(voyage_id, Duration::hours(delay_hours));
        let shifted: BTreeSet<&str> = shifts.iter().map(|s| s.voyage_id.as_str()).collect();

        // R3: connections that held before and fail after.
        let mct = Duration::hours(MCT_H);
        let mut missed = Vec::new();
        for c in self.o.containers.values() {
            let (Some(i), Some(out)) = (&c.inbound_voyage_id, &c.outbound_voyage_id) else { continue };
            if !shifted.contains(i.as_str()) && !shifted.contains(out.as_str()) {
                continue;
            }
            let ok_before = base[out].1 >= base[i].0 + mct;
            let ok_after = sched[out].1 >= sched[i].0 + mct;
            if ok_before && !ok_after {
                let (priority, reasons) = self.priority(c);
                missed.push(MissedConnection {
                    container_id: c.id.clone(),
                    shipper_id: c.shipper_id.clone(),
                    from_voyage: i.clone(),
                    to_voyage: out.clone(),
                    destination: c.destination.clone(),
                    short_by_hours: ((sched[i].0 + mct) - sched[out].1).num_hours(),
                    reefer: c.reefer,
                    imdg_class: c.imdg_class,
                    priority,
                    priority_reasons: reasons,
                    alternative_voyage: self.alternative_voyage(&sched, c, out),
                });
            }
        }
        missed.sort_by_key(|m| (m.priority != "HIGH", m.from_voyage.clone(), m.to_voyage.clone(), m.container_id.clone()));

        // Group by connection, with an evidence path explaining the chain.
        let mut groups: BTreeMap<(String, String), ConnectionGroup> = BTreeMap::new();
        for m in &missed {
            let g = groups.entry((m.from_voyage.clone(), m.to_voyage.clone())).or_insert_with(|| ConnectionGroup {
                from_voyage: m.from_voyage.clone(),
                to_voyage: m.to_voyage.clone(),
                containers: 0,
                reefer: 0,
                dangerous_goods: 0,
                high_priority: 0,
                evidence_path: self.evidence_path(&shifts, voyage_id, &m.from_voyage, &m.to_voyage),
            });
            g.containers += 1;
            g.reefer += m.reefer as usize;
            g.dangerous_goods += m.imdg_class.is_some() as usize;
            g.high_priority += (m.priority == "HIGH") as usize;
        }
        let connection_groups: Vec<ConnectionGroup> = groups.into_values().collect();

        let mut by_shipper: BTreeMap<String, ShipperImpact> = BTreeMap::new();
        for m in &missed {
            let s = &self.o.shippers[&m.shipper_id];
            let e = by_shipper.entry(s.id.clone()).or_insert_with(|| ShipperImpact {
                shipper_id: s.id.clone(),
                name: s.name.clone(),
                tier: s.tier.clone(),
                containers: 0,
                reefer: 0,
                dangerous_goods: 0,
            });
            e.containers += 1;
            e.reefer += m.reefer as usize;
            e.dangerous_goods += m.imdg_class.is_some() as usize;
        }
        let mut shippers: Vec<ShipperImpact> = by_shipper.into_values().collect();
        shippers.sort_by_key(|s| (s.tier != "KEY", std::cmp::Reverse(s.containers)));

        // Mitigation: move a pushed voyage to another berth that fits (R1) and
        // is free (R2) over its original window.
        let mut berth_options = Vec::new();
        for s in shifts.iter().filter(|s| s.caused_by.is_some()) {
            let v = &self.o.voyages[&s.voyage_id];
            for b in self.o.berths.values().filter(|b| b.id != v.berth_id) {
                if !self.check_berth_fit(&v.vessel_id, &b.id)?.is_empty() {
                    continue;
                }
                if self.berth_conflicts(&sched, &b.id, s.old_eta, s.old_etd, &v.id).is_empty() {
                    berth_options.push(BerthOption {
                        voyage_id: v.id.clone(),
                        berth_id: b.id.clone(),
                        saves_hours: s.delay_hours,
                        note: format!("{}는 {} 제원에 맞고, 원래 일정 {} ~ {} 동안 비어 있음", self.vessel_of(v).name, b.name, s.old_eta.format("%m-%d %H:%M"), s.old_etd.format("%m-%d %H:%M")),
                    });
                }
            }
        }

        let graph = self.impact_graph(voyage_id, delay_hours, &shifts, &connection_groups, &missed);
        let high = missed.iter().filter(|m| m.priority == "HIGH").count();
        let summary = format!(
            "{}가 {}시간 지연됩니다. 선석 점유(R2)로 다른 항차 {}건이 밀리고, 환적 컨테이너 {}개가 연결편을 놓칩니다(R3). 그중 우선순위 높음 {}개이며 화주 {}곳이 영향을 받습니다. 선석 변경 대안 {}건.",
            self.voyage_label(voyage_id),
            delay_hours,
            shifts.len().saturating_sub(1),
            missed.len(),
            high,
            shippers.len(),
            berth_options.len()
        );
        Ok(ImpactReport {
            root_voyage: voyage_id.to_string(),
            delay_hours,
            summary,
            voyage_shifts: shifts,
            connection_groups,
            missed_connections: missed,
            shippers,
            berth_options,
            graph,
        })
    }

    fn evidence_path(&self, shifts: &[VoyageShift], root: &str, from: &str, to: &str) -> Vec<String> {
        // Walk back from whichever side of the connection was moved to the root.
        let chain_to_root = |start: &str| -> Vec<String> {
            let mut chain = vec![start.to_string()];
            let mut cur = start.to_string();
            while let Some(s) = shifts.iter().find(|s| s.voyage_id == cur) {
                match &s.caused_by {
                    Some(prev) => {
                        chain.push(format!("{} (R2 선석 점유)", s.berth_id));
                        chain.push(prev.clone());
                        cur = prev.clone();
                    }
                    None => break,
                }
            }
            chain.reverse();
            chain
        };
        let moved = if shifts.iter().any(|s| s.voyage_id == from) { from } else { to };
        let mut path = vec![format!("{root} 지연 발생")];
        path.extend(chain_to_root(moved).into_iter().map(|p| if self.o.voyages.contains_key(&p) { self.voyage_label(&p) } else { p }));
        if moved == from {
            path.push(format!("{} 환적 연결 실패 (R3 최소 환적 시간)", self.voyage_label(to)));
        } else {
            path.push(format!("{} 화물 준비 전에 출항 (R3 최소 환적 시간)", self.voyage_label(from)));
        }
        path
    }

    fn impact_graph(&self, root: &str, delay: i64, shifts: &[VoyageShift], groups: &[ConnectionGroup], missed: &[MissedConnection]) -> Graph {
        let mut g = Graph::default();
        let ev = "EVENT";
        g.node(ev, format!("지연 +{delay}시간"), "Event", 0);
        // Level by cascade depth: root voyage = 1, each R2 hop adds 2 (berth in between).
        let depth = |id: &str| -> usize {
            let mut d = 0;
            let mut cur = id.to_string();
            while let Some(prev) = shifts.iter().find(|s| s.voyage_id == cur).and_then(|s| s.caused_by.clone()) {
                d += 1;
                cur = prev;
            }
            d
        };
        let mut max_level = 1;
        for s in shifts {
            let lvl = 1 + 2 * depth(&s.voyage_id);
            max_level = max_level.max(lvl);
            let v = &self.o.voyages[&s.voyage_id];
            g.node(&s.voyage_id, format!("{} {}\n+{}시간", self.vessel_of(v).name, v.voyage_no, s.delay_hours), "Voyage", lvl);
            match &s.caused_by {
                None => g.edge(ev, &s.voyage_id, "지연"),
                Some(prev) => {
                    let bnode = format!("{}@{}", s.berth_id, prev);
                    g.node(&bnode, self.o.berths[&s.berth_id].name.clone(), "Berth", lvl - 1);
                    g.edge(prev, &bnode, "선석 점유");
                    g.edge(&bnode, &s.voyage_id, "접안 대기 (R2)");
                }
            }
        }
        let _ = root;
        let glevel = max_level + 1;
        for grp in groups {
            let gid = format!("CONN:{}>{}", grp.from_voyage, grp.to_voyage);
            let mut label = format!("환적 실패 {}개", grp.containers);
            let mut extra = Vec::new();
            if grp.reefer > 0 {
                extra.push(format!("냉동 {}", grp.reefer));
            }
            if grp.dangerous_goods > 0 {
                extra.push(format!("위험물 {}", grp.dangerous_goods));
            }
            if !extra.is_empty() {
                label.push('\n');
                label.push_str(&extra.join(" · "));
            }
            g.node(&gid, label, "ConnectionGroup", glevel);
            if !g.nodes.iter().any(|n| n.id == grp.from_voyage) {
                g.node(&grp.from_voyage, self.voyage_label(&grp.from_voyage), "Voyage", glevel - 1);
            }
            g.edge(&grp.from_voyage, &gid, "양하 (DISCHARGED_FROM)");
            if !g.nodes.iter().any(|n| n.id == grp.to_voyage) {
                g.node(&grp.to_voyage, self.voyage_label(&grp.to_voyage), "Voyage", glevel + 1);
            }
            g.edge(&gid, &grp.to_voyage, "연결 실패 (R3)");
        }
        let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
        for m in missed {
            let gid = format!("CONN:{}>{}", m.from_voyage, m.to_voyage);
            if seen.insert((gid.clone(), m.shipper_id.clone())) {
                let s = &self.o.shippers[&m.shipper_id];
                g.node(&s.id, format!("{}{}", s.name, if s.tier == "KEY" { " (핵심)" } else { "" }), "Shipper", glevel + 2);
                g.edge(&gid, &s.id, "소유 화주 (OWNED_BY)");
            }
        }
        g
    }

    // ---------- actions (propose -> apply) ----------

    fn new_plan(&mut self, action: Action, description: String) -> Plan {
        let plan = Plan { plan_id: format!("PLAN-{:03}", self.next_plan), action, description, applied: false };
        self.next_plan += 1;
        self.plans.insert(plan.plan_id.clone(), plan.clone());
        plan
    }

    pub fn propose_berth_reassignment(&mut self, voyage_id: &str, berth_id: &str) -> Result<Value, String> {
        let v = self.voyage(voyage_id)?.clone();
        if !self.o.berths.contains_key(berth_id) {
            return Err(format!("존재하지 않는 선석 '{berth_id}'"));
        }
        if v.berth_id == berth_id {
            return Err(format!("{voyage_id}는 이미 {berth_id}에 배정되어 있습니다"));
        }
        let mut violations = self.check_berth_fit(&v.vessel_id, berth_id)?;
        let conflicts = self.berth_conflicts(&self.current_schedule(), berth_id, v.eta, v.etd, &v.id);
        for c in &conflicts {
            violations.push(Violation { rule: "R2", message: format!("{berth_id}에서 {}와 일정이 겹침", self.voyage_label(c)) });
        }
        if !violations.is_empty() {
            return Ok(json!({"feasible": false, "violations": violations}));
        }
        let desc = format!(
            "{} 선석 변경 {} → {} ({} ~ {})",
            self.voyage_label(voyage_id),
            v.berth_id,
            berth_id,
            v.eta.format("%m-%d %H:%M"),
            v.etd.format("%m-%d %H:%M")
        );
        let plan = self.new_plan(Action::ReassignBerth { voyage_id: v.id.clone(), from_berth: v.berth_id.clone(), to_berth: berth_id.into() }, desc);
        Ok(json!({"feasible": true, "plan": plan, "next_step": NEXT_STEP}))
    }

    pub fn propose_rebooking(&mut self, container_id: &str, voyage_id: &str) -> Result<Value, String> {
        let c = self.o.containers.get(container_id).ok_or_else(|| format!("존재하지 않는 컨테이너 '{container_id}'"))?.clone();
        let target = self.voyage(voyage_id)?.clone();
        let mut violations = Vec::new();
        if c.outbound_voyage_id.as_deref() == Some(voyage_id) {
            return Err(format!("{container_id}는 이미 {voyage_id}에 선적 예정입니다"));
        }
        if c.outbound_voyage_id.is_none() {
            violations.push(Violation { rule: "R3", message: "수입 컨테이너라 변경할 출항편이 없음".into() });
        }
        if !target.next_ports.contains(&c.destination) {
            violations.push(Violation { rule: "ROUTE", message: format!("{voyage_id}는 {}에 기항하지 않음", c.destination) });
        }
        if let Some(i) = &c.inbound_voyage_id {
            let inbound = self.voyage(i)?;
            if target.etd < inbound.eta + Duration::hours(MCT_H) {
                violations.push(Violation { rule: "R3", message: format!("{voyage_id}는 {i} 입항 후 최소 환적 시간 전에 출항") });
            }
        }
        if !violations.is_empty() {
            return Ok(json!({"feasible": false, "violations": violations}));
        }
        let desc = format!(
            "{container_id} ({}) 선적 항차 변경 {} → {}",
            c.destination,
            c.outbound_voyage_id.clone().unwrap_or_default(),
            self.voyage_label(voyage_id)
        );
        let plan = self.new_plan(Action::RebookContainer { container_id: c.id.clone(), from_voyage: c.outbound_voyage_id.clone(), to_voyage: voyage_id.into() }, desc);
        Ok(json!({"feasible": true, "plan": plan, "next_step": NEXT_STEP}))
    }

    pub fn apply_plan(&mut self, plan_id: &str) -> Result<Value, String> {
        let plan = self.plans.get(plan_id).ok_or_else(|| format!("존재하지 않는 계획 '{plan_id}'"))?.clone();
        if plan.applied {
            return Err(format!("{plan_id}는 이미 적용된 계획입니다"));
        }
        // Re-validate: state may have changed since the proposal.
        match &plan.action {
            Action::ReassignBerth { voyage_id, to_berth, .. } => {
                let v = self.voyage(voyage_id)?.clone();
                let mut problems = self.check_berth_fit(&v.vessel_id, to_berth)?;
                for c in self.berth_conflicts(&self.current_schedule(), to_berth, v.eta, v.etd, &v.id) {
                    problems.push(Violation { rule: "R2", message: format!("그사이 {c}와 일정이 겹치게 됨") });
                }
                if !problems.is_empty() {
                    return Ok(json!({"applied": false, "violations": problems}));
                }
                self.o.voyages.get_mut(voyage_id).unwrap().berth_id = to_berth.clone();
            }
            Action::RebookContainer { container_id, to_voyage, .. } => {
                self.o.containers.get_mut(container_id).unwrap().outbound_voyage_id = Some(to_voyage.clone());
            }
        }
        self.plans.get_mut(plan_id).unwrap().applied = true;
        self.audit_log.push(format!("{plan_id} 적용: {}", plan.description));
        Ok(json!({"applied": true, "plan": self.plans[plan_id]}))
    }

    pub fn list_plans(&self) -> Value {
        json!({"plans": self.plans.values().collect::<Vec<_>>(), "audit_log": self.audit_log})
    }

    pub fn draft_delay_notice(&self, shipper_id: &str, voyage_id: &str, delay_hours: i64) -> Result<Value, String> {
        let s = self.o.shippers.get(shipper_id).ok_or_else(|| format!("존재하지 않는 화주 '{shipper_id}'"))?;
        let r = self.trace_delay_impact(voyage_id, delay_hours)?;
        let mine: Vec<&MissedConnection> = r.missed_connections.iter().filter(|m| m.shipper_id == shipper_id).collect();
        if mine.is_empty() {
            return Ok(json!({"affected": false, "message": format!("이 시나리오에서 {}의 환적 실패 컨테이너는 없습니다", s.name)}));
        }
        let mut lines = vec![
            format!("수신: {} 담당자님", s.name),
            format!("제목: [환적 지연 안내] {}", self.voyage_label(voyage_id)),
            String::new(),
            format!("{}의 입항이 약 {}시간 지연될 예정입니다. 이에 따라 아래 컨테이너가 예정된 연결 항차에 선적되지 못합니다.", self.voyage_label(voyage_id), delay_hours),
            String::new(),
        ];
        for m in &mine {
            let alt = m.alternative_voyage.as_deref().map(|a| self.voyage_label(a)).unwrap_or_else(|| "추후 안내".into());
            let mut flags = String::new();
            if m.reefer {
                flags.push_str(" [냉동 - 전원 공급 유지 중]");
            }
            if let Some(k) = m.imdg_class {
                flags.push_str(&format!(" [위험물 {k}급]"));
            }
            lines.push(format!("- {} (목적지 {}): 예정 {} → 대체 {}{}", m.container_id, m.destination, self.voyage_label(&m.to_voyage), alt, flags));
        }
        lines.push(String::new());
        lines.push("대체 항차는 확정 전이며, 선박 접안 후 다시 안내드리겠습니다.".into());
        Ok(json!({"affected": true, "containers": mine.len(), "draft": lines.join("\n"), "note": "초안입니다. 발송되지 않았습니다."}))
    }
}


#[cfg(test)]
mod tests {
    //! 시드 데이터 기준 회귀 테스트. 시드(data.rs)를 바꾸면 수치 기대값도 함께 확인할 것.
    use super::*;

    #[test]
    fn seed_data_satisfies_all_rules() {
        let e = Engine::new();
        assert_eq!(e.validate_all()["violations"], 0, "{}", e.validate_all());
    }

    #[test]
    fn delay_propagates_along_berth_in_order() {
        let e = Engine::new();
        let r = e.trace_delay_impact("V01", 18).unwrap();
        let chain: Vec<(&str, Option<&str>)> = r.voyage_shifts.iter().map(|s| (s.voyage_id.as_str(), s.caused_by.as_deref())).collect();
        assert_eq!(chain, vec![("V01", None), ("V05", Some("V01")), ("V09", Some("V05"))]);
        // 연쇄가 커질수록 밀리는 시간은 줄어든다(사이 간격이 흡수).
        assert!(r.voyage_shifts[1].delay_hours <= 18 && r.voyage_shifts[2].delay_hours <= r.voyage_shifts[1].delay_hours);
        assert!(!r.missed_connections.is_empty());
        assert!(r.connection_groups.iter().all(|g| g.evidence_path.len() >= 3));
    }

    #[test]
    fn trace_does_not_change_state() {
        let e = Engine::new();
        let before = e.berth_schedule(None).unwrap();
        e.trace_delay_impact("V02", 30).unwrap();
        assert_eq!(before, e.berth_schedule(None).unwrap());
    }

    #[test]
    fn berth_fit_rejects_ultra_large_vessel_on_b1() {
        let e = Engine::new();
        let v = e.check_berth_fit("VSL-03", "B1").unwrap();
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|x| x.rule == "R1"));
        assert!(e.check_berth_fit("VSL-05", "B6").unwrap().is_empty());
    }

    #[test]
    fn suggested_berth_move_reduces_missed_connections() {
        let mut e = Engine::new();
        let before = e.trace_delay_impact("V01", 18).unwrap();
        let opt = before.berth_options.first().expect("seed should offer a berth option").clone();
        assert_eq!((opt.voyage_id.as_str(), opt.berth_id.as_str()), ("V05", "B6"));

        let p = e.propose_berth_reassignment(&opt.voyage_id, &opt.berth_id).unwrap();
        assert_eq!(p["feasible"], true);
        // 제안만으로는 아무것도 바뀌지 않는다.
        assert_eq!(e.o.voyages["V05"].berth_id, "B1");

        let id = p["plan"]["plan_id"].as_str().unwrap().to_string();
        assert_eq!(e.apply_plan(&id).unwrap()["applied"], true);
        assert_eq!(e.o.voyages["V05"].berth_id, "B6");
        assert!(e.apply_plan(&id).is_err(), "a plan must not apply twice");

        let after = e.trace_delay_impact("V01", 18).unwrap();
        assert!(after.missed_connections.len() < before.missed_connections.len());
        assert_eq!(e.validate_all()["violations"], 0);
    }

    #[test]
    fn infeasible_proposal_returns_violations_without_plan() {
        let mut e = Engine::new();
        let p = e.propose_berth_reassignment("V05", "B4").unwrap();
        assert_eq!(p["feasible"], false);
        assert!(p.get("plan").is_none());
        assert!(e.list_plans()["plans"].as_array().unwrap().is_empty());
    }

    #[test]
    fn rebooking_checks_route_and_connection_time() {
        let mut e = Engine::new();
        let r = e.trace_delay_impact("V01", 18).unwrap();
        let m = r.missed_connections.iter().find(|m| m.alternative_voyage.is_some()).unwrap();
        // 대체 항차는 현재 일정에서 MCT를 만족하므로 제안이 통과해야 한다.
        let ok = e.propose_rebooking(&m.container_id, m.alternative_voyage.as_ref().unwrap()).unwrap();
        assert_eq!(ok["feasible"], true);
        // 목적지에 기항하지 않는 항차는 거부된다.
        let bad = e.propose_rebooking(&m.container_id, "V02").unwrap();
        assert_eq!(bad["feasible"], false);
    }
}
