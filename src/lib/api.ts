import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ToolDef } from "./types";

export const inTauri = () => typeof window !== "undefined" && isTauri();

/** Calls the same tool dispatcher the MCP server exposes. */
export async function callTool<T = unknown>(name: string, args: Record<string, unknown> = {}): Promise<T> {
  if (!inTauri()) {
    throw new Error("백엔드에 연결할 수 없습니다. 브라우저가 아니라 `npm run tauri dev`로 실행하세요.");
  }
  return invoke<T>("call_tool", { name, args });
}

export async function listTools(): Promise<ToolDef[]> {
  if (!inTauri()) return [];
  return invoke<ToolDef[]>("list_tools");
}
