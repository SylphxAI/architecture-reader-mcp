import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'bun:test';

const readText = (path: string) => readFileSync(path, 'utf8');

describe('README discovery surfaces', () => {
  it('keeps pain-first fold content and honest discovery status', () => {
    const readme = readText('README.md');

    expect(readme).toContain('Did it trace the right boundary?');
    expect(readme).toContain('## Why not grep or a dashboard?');
    expect(readme).toContain('Beta 0.1');
    expect(readme).toContain('Rust core');
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
    expect(readme).toContain('## Security model');
    expect(readme).toContain('claude mcp add architecture-reader');
    expect(readme).toContain('examples/');
    expect(readme).toContain('MIT');
    expect(readme).not.toContain('pdf-reader-mcp');
    expect(readme).not.toContain('ADR-0002');
  });

  it('ships MCP Registry metadata and publish workflow scaffolding', () => {
    const server = JSON.parse(readText('server.json'));
    const pkg = JSON.parse(readText('packages/mcp-server/package.json'));

    expect(server.name).toBe('io.github.SylphxAI/architecture-reader-mcp');
    expect(server.packages[0]?.identifier).toBe('@sylphx/architecture-reader-mcp');
    expect(pkg.publishConfig?.access).toBe('public');
    expect(existsSync('fixtures/sample-repo/package.json')).toBe(true);
    expect(existsSync('crates/architecture-reader-cli/Cargo.toml')).toBe(true);
    expect(existsSync('docs/specs/2026-07-09-tool-contract.md')).toBe(true);
    expect(existsSync('docs/portfolio/roadmaps/architecture-reader-mcp.md')).toBe(true);
    expect(existsSync('docs/portfolio/roadmaps/gpt-review.md')).toBe(true);
    expect(existsSync('.github/workflows/ci.yml')).toBe(true);
    expect(existsSync('.github/workflows/release.yml')).toBe(true);
    expect(existsSync('.github/workflows/publish-mcp-registry.yml')).toBe(true);
    expect(existsSync('.changeset/config.json')).toBe(true);
  });

  it('marks sunset MCP repos as archived in the portfolio plan', () => {
    const portfolio = readText('docs/portfolio/README.md');

    expect(portfolio).toContain('## Archived MCP Projects');
    expect(portfolio).toContain('consultant-mcp');
    expect(portfolio).toContain('gpt-review');
    expect(portfolio).toContain('2026-07-09');
    expect(portfolio).not.toContain('consultant-mcp` | Give agents safe local operations and structured decision review');
  });
});