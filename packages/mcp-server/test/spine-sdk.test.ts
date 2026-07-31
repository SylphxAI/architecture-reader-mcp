import { describe, expect, test } from 'bun:test';
import { Spine } from '../src/spine-sdk.ts';

describe('Spine SDK', () => {
  test('create binds root and exposes path method', () => {
    const spine = Spine.create({ root: process.cwd() });
    expect(spine.root).toBe(process.cwd());
    expect(typeof spine.path).toBe('function');
    expect(typeof spine.index).toBe('function');
    expect(typeof spine.search).toBe('function');
  });
});
