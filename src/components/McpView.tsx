"use client";

import { useState } from "react";
import { useTerminal } from "@/store/useTerminal";
import { callTool } from "@/lib/api";

/** LLM CLI 연결 방법과, 같은 도구를 직접 호출해 보는 콘솔. */
export function McpView() {
  const tools = useTerminal((s) => s.tools);
  const [name, setName] = useState("trace_delay_impact");
  const [args, setArgs] = useState('{\n  "voyage_id": "V01",\n  "delay_hours": 18\n}');
  const [out, setOut] = useState("");

  const run = async () => {
    try {
      const parsed = args.trim() ? JSON.parse(args) : {};
      setOut(JSON.stringify(await callTool(name, parsed), null, 2));
    } catch (e) {
      setOut(`오류: ${e}`);
    }
  };

  const pick = (n: string) => {
    setName(n);
    const t = tools.find((x) => x.name === n);
    const sample = Object.fromEntries(Object.keys(t?.inputSchema.properties ?? {}).map((k) => [k, ""]));
    setArgs(JSON.stringify(sample, null, 2));
  };

  const config = `{
  "mcpServers": {
    "port-ontology": {
      "command": "<설치 경로>\\\\port-ontology.exe",
      "args": ["--mcp"]
    }
  }
}`;

  return (
    <div className="grid h-full grid-cols-[420px_1fr] overflow-hidden">
      <div className="space-y-5 overflow-y-auto border-r border-line bg-panel p-5">
        <section>
          <h2 className="mb-2 text-sm font-semibold">LLM CLI 연결</h2>
          <p className="mb-2 text-xs leading-relaxed text-muted">
            이 실행 파일은 MCP 서버를 겸합니다. CLI의 <span className="font-mono">.mcp.json</span>에 등록하면 CLI가 <span className="font-mono">--mcp</span> 옵션으로 실행해 표준 입출력으로
            통신합니다. LLM은 어떤 도구를 부를지만 정하고, 규칙 검사와 그래프 탐색은 모두 이 앱이 수행합니다.
          </p>
          <pre className="overflow-x-auto rounded bg-bg p-3 font-mono text-[11px]">{config}</pre>
        </section>
        <section>
          <h2 className="mb-2 text-sm font-semibold">도구 ({tools.length}개)</h2>
          <ul className="space-y-1">
            {tools.map((t) => (
              <li key={t.name}>
                <button onClick={() => pick(t.name)} className={`w-full rounded px-2 py-1.5 text-left hover:bg-panel-2 ${name === t.name ? "bg-panel-2" : ""}`}>
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-xs font-semibold">{t.name}</span>
                    <span className={`rounded px-1 text-[10px] ${t.annotations.readOnlyHint ? "bg-ok/15 text-ok" : "bg-shift/15 text-shift"}`}>
                      {t.annotations.readOnlyHint ? "조회 전용" : "상태 변경"}
                    </span>
                  </div>
                  <div className="text-[11px] text-muted">{t.description}</div>
                </button>
              </li>
            ))}
          </ul>
        </section>
      </div>
      <div className="flex min-h-0 flex-col gap-3 p-5">
        <div className="flex items-center gap-3">
          <span className="font-mono text-sm font-semibold">{name}</span>
          <button onClick={run} className="rounded bg-accent px-3 py-1.5 text-sm font-semibold text-bg">
            도구 호출
          </button>
          <span className="text-xs text-muted">MCP 서버와 같은 디스패처를 호출합니다. 인자는 JSON으로 입력하세요.</span>
        </div>
        <textarea value={args} onChange={(e) => setArgs(e.target.value)} spellCheck={false} className="h-32 shrink-0 rounded border border-line bg-panel p-3 font-mono text-xs" />
        <pre className="min-h-0 flex-1 overflow-auto rounded border border-line bg-panel p-3 font-mono text-[11px]">{out || "결과가 여기에 표시됩니다."}</pre>
      </div>
    </div>
  );
}
