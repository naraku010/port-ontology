# 프로젝트 문서

이 폴더는 사람과 AI 코딩 에이전트가 이 저장소를 처음 열었을 때 빠르게 맥락을 잡도록 정리한 문서입니다.
코드와 문서가 어긋나면 **코드가 기준**이며, 발견한 쪽이 문서를 고칩니다.

## 한 줄 요약

컨테이너 터미널의 선박·선석·컨테이너·화주를 온톨로지로 모델링하고, "이 배가 N시간 늦으면 무슨 일이 생기나"를 규칙 기반으로 추론하는 Tauri 데스크톱 앱입니다. 같은 exe가 `--mcp`로 실행되면 로컬 LLM CLI용 MCP 서버가 됩니다. 포트폴리오 [KODEX](https://ko-dex.com)의 하위 프로젝트이며, 자매 프로젝트 [corp-ontology](https://github.com/naraku010/corp-ontology)와 같은 아키텍처를 씁니다.

## 읽는 순서

| 순서 | 문서 | 내용 |
|---|---|---|
| 1 | [architecture.md](architecture.md) | 전체 구조, 실행 모드 두 가지, 데이터 흐름, 파일별 책임 |
| 2 | [ontology.md](ontology.md) | 클래스·관계·규칙(R1~R6), 지연 전파 알고리즘, 시드 데이터 |
| 3 | [mcp-tools.md](mcp-tools.md) | 도구 14개 명세, MCP 프로토콜 구현, CLI 연결 |
| 4 | [development.md](development.md) | 설치, 실행, 테스트, UI 검증 방법, 함정(gotcha) |
| 5 | [decisions.md](decisions.md) | 주요 설계 결정과 이유. 바꾸기 전에 반드시 읽을 것 |
| 6 | [roadmap.md](roadmap.md) | 알려진 한계와 다음 작업 후보 |

## AI 에이전트를 위한 핵심 규칙

작업 전에 이것만은 지켜 주세요. 자세한 이유는 [decisions.md](decisions.md)에 있습니다.

1. **규칙과 추론은 Rust(`src-tauri/src/engine.rs`)에만 둡니다.** 프론트엔드나 LLM 프롬프트에 비즈니스 규칙을 복제하지 않습니다.
2. **UI와 MCP는 같은 도구 디스패처(`tools.rs`)를 씁니다.** 새 기능은 도구로 추가하고, UI는 `callTool`로 그 도구를 부릅니다. Tauri command를 기능별로 늘리지 않습니다.
3. **상태를 바꾸는 작업은 `propose_*` → `apply_plan` 2단계입니다.** 바로 상태를 바꾸는 도구를 만들지 않습니다.
4. **식별자는 영어, 화면과 메시지는 한국어입니다.** 클래스·관계·도구·규칙 ID(`Voyage`, `BERTHS_AT`, `R2`, `trace_delay_impact`)는 LLM과 공유하는 계약이라 영어로 유지합니다. 사용자에게 보이는 문구는 한국어로 씁니다.
5. **`--mcp` 모드에서는 stdout에 JSON-RPC 외의 것을 쓰지 않습니다.** 로그는 `eprintln!`(stderr)만 씁니다.
6. **Next.js 16은 학습 데이터와 다를 수 있습니다.** 프론트 코드를 쓰기 전에 `node_modules/next/dist/docs/`를 확인하세요(루트 `AGENTS.md` 참고).
7. **변경 후에는** `npm run test:rust`, `npm run test:mcp`, `npm run typecheck`, `npm run lint`를 돌립니다.
