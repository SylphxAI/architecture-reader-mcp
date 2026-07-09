#!/usr/bin/env node

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  architectureEvidenceSchema,
  architectureImpactSchema,
  architectureIndexSchema,
  architectureOverviewSchema,
  architectureSearchSchema,
  architectureStatusSchema,
  architectureTraceSchema,
  runTool,
} from './tools.js';

const server = new McpServer({
  name: '@sylphx/architecture-reader-mcp',
  version: '0.1.0',
});

function registerTool<T extends Record<string, unknown>>(
  name: string,
  description: string,
  schema: { shape: Record<string, unknown> },
  tool: string,
) {
  server.registerTool(
    name,
    {
      description,
      inputSchema: schema.shape as Record<string, unknown>,
    },
    async (input) => {
      const envelope = runTool(tool, input as T);
      return {
        content: [{ type: 'text', text: JSON.stringify(envelope, null, 2) }],
        isError: envelope.status === 'error',
      };
    },
  );
}

registerTool(
  'architecture_index',
  'Create or refresh the local architecture evidence index for a repository.',
  architectureIndexSchema,
  'architecture_index',
);
registerTool(
  'architecture_status',
  'Report architecture index freshness, coverage, extractor versions, and known gaps.',
  architectureStatusSchema,
  'architecture_status',
);
registerTool(
  'architecture_overview',
  'Return a compact architecture map for a repository or subpath.',
  architectureOverviewSchema,
  'architecture_overview',
);
registerTool(
  'architecture_search',
  'Search architecture nodes, edges, and claims with evidence.',
  architectureSearchSchema,
  'architecture_search',
);
registerTool(
  'architecture_trace',
  'Trace dependency, import, or documentation paths between architecture entities.',
  architectureTraceSchema,
  'architecture_trace',
);
registerTool(
  'architecture_impact',
  'Estimate architecture impact radius for changed files.',
  architectureImpactSchema,
  'architecture_impact',
);
registerTool(
  'architecture_evidence',
  'Fetch exact evidence behind nodes, edges, or claims.',
  architectureEvidenceSchema,
  'architecture_evidence',
);

async function main(): Promise<void> {
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((error: unknown) => {
  console.error('[Architecture Reader MCP] Server error:', error);
  process.exit(1);
});