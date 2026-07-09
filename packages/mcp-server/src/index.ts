export const serverName = "@sylphx/architecture-reader-mcp";
export const serverVersion = "0.0.0";

export const plannedTools = [
  "architecture_index",
  "architecture_status",
  "architecture_overview",
  "architecture_search",
  "architecture_trace",
  "architecture_impact",
  "architecture_evidence",
] as const;

export type PlannedTool = (typeof plannedTools)[number];
