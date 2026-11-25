# 아키텍처

## 실행 모드

실행 파일은 하나이고, 진입점이 두 개입니다(`src-tauri/src/main.rs`).

```
port-ontology.exe          → port_ontology_lib::run()   데스크톱 앱 (Tauri + WebView2)
port-ontology.exe --mcp    → port_ontology_lib::mcp::run()   stdio MCP 서버 (창 없음)
```

```
┌──────────────────────── 데스크톱 앱 모드 ────────────────────────┐
│  Next.js 정적 페이지 (WebView)                                    │
│    zustand store ──callTool(name,args)──▶ Tauri command call_tool │
└──────────────────────────────────────────────┬───────────────────┘
                                               ▼
                                  tools::dispatch(engine, name, args)
                                               ▲
┌──────────────────────── MCP 서버 모드 ───────┴───────────────────┐
│  LLM CLI ──stdin JSON-RPC──▶ mcp::handle ──tools/call──▶          │
│          ◀─stdout JSON-RPC─                                       │
└───────────────────────────────────────────────────────────────────┘
                                               │
                                               ▼
                         Engine { Ontology, plans, audit_log }
                         (model.rs 타입 + data.rs 시드 + engine.rs 추론)
```

두 모드는 **별도 프로세스**이며 상태를 공유하지 않습니다. 둘 다 시작할 때 `data::seed()`로 같은 초기 상태를 만듭니다. 영속 저장소는 아직 없습니다([roadmap.md](roadmap.md)).

## 계층과 파일별 책임

### 백엔드 `src-tauri/src/`

| 파일 | 책임 | 고칠 때 주의 |
|---|---|---|
| `main.rs` | `--mcp` 인자로 진입점 분기. 릴리스에서는 `windows_subsystem = "windows"` | 이 속성은 지우지 말 것(콘솔 창이 뜸). GUI 서브시스템이어도 파이프 stdio는 동작함을 확인했음 |
| `model.rs` | 온톨로지 클래스(구조체), 규칙 상수 `rules::*`, `schema()` 설명 | 규칙 수치는 반드시 `rules` 모듈 상수로. `schema()`의 문장도 상수를 참조 |
| `data.rs` | 시드 데이터. 고정 시드 LCG로 결정적 생성 | 값을 바꾸면 테스트 기대값(`engine.rs` tests, `scripts/mcp-smoke.mjs`)과 문서 수치가 바뀜 |
| `engine.rs` | 조회, 관계 탐색, 규칙 검사, 지연 전파, 영향 그래프, 제안/적용, 안내문 | 모든 비즈니스 로직의 유일한 위치. 결정적이어야 함(무작위·시계 사용 금지) |
| `tools.rs` | 도구 정의(JSON Schema)와 `dispatch`. `READ_ONLY` 목록 | 도구 추가 시 `definitions`, `dispatch`, (필요시) `READ_ONLY` 세 곳 |
| `mcp.rs` | JSON-RPC 2.0 over stdio. initialize / ping / tools/list / tools/call | stdout은 프로토콜 전용 |
| `lib.rs` | Tauri 앱 구성. `AppState(Mutex<Engine>)`, command `call_tool`, `list_tools` | command를 기능별로 늘리지 말 것 |

### 프론트엔드 `src/`

Next.js 16 App Router, `output: "export"`(정적 SPA). 서버 기능(SSR, Route Handler, Server Action)은 쓸 수 없습니다.

| 파일 | 책임 |
|---|---|
| `app/page.tsx` | 탭 전환(운영 / 온톨로지 / MCP), 최초 로드 |
| `store/useTerminal.ts` | zustand 스토어. 백엔드 호출, 로딩·오류, 현재 시나리오, 마지막 제안 |
| `lib/api.ts` | `callTool`, `listTools`. Tauri 밖(일반 브라우저)이면 오류 |
| `lib/types.ts` | Rust가 돌려주는 JSON의 TypeScript 타입. Rust 구조체를 바꾸면 여기도 |
| `lib/labels.ts` | 영문 식별자 → 한글 표시명 표 |
| `lib/time.ts` | 백엔드 시각(타임존 없는 로컬 시각) 처리 |
| `components/BerthGantt.tsx` | 선석 간트 차트. 지연·연쇄 지연·원래 일정 표시. 클릭으로 시나리오 대상 선택 |
| `components/ImpactGraph.tsx` | 영향 그래프. 외부 라이브러리 없이 SVG 계층 배치 + 바리센터 정렬 |
| `components/ImpactPanel.tsx` | 영향 그래프 / 근거 경로 / 환적 실패 표 / 대응 방안 탭 |
| `components/ScenarioPanel.tsx` | 왼쪽 패널. 항차·지연 시간 입력, 요약, 화주, 감사 로그 |
| `components/ProposalBar.tsx` | 제안 결과 표시와 [계획 적용] 버튼(2단계 액션의 두 번째 단계) |
| `components/EntityInspector.tsx` | 엔티티 상세와 관계 이동 |
| `components/OntologyView.tsx` | 온톨로지 탭. 왼쪽은 스키마 다이어그램과 규칙 카드(마우스를 올리면 관련 클래스 강조), 오른쪽은 선택한 클래스의 속성·관계·적용 규칙·인스턴스 목록 |
| `components/SchemaDiagram.tsx` | 클래스=노드, 관계=화살표인 SVG 다이어그램. 클래스 좌표는 `POS`에 손으로 지정(클래스를 추가하면 여기에도 좌표를 넣을 것) |
| `components/McpView.tsx` | CLI 연결 안내와 도구 직접 호출 콘솔 |

## 데이터 흐름 예: "영향 분석" 버튼

1. `ScenarioPanel`이 `useTerminal.runImpact()` 호출
2. 스토어가 `callTool("trace_delay_impact", { voyage_id, delay_hours })`
3. Tauri `call_tool` → `tools::dispatch` → `Engine::trace_delay_impact`
4. 엔진이 `ImpactReport`(연쇄 지연, 환적 실패, 근거 경로, 화주별 집계, 선석 대안, 그래프) 반환
5. 스토어의 `impact`가 바뀌면 간트·그래프·패널이 다시 그려짐

LLM이 같은 질문을 받으면 3번부터 똑같이 실행되고, 결과 JSON을 읽어 한국어로 설명합니다.

## 스타일

- Tailwind v4. 색상은 `app/globals.css`의 `@theme` 토큰(`bg`, `panel`, `line`, `ink`, `muted`, `accent`, `root`, `shift`, `ok`, `reefer`, `dg`)만 씁니다. 컴포넌트에 hex 값을 직접 쓰지 않습니다.
- 운영 콘솔 성격이라 다크 테마 하나만 있습니다.
- 본문 폰트는 Pretendard(가변, SIL OFL 1.1)입니다. `npm` 패키지 `pretendard`의 CSS를 `app/layout.tsx`에서 import해 폰트 파일이 번들에 포함되므로 오프라인 설치본에서도 보입니다. CDN 폰트로 바꾸지 마세요.
- 모노 폰트에서는 합자를 꺼 두었습니다(`--mcp`가 대시 한 개처럼 보이는 문제).
