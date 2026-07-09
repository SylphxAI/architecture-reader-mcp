import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'bun:test';

const readText = (path: string) => readFileSync(path, 'utf8');

describe('README discovery surfaces', () => {
  it('keeps pain-first fold content and honest discovery status', () => {
    const readme = readText('README.md');

    expect(readme).toContain('Did it trace the right boundary?');
    expect(readme).toContain('## Why not grep or a dashboard?');
    expect(readme).toContain('Draft scaffold');
    expect(readme).toContain('not shipped yet');
    expect(readme).toMatch(/Star the repo|Star this repo/);
    expect(readme).toContain('Not listed yet');
    expect(readme).toContain('registry.modelcontextprotocol.io');
    expect(readme).toContain('glama.ai/mcp/servers');
    expect(readme).toContain('mcpservers.org/submit');
    expect(readme).toContain('mcp.so/submit');
    expect(readme).not.toContain('Publishing on next release');
    expect(readme).toContain('coderag');
    expect(readme).toContain('architecture_search');
    expect(readme).toContain('architecture_trace');
    expect(readme).toContain('architecture_impact');
    expect(readme).not.toContain('pdf-reader-mcp');
    expect(readme).not.toContain('ADR-0002');
  });

  it('ships draft MCP metadata and design SSOT links', () => {
    const server = JSON.parse(readText('server.json'));

    expect(server.status).toBe('draft');
    expect(server.tools).toContain('architecture_index');
    expect(server.tools).toContain('architecture_evidence');
    expect(existsSync('docs/specs/2026-07-09-tool-contract.md')).toBe(true);
    expect(existsSync('docs/portfolio/roadmaps/architecture-reader-mcp.md')).toBe(true);
    expect(existsSync('.github/workflows/ci.yml')).toBe(true);
  });
});