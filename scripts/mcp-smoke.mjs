// MCP 서버 스모크 테스트. 실제 exe를 --mcp로 띄워 JSON-RPC로 검사한다.
// 사용법: npm run test:mcp            (디버그 빌드)
//        node scripts/mcp-smoke.mjs <exe 경로>
import { spawn } from "node:child_process";
import readline from "node:readline";
import assert from "node:assert/strict";
import path from "node:path";

const exe = process.argv[2] ?? path.join("src-tauri", "target", "debug", process.platform === "win32" ? "port-ontology.exe" : "port-ontology");
const proc = spawn(exe, ["--mcp"], { stdio: ["pipe", "pipe", "ignore"] });
proc.on("error", (e) => {
  console.error(`실행 실패: ${exe}\n먼저 cd src-tauri && cargo build 하세요.\n${e}`);
  process.exit(1);
});

const pending = new Map();
readline.createInterface({ input: proc.stdout }).on("line", (line) => {
  const msg = JSON.parse(line); // stdout에 JSON 외의 출력이 섞이면 여기서 실패한다.
  pending.get(msg.id)?.(msg);
});
let nextId = 0;
const rpc = (method, params) =>
  new Promise((resolve) => {
    const id = ++nextId;
    pending.set(id, resolve);
    proc.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
  });
const tool = async (name, args = {}) => {
  const r = await rpc("tools/call", { name, arguments: args });
  return { isError: r.result.isError, text: r.result.content[0].text, json: r.result.isError ? null : JSON.parse(r.result.content[0].text) };
};
const step = (msg) => console.log(`  ✓ ${msg}`);

const init = await rpc("initialize", { protocolVersion: "2025-06-18", capabilities: {}, clientInfo: { name: "smoke", version: "0" } });
assert.equal(init.result.protocolVersion, "2025-06-18");
proc.stdin.write(JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }) + "\n");
step("initialize");

const list = await rpc("tools/list", {});
const names = list.result.tools.map((t) => t.name);
for (const n of ["describe_schema", "trace_delay_impact", "propose_berth_reassignment", "apply_plan"]) assert.ok(names.includes(n), n);
assert.equal(list.result.tools.find((t) => t.name === "apply_plan").annotations.readOnlyHint, false);
step(`tools/list (${names.length}개)`);

assert.equal((await tool("validate_all")).json.violations, 0);
step("시드 데이터 규칙 위반 0건");

const before = (await tool("trace_delay_impact", { voyage_id: "V01", delay_hours: 18 })).json;
assert.deepEqual(before.voyage_shifts.map((s) => s.voyage_id), ["V01", "V05", "V09"]);
step(`V01 +18h → 밀린 항차 ${before.voyage_shifts.length - 1}건, 환적 실패 ${before.missed_connections.length}개`);

const opt = before.berth_options[0];
const proposal = (await tool("propose_berth_reassignment", { voyage_id: opt.voyage_id, berth_id: opt.berth_id })).json;
assert.equal(proposal.feasible, true);
const applied = (await tool("apply_plan", { plan_id: proposal.plan.plan_id })).json;
assert.equal(applied.applied, true);
assert.equal((await tool("apply_plan", { plan_id: proposal.plan.plan_id })).isError, true);
const after = (await tool("trace_delay_impact", { voyage_id: "V01", delay_hours: 18 })).json;
assert.ok(after.missed_connections.length < before.missed_connections.length);
step(`${opt.voyage_id} → ${opt.berth_id} 적용 후 환적 실패 ${before.missed_connections.length} → ${after.missed_connections.length}`);

const bad = await tool("get_entity", { id: "NOPE" });
assert.equal(bad.isError, true);
step("잘못된 입력은 isError로 반환");

proc.stdin.end();
console.log("MCP 스모크 테스트 통과");
