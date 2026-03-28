import { invoke } from "@tauri-apps/api/core";

export interface PortCheckResult {
  occupied: boolean;
  pid: string | null;
  process: string | null;
}

export async function checkLocalPort(port: string): Promise<PortCheckResult> {
  return invoke<PortCheckResult>("check_local_port", { port });
}
