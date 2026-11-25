//! 데스크톱 UI(Tauri command)와 LLM CLI(MCP 서버)가 함께 쓰는 도구 목록.
//! 둘 다 `dispatch`를 거치므로 버튼과 도구 호출의 동작이 항상 같다.
//! 도구를 추가할 때는 definitions()와 dispatch() 두 곳을 함께 고치고,
//! 상태를 바꾸지 않는 도구라면 READ_ONLY에도 넣는다. (docs/mcp-tools.md 참고)

use crate::engine::Engine;
use crate::model;
use serde_json::{Value, json};

/// 상태를 읽기만 하는 도구. CLI에서 자동 허용해도 되는 목록이며,
/// 나머지는 상태를 바꾸므로 사용자 확인이 필요하다.
pub const READ_ONLY: &[&str] = &[
    "describe_schema",
    "search_entities",
    "get_entity",
    "get_berth_schedule",
    "trace_delay_impact",
    "check_berth_fit",
    "check_yard_placement",
    "validate_all",
    "draft_delay_notice",
    "list_plans",
];

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {"type": "object", "properties": properties, "required": required},
        "annotations": {"readOnlyHint": READ_ONLY.contains(&name)}
    })
}

pub fn definitions() -> Vec<Value> {
    let id = |d: &str| json!({"type": "string", "description": d});
    let hours = json!({"type": "integer", "minimum": 1, "maximum": 240, "description": "지연 시간(시간 단위)"});
    vec![
        tool("describe_schema", "온톨로지의 클래스, 관계, 규칙(R1~R6)을 반환합니다. 처음에 호출해 모델을 파악하세요.", json!({}), &[]),
        tool(
            "search_entities",
            "ID나 이름 일부로 엔티티를 찾습니다. type: Voyage, Vessel, Berth, Container, Shipper, YardBlock, Operator.",
            json!({"query": id("찾을 문자열(대소문자 무시)"), "type": id("선택: 엔티티 타입 필터"), "limit": {"type": "integer", "default": 20}}),
            &["query"],
        ),
        tool("get_entity", "엔티티 하나의 속성과 들어오고 나가는 관계를 반환합니다.", json!({"id": id("엔티티 ID. 예: V01, VSL-03, B2, SH-02")}), &["id"]),
        tool("get_berth_schedule", "선석별 기항 계획(항차, 선박, ETA/ETD)을 반환합니다. berth_id를 생략하면 전체 선석.", json!({"berth_id": id("선택: 선석 ID. 예: B1")}), &[]),
        tool(
            "trace_delay_impact",
            "항차 하나의 지연을 시뮬레이션합니다. 같은 선석의 뒤 항차를 밀고(R2), 연결편을 놓치는 환적 컨테이너를 찾고(R3), 우선순위를 매기고(R6), 대체 항차와 선석 변경 대안을 제시합니다. 근거 경로(evidence_path)를 포함하며 상태는 바꾸지 않습니다.",
            json!({"voyage_id": id("지연되는 항차 ID. 예: V01"), "delay_hours": hours}),
            &["voyage_id", "delay_hours"],
        ),
        tool("check_berth_fit", "선박이 선석에 접안 가능한지 R1(길이, 흘수)을 검사합니다.", json!({"vessel_id": id("선박 ID"), "berth_id": id("선석 ID")}), &["vessel_id", "berth_id"]),
        tool(
            "check_yard_placement",
            "컨테이너를 장치장 블록에 둘 수 있는지 R4(냉동 전원), R5(위험물)를 검사합니다.",
            json!({"container_id": id("컨테이너 번호"), "block_id": id("장치장 블록 ID")}),
            &["container_id", "block_id"],
        ),
        tool("validate_all", "터미널 전체 상태를 모든 규칙으로 감사합니다.", json!({}), &[]),
        tool(
            "draft_delay_notice",
            "지연 시나리오에서 특정 화주에게 보낼 지연 안내문 초안을 만듭니다. 발송하지 않습니다.",
            json!({"shipper_id": id("화주 ID"), "voyage_id": id("지연되는 항차 ID"), "delay_hours": hours}),
            &["shipper_id", "voyage_id", "delay_hours"],
        ),
        tool(
            "propose_berth_reassignment",
            "항차를 다른 선석으로 옮길 수 있는지 R1, R2로 검증합니다. 가능하면 plan_id를 발급하며, apply_plan 전까지 아무것도 바뀌지 않습니다.",
            json!({"voyage_id": id("항차 ID"), "berth_id": id("옮길 선석 ID")}),
            &["voyage_id", "berth_id"],
        ),
        tool(
            "propose_transshipment_rebooking",
            "환적 컨테이너를 다른 출항 항차로 바꿀 수 있는지 항로와 R3로 검증합니다. 가능하면 plan_id를 발급하며, apply_plan 전까지 아무것도 바뀌지 않습니다.",
            json!({"container_id": id("컨테이너 번호"), "voyage_id": id("바꿀 출항 항차 ID")}),
            &["container_id", "voyage_id"],
        ),
        tool("apply_plan", "제안된 계획을 실제로 반영합니다. 반드시 사용자가 명시적으로 확인한 뒤에만 호출하세요.", json!({"plan_id": id("계획 ID. 예: PLAN-001")}), &["plan_id"]),
        tool("list_plans", "제안·적용된 계획 목록과 감사 로그를 반환합니다.", json!({}), &[]),
        tool("reset_data", "시드 데이터로 되돌리고 모든 계획을 지웁니다.", json!({}), &[]),
    ]
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key).and_then(Value::as_str).ok_or_else(|| format!("문자열 인자 '{key}'가 필요합니다"))
}

fn int_arg(args: &Value, key: &str) -> Result<i64, String> {
    // LLM이 숫자를 문자열이나 실수로 보내는 경우도 받아준다.
    args.get(key)
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)).or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .ok_or_else(|| format!("정수 인자 '{key}'가 필요합니다"))
}

fn to_value<T: serde::Serialize>(v: T) -> Result<Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

pub fn dispatch(engine: &mut Engine, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "describe_schema" => {
            // 정적 스키마에 현재 인스턴스 수를 붙여서 돌려준다.
            let mut schema = model::schema();
            let o = &engine.o;
            for class in schema["classes"].as_array_mut().unwrap() {
                let count = match class["name"].as_str().unwrap() {
                    "Operator" => o.operators.len(),
                    "Vessel" => o.vessels.len(),
                    "Berth" => o.berths.len(),
                    "Voyage" => o.voyages.len(),
                    "YardBlock" => o.blocks.len(),
                    "Shipper" => o.shippers.len(),
                    "Container" => o.containers.len(),
                    _ => 0,
                };
                class["instance_count"] = json!(count);
            }
            Ok(schema)
        }
        "search_entities" => {
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20).min(200) as usize;
            Ok(engine.search(str_arg(args, "query")?, args.get("type").and_then(Value::as_str), limit))
        }
        "get_entity" => engine.get_entity(str_arg(args, "id")?),
        "get_berth_schedule" => engine.berth_schedule(args.get("berth_id").and_then(Value::as_str)),
        "trace_delay_impact" => to_value(engine.trace_delay_impact(str_arg(args, "voyage_id")?, int_arg(args, "delay_hours")?)?),
        "check_berth_fit" => {
            let v = engine.check_berth_fit(str_arg(args, "vessel_id")?, str_arg(args, "berth_id")?)?;
            Ok(json!({"ok": v.is_empty(), "violations": v}))
        }
        "check_yard_placement" => {
            let v = engine.check_yard_placement(str_arg(args, "container_id")?, str_arg(args, "block_id")?)?;
            Ok(json!({"ok": v.is_empty(), "violations": v}))
        }
        "validate_all" => Ok(engine.validate_all()),
        "draft_delay_notice" => engine.draft_delay_notice(str_arg(args, "shipper_id")?, str_arg(args, "voyage_id")?, int_arg(args, "delay_hours")?),
        "propose_berth_reassignment" => engine.propose_berth_reassignment(str_arg(args, "voyage_id")?, str_arg(args, "berth_id")?),
        "propose_transshipment_rebooking" => engine.propose_rebooking(str_arg(args, "container_id")?, str_arg(args, "voyage_id")?),
        "apply_plan" => engine.apply_plan(str_arg(args, "plan_id")?),
        "list_plans" => Ok(engine.list_plans()),
        "reset_data" => {
            engine.reset();
            Ok(json!({"reset": true}))
        }
        _ => Err(format!("존재하지 않는 도구 '{name}'")),
    }
}
