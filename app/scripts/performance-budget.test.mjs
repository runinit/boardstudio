import { test } from 'node:test';
import assert from 'node:assert/strict';
import { withinPerformanceBudget } from './performance-budget.mjs';

test('does not reject equal millisecond timings because of timestamp subtraction noise', () => {
  assert.equal(withinPerformanceBudget(4.400000035762787, 4.4), true);
  assert.equal(withinPerformanceBudget(34.10000002384186, 34.1), true);
});

test('retains real over-budget and invalid measurement failures', () => {
  for (const measured of [4.40001, 4.5, Infinity, NaN]) {
    assert.equal(withinPerformanceBudget(measured, 4.4), false);
  }
  assert.equal(withinPerformanceBudget(4.3, 4.4), true);
  assert.equal(withinPerformanceBudget(4.4, NaN), false);
});
