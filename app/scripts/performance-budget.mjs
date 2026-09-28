// Timestamp subtraction can put an equal decimal timing a fraction of a
// nanosecond over its limit. Normalize only that noise; budgets stay unchanged.
export function withinPerformanceBudget(measuredMs, limitMs) {
  return Number.isFinite(measuredMs) && Number.isFinite(limitMs)
    && Math.round(measuredMs * 1e6) <= Math.round(limitMs * 1e6);
}
