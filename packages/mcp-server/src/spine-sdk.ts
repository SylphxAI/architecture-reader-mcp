/**
 * Spine SDK — thin isomorphic client over the Rust architecture engine.
 * Same tools as MCP/CLI (`architecture_*`).
 */
import { runTool } from './tools.js';

export type SpineRootOptions = {
  root: string;
};

export class Spine {
  readonly root: string;

  constructor(options: SpineRootOptions) {
    this.root = options.root;
  }

  static create(options: SpineRootOptions): Spine {
    return new Spine(options);
  }

  private call(tool: string, input: Record<string, unknown> = {}) {
    return runTool(tool, { root: this.root, ...input });
  }

  index(input: { mode?: 'auto' | 'full' | 'status_only' } = {}) {
    return this.call('architecture_index', input);
  }

  status() {
    return this.call('architecture_status');
  }

  overview(input: Record<string, unknown> = {}) {
    return this.call('architecture_overview', input);
  }

  search(query: string, input: Record<string, unknown> = {}) {
    return this.call('architecture_search', { query, ...input });
  }

  path(from: string, to: string, input: Record<string, unknown> = {}) {
    return this.call('architecture_path', { from, to, ...input });
  }

  trace(from: string, to: string, input: Record<string, unknown> = {}) {
    return this.call('architecture_trace', { from, to, ...input });
  }

  impact(changedPaths: string[], input: Record<string, unknown> = {}) {
    return this.call('architecture_impact', { changedPaths, ...input });
  }

  evidence(ids: string[], input: Record<string, unknown> = {}) {
    return this.call('architecture_evidence', { ids, ...input });
  }
}

export default Spine;
