import { useEffect, useRef, useState } from 'react';
import type { Vec2 } from '@boardstudio/v2-contracts';
import type { Mode } from './workbenchTypes';

type Inputs = {
  documentId: string;
  session?: number;
  boardId: () => string;
  beforeNavigate: (mode: Mode, changed: boolean) => void;
  onProjectChange: () => void;
};

/** Owns workflow return state and cameras; the shell coordinates shared interactions. */
export function useWorkbenchNavigation({ documentId, session, boardId, beforeNavigate, onProjectChange }: Inputs) {
  const [mode, setMode] = useState<Mode>('Design');
  const [lastDesignMode, setLastDesignMode] = useState<Mode>('Design');
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState<Vec2>({ x: 0, y: 0 });
  const cameras = useRef(new Map<string, { zoom: number; pan: Vec2 }>());
  const scope = useRef({ documentId, session });

  useEffect(() => {
    if (scope.current.documentId === documentId && scope.current.session === session) return;
    scope.current = { documentId, session };
    cameras.current.clear();
    setMode('Design');
    setLastDesignMode('Design');
    setZoom(1);
    setPan({ x: 0, y: 0 });
    onProjectChange();
  }, [documentId, session]);

  const navigate = (next: Mode) => {
    beforeNavigate(next, next !== mode);
    if (next !== mode) {
      cameras.current.set(`${boardId()}:${mode}`, { zoom, pan });
      const camera = cameras.current.get(`${boardId()}:${next}`);
      setZoom(camera?.zoom ?? 1);
      setPan(camera?.pan ?? { x: 0, y: 0 });
    }
    if (next === 'Design' || next === 'PCB' || next === 'Keymap' || next === 'Case') setLastDesignMode(next);
    setMode(next);
  };

  return { mode, lastDesignMode, navigate, setMode, zoom, pan, setZoom, setPan };
}
