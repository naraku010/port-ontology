//! Ontology classes for a container terminal.
//!
//! Instances are stored as typed structs; relations between them are the
//! `*_id` fields, and `Engine::relations_of` turns them into graph edges.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub type Time = NaiveDateTime;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Operator {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vessel {
    pub id: String,
    pub name: String,
    pub imo: String,
    pub operator_id: String,
    /// Length overall, metres.
    pub loa_m: f64,
    /// Maximum draft, metres.
    pub draft_m: f64,
    pub teu: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Berth {
    pub id: String,
    pub name: String,
    pub length_m: f64,
    /// Water depth alongside, metres.
    pub depth_m: f64,
    pub cranes: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Voyage {
    pub id: String,
    pub vessel_id: String,
    pub service: String,
    pub voyage_no: String,
    pub berth_id: String,
    pub eta: Time,
    pub etd: Time,
    pub prev_port: String,
    /// Ports called after this terminal, in order.
    pub next_ports: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct YardBlock {
    pub id: String,
    pub name: String,
    pub capacity_teu: u32,
    pub reefer_plugs: u32,
    /// Designated dangerous-goods area.
    pub dg_area: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shipper {
    pub id: String,
    pub name: String,
    /// "KEY" accounts get priority handling.
    pub tier: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub iso_type: String,
    pub reefer: bool,
    pub set_temp_c: Option<f64>,
    pub imdg_class: Option<u8>,
    pub weight_t: f64,
    pub shipper_id: String,
    pub inbound_voyage_id: Option<String>,
    pub outbound_voyage_id: Option<String>,
    pub destination: String,
    pub block_id: String,
}

impl Container {
    pub fn kind(&self) -> &'static str {
        match (&self.inbound_voyage_id, &self.outbound_voyage_id) {
            (Some(_), Some(_)) => "TRANSSHIPMENT",
            (Some(_), None) => "IMPORT",
            (None, Some(_)) => "EXPORT",
            (None, None) => "UNKNOWN",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ontology {
    pub operators: BTreeMap<String, Operator>,
    pub vessels: BTreeMap<String, Vessel>,
    pub berths: BTreeMap<String, Berth>,
    pub voyages: BTreeMap<String, Voyage>,
    pub blocks: BTreeMap<String, YardBlock>,
    pub shippers: BTreeMap<String, Shipper>,
    pub containers: BTreeMap<String, Container>,
}

/// Rule parameters. Kept in one place so the schema description and the
/// engine cannot drift apart.
pub mod rules {
    /// R1: under-keel clearance required at berth, metres.
    pub const UKC_M: f64 = 1.0;
    /// R2: minimum gap between consecutive vessels on one berth, hours.
    pub const BERTH_BUFFER_H: i64 = 1;
    /// R3: minimum connection time for transshipment, inbound ETA -> outbound ETD, hours.
    pub const MCT_H: i64 = 12;
    /// R5: IMDG class pairs that must not share a yard block (simplified table).
    pub const INCOMPATIBLE_IMDG: &[(u8, u8)] = &[(2, 5), (3, 5), (4, 5), (3, 8)];
    /// R5: classes that may not be stored at the terminal at all (direct delivery only).
    pub const DIRECT_DELIVERY_IMDG: &[u8] = &[1, 7];
}

/// Machine-readable description of the ontology, used by the UI schema view
/// and by the `describe_schema` MCP tool.
pub fn schema() -> Value {
    use rules::*;
    // name, 관계 이름은 식별자라 영어로 두고 설명(label, description)만 한글로 둔다.
    json!({
        "classes": [
            {"name": "Operator", "label": "선사", "description": "선박을 운항하는 해운사", "properties": ["name"]},
            {"name": "Vessel", "label": "선박", "description": "물리적 제원(전장, 흘수)을 가진 배", "properties": ["name", "imo", "loa_m", "draft_m", "teu"]},
            {"name": "Berth", "label": "선석", "description": "터미널 안벽의 접안 위치", "properties": ["name", "length_m", "depth_m", "cranes"]},
            {"name": "Voyage", "label": "항차", "description": "선박이 이 터미널에 한 번 기항하는 것", "properties": ["service", "voyage_no", "eta", "etd", "prev_port", "next_ports"]},
            {"name": "YardBlock", "label": "장치장 블록", "description": "컨테이너 보관 구역", "properties": ["capacity_teu", "reefer_plugs", "dg_area"]},
            {"name": "Shipper", "label": "화주", "description": "화물 소유자. tier=KEY는 핵심 화주", "properties": ["name", "tier"]},
            {"name": "Container", "label": "컨테이너", "description": "화물 단위. 종류(수입/수출/환적)는 항차 관계로부터 추론", "properties": ["iso_type", "reefer", "set_temp_c", "imdg_class", "weight_t", "destination"]}
        ],
        "relations": [
            {"name": "OPERATED_BY", "label": "운항사", "domain": "Vessel", "range": "Operator"},
            {"name": "PERFORMED_BY", "label": "투입 선박", "domain": "Voyage", "range": "Vessel"},
            {"name": "BERTHS_AT", "label": "접안 선석", "domain": "Voyage", "range": "Berth"},
            {"name": "DISCHARGED_FROM", "label": "양하 항차", "domain": "Container", "range": "Voyage"},
            {"name": "LOADED_ONTO", "label": "선적 항차", "domain": "Container", "range": "Voyage"},
            {"name": "STORED_IN", "label": "장치 위치", "domain": "Container", "range": "YardBlock"},
            {"name": "OWNED_BY", "label": "소유 화주", "domain": "Container", "range": "Shipper"}
        ],
        "rules": [
            // classes: 규칙이 참조하는 클래스. UI가 규칙과 클래스를 연결해 보여줄 때 쓴다.
            {"id": "R1", "name": "BerthFit", "label": "선석 제원 적합", "classes": ["Vessel", "Berth"], "statement": format!("선박 전장 <= 선석 길이, 그리고 선박 흘수 + 여유수심 {UKC_M}m <= 선석 수심")},
            {"id": "R2", "name": "BerthExclusive", "label": "선석 점유", "classes": ["Voyage", "Berth"], "statement": format!("같은 선석의 기항은 겹칠 수 없고 {BERTH_BUFFER_H}시간 이상 간격을 둔다. 앞 선박이 늦으면 뒤 선박이 모두 밀린다(접안 순서 유지).")},
            {"id": "R3", "name": "MinConnectionTime", "label": "최소 환적 시간", "classes": ["Container", "Voyage"], "statement": format!("환적 컨테이너는 출항편 ETD >= 입항편 ETA + {MCT_H}시간")},
            {"id": "R4", "name": "ReeferPower", "label": "냉동 전원", "classes": ["Container", "YardBlock"], "statement": "냉동 컨테이너는 남은 리퍼 플러그가 있는 블록에만 장치"},
            {"id": "R5", "name": "DangerousGoods", "label": "위험물", "classes": ["Container", "YardBlock"], "statement": format!("위험물은 위험물 구역에만 장치. {DIRECT_DELIVERY_IMDG:?}급은 직반출 대상. 비호환 등급 쌍 {INCOMPATIBLE_IMDG:?}은 같은 블록 금지(단순화한 표)")},
            {"id": "R6", "name": "Priority", "label": "우선순위", "classes": ["Container", "Shipper"], "statement": "냉동·위험물·핵심(KEY) 화주 화물의 환적 실패는 HIGH"}
        ]
    })
}
