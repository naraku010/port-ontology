# 개발 가이드

## 준비물

- Node 20 이상(작성 시점 22), npm
- Rust stable(작성 시점 1.97, edition 2024)
- Windows: Visual Studio Build Tools(MSVC C++), WebView2 런타임(Windows 11 기본 포함)

## 명령어

| 명령 | 용도 |
|---|---|
| `npm install` | 프론트 의존성 설치 |
| `npm run tauri dev` | 개발 실행(Next dev 서버 3000번 + 데스크톱 창) |
| `npm run tauri build` | 릴리스 빌드와 NSIS 설치 파일 → `src-tauri/target/release/bundle/nsis/` |
| `npm run build` | 프론트 정적 export만(`out/`) |
| `npm run test:rust` | Rust 단위 테스트(`engine.rs` 하단) |
| `npm run test:mcp` | MCP 스모크 테스트(디버그 exe 필요) |
| `npm run typecheck` | `next typegen`(라우트 타입 생성) 후 `tsc --noEmit` |
| `npm run lint` | ESLint(`src-tauri/**`는 제외) |
| `cd src-tauri && cargo build` | 백엔드만 빌드(MCP 테스트 전에 필요) |

## 변경 후 확인 순서

1. `npm run test:rust`
2. `cd src-tauri && cargo build` 후 `npm run test:mcp`
3. `npm run typecheck` 그리고 `npm run lint`
4. UI를 바꿨다면 아래 방법으로 화면 확인

## UI 확인 방법 (AI 에이전트용)

데스크톱 창은 일반 브라우저로 열 수 없습니다. 브라우저에서 `localhost:3000`을 열면 Tauri 백엔드가 없어서 오류 배너만 보입니다.
WebView2의 원격 디버깅 포트를 열어 CDP로 조작·캡처합니다.

```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9333"
npm run tauri dev
```

그다음 `http://127.0.0.1:9333/json`에서 page 타깃의 `webSocketDebuggerUrl`에 연결해 `Runtime.evaluate`로 버튼을 누르고 `Page.captureScreenshot`으로 캡처합니다. Node 22에는 전역 `WebSocket`이 있어 추가 패키지 없이 가능합니다.
버튼은 텍스트로 찾을 수 있습니다(예: `영향 분석`, `대응 방안`, `변경 제안`, `계획 적용`).

## 함정 (Gotchas)

- **Next.js 16**: create-next-app 16이 만든 프로젝트입니다. API와 설정이 학습 데이터와 다를 수 있으니 `node_modules/next/dist/docs/`를 먼저 확인하세요. `AGENTS.md`의 Next 블록은 `next dev`가 자동으로 다시 씁니다.
- **정적 export**: `next.config.ts`에 `output: "export"`가 있습니다. 서버 컴포넌트에서 요청 시점 데이터, Route Handler, Server Action, `next/image` 최적화는 쓸 수 없습니다. 모든 페이지는 클라이언트에서 `callTool`로 데이터를 받습니다.
- **Tailwind v4**: 설정 파일이 없고, 토큰은 `globals.css`의 `@theme`에 있습니다. `@tailwindcss/turbopack` 로더를 `next.config.ts`에서 씁니다.
- **개발 서버의 오래된 CSS**: 새 컴포넌트에서 처음 쓰는 Tailwind 클래스가 개발 모드에서 적용되지 않을 때가 있습니다(레이아웃이 한 줄로 무너짐). Turbopack 캐시 문제입니다. 개발 앱을 끄고 `.next` 폴더를 지운 뒤 다시 실행하세요. `npm run build` 결과에는 문제가 없습니다.
- **stdout 오염**: `--mcp` 모드에서 `println!`을 쓰면 클라이언트 JSON 파싱이 깨집니다. `eprintln!`만 쓰세요.
- **릴리스 exe와 stdio**: 릴리스 빌드는 GUI 서브시스템이지만, 부모 프로세스가 파이프를 넘기면 stdio가 정상 동작합니다(`npm run test:mcp -- <릴리스 exe>`로 확인 가능).
- **시각 처리**: 백엔드 시각은 타임존 없는 `NaiveDateTime` 문자열입니다. 프론트에서는 서로 비교·차이 계산에만 씁니다(`lib/time.ts`).
- **PowerShell에서 node 인자 전달**: PowerShell 5.1은 외부 프로그램 인자 속 큰따옴표를 지웁니다. JSON을 인자로 넘기지 말고 스크립트 안에서 만드세요.
- **링커 경고**: 한국어 Windows에서 `linker stdout: ... 라이브러리 및 ... 개체를 생성하고 있습니다` 경고가 나옵니다. 정상입니다.

## 코드 규칙

- 사용자에게 보이는 문자열(오류, 위반 사유, 요약, 근거 경로, UI 문구)은 한국어.
- 식별자(클래스, 관계, 도구 이름, 규칙 ID, JSON 키, enum 값 `HIGH`/`KEY` 등)는 영어. 화면 표시는 `src/lib/labels.ts`로 변환.
- `validate_all`은 위반 메시지 일부 문자열(`플러그 부족`, `격리 필요`)로 블록 단위 위반을 거릅니다. 해당 메시지를 바꾸면 이 필터도 고쳐야 합니다.
- 색상은 테마 토큰만 사용.
- 커밋 전 위 확인 순서를 통과시킬 것.
