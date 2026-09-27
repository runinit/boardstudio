import { expect, it, vi } from 'vitest';
import type { ReactElement } from 'react';
import { ModelPreviewBoundary } from './ModelPreviewBoundary';

it('retries the failed preview instead of leaving the error boundary latched', () => {
  const boundary = new ModelPreviewBoundary({ resetKey: 'switch', children: <span>Preview</span> });
  boundary.state = { failed: true };
  const update = vi.spyOn(boundary, 'setState');
  const fallback = boundary.render() as ReactElement<{ children: ReactElement[] }>;
  const button = fallback.props.children[1] as ReactElement<{ onClick?: () => void }>;
  button.props.onClick?.();
  expect(update).toHaveBeenCalledWith({ failed: false });
});
