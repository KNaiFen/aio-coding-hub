import { commands, type ActiveRequestSnapshotItem } from "../../generated/bindings";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";

export type ActiveRequest = ActiveRequestSnapshotItem;

function normalizedRequestPath(path: string) {
  const withoutQuery = path.trim().split("?", 1)[0] ?? "";
  return withoutQuery.replace(/\/+$/, "").toLowerCase() || "/";
}

export function isActiveInferenceRequest(
  request: Pick<ActiveRequest, "cli_key" | "method" | "path">
) {
  if (request.method.trim().toUpperCase() !== "POST") return false;

  const cliKey = request.cli_key.trim().toLowerCase();
  const path = normalizedRequestPath(request.path);

  if (cliKey === "claude") {
    return path === "/v1/messages" || path === "/messages";
  }

  if (cliKey === "codex") {
    return [
      "/responses",
      "/v1/responses",
      "/v1/codex/responses",
      "/responses/compact",
      "/v1/responses/compact",
      "/v1/codex/responses/compact",
    ].includes(path);
  }

  if (cliKey === "grok") {
    return ["/chat/completions", "/v1/chat/completions", "/responses", "/v1/responses"].includes(
      path
    );
  }

  if (cliKey === "gemini") {
    return path.endsWith(":generatecontent") || path.endsWith(":streamgeneratecontent");
  }

  return false;
}

export function countActiveInferenceRequests(requests: ActiveRequest[]) {
  return requests.filter(isActiveInferenceRequest).length;
}

export async function activeRequestLogsSnapshot() {
  return invokeGeneratedIpc<ActiveRequest[]>({
    title: "读取进行中请求失败",
    cmd: "active_request_logs_snapshot",
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.activeRequestLogsSnapshot(), (rows) => rows),
  });
}
