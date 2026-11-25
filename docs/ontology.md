# 온톨로지

정의 위치: `src-tauri/src/model.rs`(구조와 규칙 상수), `src-tauri/src/engine.rs`(규칙 적용).
런타임에서는 `describe_schema` 도구가 같은 내용을 반환합니다.

## 클래스

| 클래스 | 한글 | 주요 속성 | ID 예 |
|---|---|---|---|
| Operator | 선사 | name | OP-HBL |
| Vessel | 선박 | name, imo, loa_m(전장), draft_m(흘수), teu | VSL-01 |
| Berth | 선석 | name, length_m, depth_m, cranes | B1 |
| Voyage | 항차(1회 기항) | service, voyage_no, eta, etd, prev_port, next_ports | V01 |
| YardBlock | 장치장 블록 | capacity_teu, reefer_plugs, dg_area | YB-2R |
| Shipper | 화주 | name, tier(`KEY`/`STANDARD`) | SH-02 |
| Container | 컨테이너 | iso_type, reefer, set_temp_c, imdg_class, weight_t, destination | HBLU0100368 |

`Container.kind()`는 저장하지 않고 관계로부터 추론합니다.
입항·출항 항차가 모두 있으면 `TRANSSHIPMENT`, 입항만 있으면 `IMPORT`, 출항만 있으면 `EXPORT`입니다.

## 관계

인스턴스의 `*_id` 필드가 관계입니다. `Engine::get_entity`가 이를 나가는/들어오는 간선으로 펼칩니다.

| 관계 | 정의역 → 치역 | 필드 |
|---|---|---|
| OPERATED_BY | Vessel → Operator | `operator_id` |
| PERFORMED_BY | Voyage → Vessel | `vessel_id` |
| BERTHS_AT | Voyage → Berth | `berth_id` |
| DISCHARGED_FROM | Container → Voyage | `inbound_voyage_id` |
| LOADED_ONTO | Container → Voyage | `outbound_voyage_id` |
| STORED_IN | Container → YardBlock | `block_id` |
| OWNED_BY | Container → Shipper | `shipper_id` |

들어오는 관계가 많은 경우(항차에 실린 컨테이너 수백 개 등)는 개수와 표본으로 요약해서 반환합니다.

## 규칙

상수는 `model.rs`의 `rules` 모듈에 있습니다.

| ID | 이름 | 내용 | 상수 | 검사 위치 |
|---|---|---|---|---|
| R1 | BerthFit | 전장 ≤ 선석 길이, 흘수 + 여유수심 ≤ 수심 | `UKC_M = 1.0` | `check_berth_fit` |
| R2 | BerthExclusive | 같은 선석 기항은 겹치지 않고 간격 유지. 지연 시 뒤 항차가 밀림 | `BERTH_BUFFER_H = 1` | `propagate`, `berth_conflicts` |
| R3 | MinConnectionTime | 환적: 출항 ETD ≥ 입항 ETA + MCT | `MCT_H = 12` | `trace_delay_impact`, `propose_rebooking`, `validate_all` |
| R4 | ReeferPower | 냉동 컨테이너는 남은 리퍼 플러그가 있는 블록에만 | 블록의 `reefer_plugs` | `check_yard_placement`, `validate_all` |
| R5 | DangerousGoods | 위험물은 DG 구역에만. 1·7급 직반출. 비호환 등급 쌍 동일 블록 금지 | `DIRECT_DELIVERY_IMDG`, `INCOMPATIBLE_IMDG` | `check_yard_placement` |
| R6 | Priority | 냉동·위험물·KEY 화주 화물의 환적 실패는 HIGH | — | `priority` |

`schema()`의 각 규칙에는 그 규칙이 참조하는 클래스 목록(`classes`)이 들어 있습니다. 온톨로지 화면은 이 값으로 규칙과 클래스를 연결해 강조합니다. 규칙을 추가하면 `classes`도 채우세요.

R5의 비호환 표는 실제 IMDG 격리표를 **단순화**한 것입니다. 실무 정확도를 주장하지 않습니다.

## 지연 전파 알고리즘 (`Engine::propagate`)

입력: 항차 하나와 지연 시간.

1. 대상 항차의 ETA/ETD를 지연 시간만큼 뒤로 미룹니다.
2. 선석마다 **원래 계획의 ETA 순서**로 항차를 정렬합니다. 지연된 배도 접안 순번을 잃지 않는다는 정책입니다.
3. 앞 항차의 (새) ETD + 버퍼보다 현재 항차 ETA가 이르면 그 차이만큼 현재 항차를 밀고, `caused_by`에 앞 항차를 기록합니다.
4. 선석끼리는 서로 영향을 주지 않으므로 선석당 한 번 훑으면 끝납니다.

그다음 `trace_delay_impact`가 이어서 처리합니다.

5. 밀린 항차와 관련된 환적 컨테이너 중 **이전에는 R3를 만족했지만 이제는 위반**하는 것만 환적 실패로 봅니다(원래 위반이던 것은 이 지연 탓이 아님).
6. 각 실패 건에 R6 우선순위와 대체 항차를 붙입니다. 대체 항차는 목적지에 기항하고 R3를 만족하는 가장 이른 출항편입니다.
7. (입항 항차, 출항 항차) 쌍으로 묶고, `caused_by`를 거슬러 올라가 **근거 경로**를 만듭니다.
   예: `V01 지연 발생 → V01 → B1 (R2 선석 점유) → V05 → B1 (R2 선석 점유) → V09 → V10 환적 연결 실패 (R3 최소 환적 시간)`
8. 밀린 항차마다 R1을 통과하고 원래 시간대에 비어 있는 다른 선석을 **선석 변경 대안**으로 제시합니다.
9. UI용 그래프(이벤트 → 항차 → 선석 → 항차 … → 환적 묶음 → 출항 항차 / 화주)를 만듭니다. `level`은 연쇄 깊이입니다.

`trace_delay_impact`는 상태를 바꾸지 않습니다(테스트 `trace_does_not_change_state`).

## 시드 데이터 (`data.rs`)

모든 이름은 가상입니다. 계획 기간은 2026-12-01 00:00부터 4일입니다.

- 선석 6개: B1·B2(370m/16.5m), B3 초대형선(420m/17.5m), B4(300m/14m), B5 피더(220m/11m), **B6 예비(360m/16m, 대부분 비어 있음)**
- 선박 17척, 항차 17건, 장치장 블록 6개, 화주 8곳(KEY 4곳)
- 컨테이너 768개. 각 입항 항차마다 MCT + 0~30시간 범위의 출항편으로 환적을 배정해서, 지연이 생기면 일부가 현실적인 비율로 끊깁니다.
- 고정 시드 `20261201`이라 실행할 때마다 같은 결과가 나옵니다.

### 기준 시나리오 (테스트가 고정하는 값)

| 시나리오 | 결과 |
|---|---|
| 초기 상태 `validate_all` | 위반 0건 |
| V01 +18시간 | V05 +17h, V09 +14h(모두 B1, R2). 환적 실패 46개, 그중 HIGH 31개. 대안: V05 → B6 |
| 위 대안 적용 후 V01 +18시간 | 연쇄 지연 0건, 환적 실패 12개 |
| VSL-03(400m, 16.5m)을 B1에 | R1 위반 2건(길이, 수심) |

시드를 바꾸면 이 표, `engine.rs`의 테스트, `scripts/mcp-smoke.mjs`를 함께 확인하세요.
