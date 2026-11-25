# MCP 서버와 도구

## 프로토콜 구현 (`src-tauri/src/mcp.rs`)

- 전송: stdio, 줄 단위 JSON-RPC 2.0(메시지 하나 = 한 줄)
- 지원 메서드: `initialize`, `ping`, `tools/list`, `tools/call`
- `id`가 없는 메시지(알림, 예: `notifications/initialized`)에는 응답하지 않습니다.
- 프로토콜 버전: 클라이언트가 요청한 버전이 `SUPPORTED_VERSIONS`에 있으면 그대로, 없으면 목록의 첫 번째 버전으로 응답합니다.
- 도구 실행 오류는 JSON-RPC 오류가 아니라 `result.isError = true`와 한국어 메시지로 돌려줍니다. LLM이 읽고 스스로 고칠 수 있게 하려는 것입니다.
- 도구 결과는 `content[0].text`에 보기 좋게 정렬한 JSON 문자열로 담습니다.
- `initialize` 응답의 `instructions`에 사용 지침(먼저 스키마 확인, 근거 경로 인용, apply 전 사용자 확인, 한국어 응답)이 들어 있습니다.

외부 MCP SDK를 쓰지 않고 직접 구현했습니다. 필요한 메서드가 네 개뿐이고, exe 하나에 의존성을 적게 두려는 목적입니다.

## 도구 목록 (`src-tauri/src/tools.rs`)

`readOnlyHint`가 true인 도구는 상태를 바꾸지 않습니다. CLI에서 이 도구들만 자동 허용하면 안전합니다.

| 도구 | 조회 전용 | 인자 | 설명 |
|---|---|---|---|
| describe_schema | ✓ | — | 클래스(현재 인스턴스 수 `instance_count` 포함)·관계·규칙(관련 클래스 `classes` 포함) |
| search_entities | ✓ | query, type?, limit? | ID나 이름 일부로 검색 |
| get_entity | ✓ | id | 속성과 관계 |
| get_berth_schedule | ✓ | berth_id? | 선석별 기항 계획 |
| trace_delay_impact | ✓ | voyage_id, delay_hours(1~240) | 지연 영향 분석. 근거 경로 포함 |
| check_berth_fit | ✓ | vessel_id, berth_id | R1 검사 |
| check_yard_placement | ✓ | container_id, block_id | R4, R5 검사 |
| validate_all | ✓ | — | 전체 상태 감사 |
| draft_delay_notice | ✓ | shipper_id, voyage_id, delay_hours | 화주 안내문 초안(발송 안 함) |
| list_plans | ✓ | — | 계획 목록과 감사 로그 |
| propose_berth_reassignment | ✗ | voyage_id, berth_id | R1, R2 검증 후 plan_id 발급 |
| propose_transshipment_rebooking | ✗ | container_id, voyage_id | 항로, R3 검증 후 plan_id 발급 |
| apply_plan | ✗ | plan_id | 계획 반영. 반영 직전에 규칙을 다시 검사하고, 같은 계획은 두 번 적용되지 않음 |
| reset_data | ✗ | — | 시드 상태로 초기화 |

`propose_*`는 엔진 상태를 바꾸지 않지만 계획을 등록하므로 조회 전용으로 분류하지 않았습니다.

### 반환 형식 예

`propose_berth_reassignment` 성공:

```json
{ "feasible": true,
  "plan": { "plan_id": "PLAN-001", "description": "...", "applied": false,
            "action": { "type": "ReassignBerth", "voyage_id": "V05", "from_berth": "B1", "to_berth": "B6" } },
  "next_step": "사용자가 확인한 뒤에만 이 plan_id로 apply_plan을 호출하세요." }
```

실패: `{ "feasible": false, "violations": [{ "rule": "R1", "message": "..." }] }`

`trace_delay_impact`의 필드는 `src/lib/types.ts`의 `ImpactReport`를 보세요.

## 도구 추가 절차

1. `engine.rs`에 로직을 메서드로 추가합니다(결정적, 한국어 메시지).
2. `tools.rs`의 `definitions()`에 이름·한국어 설명·입력 스키마를 추가합니다.
3. `dispatch()`에 분기를 추가합니다.
4. 상태를 바꾸지 않는다면 `READ_ONLY`에 넣습니다. 상태를 바꾼다면 `propose_*` + `Action` 변형으로 설계합니다.
5. 엔진 테스트를 추가하고, 필요하면 `scripts/mcp-smoke.mjs`에도 검사를 넣습니다.
6. UI에서 쓰려면 `useTerminal` 스토어에 액션을 추가하고 `callTool`로 호출합니다.
7. 이 문서의 표를 갱신합니다.

## LLM CLI 연결

저장소 루트의 `.mcp.json`이 디버그 빌드를 등록합니다.

```json
{ "mcpServers": { "port-ontology": { "command": "./src-tauri/target/debug/port-ontology.exe", "args": ["--mcp"] } } }
```

- 먼저 `cd src-tauri && cargo build`로 exe를 만들어야 합니다.
- Claude Code는 프로젝트 범위 MCP 서버를 처음 쓸 때 사용자 승인을 요구합니다. 승인은 사용자가 직접 합니다. `claude mcp list`로 상태를 확인할 수 있습니다.
- 설치본을 쓰려면 `command`를 설치 경로의 exe로 바꿉니다.

## 수동 테스트

```powershell
npm run test:mcp
```

`scripts/mcp-smoke.mjs`가 exe를 띄워 initialize → tools/list → 시나리오 → 제안 → 적용 → 재적용 거부 → 오류 처리까지 확인합니다.
