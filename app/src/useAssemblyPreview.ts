import { useCallback, useEffect, useRef, useState } from 'react';
import { AssemblyPreview, type AssemblyPreviewInput, type AssemblyPreviewSnapshot } from './assemblyPreview';

/** React adapter; the preview module owns all worker and acceptance rules. */
export function useAssemblyPreview(input: AssemblyPreviewInput) {
  const latest = useRef(input);
  latest.current = input;
  const owner = useRef<AssemblyPreview | undefined>(undefined);
  const [snapshot, setSnapshot] = useState<AssemblyPreviewSnapshot>({
    models: [], messages: [], pending: true, error: '', keycaps: [], keycapsPending: false, keycapError: '',
  });
  useEffect(() => {
    const preview = new AssemblyPreview();
    owner.current = preview;
    const unsubscribe = preview.subscribe(setSnapshot);
    preview.update(latest.current);
    return () => { unsubscribe(); preview.close(); owner.current = undefined; };
  }, []);
  useEffect(() => { owner.current?.update(input); }, [input.document, input.boardId, input.contours, input.preparedCase, input.session, input.instanceId]);
  const retry = useCallback(() => owner.current?.retry(), []);
  return { ...snapshot, retry };
}
