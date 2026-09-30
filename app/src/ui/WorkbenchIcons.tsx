import type {
  PartDefinition
} from '../../../contracts/src/index';
import { Mode, SelectionScope } from './workbenchTypes';

export const PartGlyph = ({ kind }: { kind: PartDefinition['kind'] }) => <svg viewBox="0 0 28 28" aria-hidden="true" className="wb-glyph-svg">
  {kind === 'switch' ? <><rect x="4" y="4" width="20" height="20" rx="4" /><circle cx="14" cy="14" r="4.3" /><path d="M14 2v4M14 22v4M2 14h4M22 14h4" /></>
    : kind === 'controller' ? <><rect x="6" y="3" width="16" height="22" rx="2" /><path d="M10 7h8M10 21h8M3 9v2M3 14v2M25 9v2M25 14v2" /><circle cx="14" cy="14" r="3" /></>
      : kind === 'connector' ? <><rect x="4" y="7" width="20" height="14" rx="2" /><path d="M8 10v8M12 10v8M16 10v8M20 10v8" /></>
        : kind === 'encoder' ? <><circle cx="14" cy="14" r="9" /><circle cx="14" cy="14" r="3" /><path d="M14 2v3M14 23v3M2 14h3M23 14h3" /></>
          : <><rect x="5" y="5" width="18" height="18" rx="2" /><path d="M10 10h8v8h-8zM2 10h3M2 18h3M23 10h3M23 18h3" /></>}
</svg>;

export const ScopeIcon = ({ kind }: { kind: SelectionScope['kind'] }) => <svg viewBox="0 0 20 20" aria-hidden="true">
  {kind === 'matrix' ? <><rect x="3" y="3" width="14" height="14" rx="1" /><path d="M3 8h14M3 12h14M8 3v14M12 3v14" /></>
    : kind === 'row' ? <><rect x="2" y="7" width="16" height="6" rx="1" /><path d="M7 7v6M13 7v6" /></>
      : kind === 'column' ? <><rect x="7" y="2" width="6" height="16" rx="1" /><path d="M7 7h6M7 13h6" /></>
        : kind === 'key' ? <><rect x="3" y="3" width="14" height="14" rx="3" /><path d="M6 13h8" /></>
          : <><rect x="6" y="6" width="8" height="8" rx="1" /><path d="M7 2v4M13 2v4M7 14v4M13 14v4M2 7h4M2 13h4M14 7h4M14 13h4" /></>}
</svg>;

export const ModeIcon = ({ mode }: { mode: Mode }) => <svg viewBox="0 0 20 20" aria-hidden="true" className="wb-mode-icon">
  {mode === 'Design' ? <><path d="M3 14.5 14.5 3l2.5 2.5L5.5 17H3z" /><path d="m11 6 3 3M3 17h14" /></>
    : mode === 'Keymap' ? <><rect x="3" y="4" width="14" height="12" rx="2" /><path d="M6 7h2M10 7h2M14 7h1M6 10h2M10 10h2M6 13h8" /></>
    : mode === 'PCB' ? <><rect x="3" y="3" width="14" height="14" rx="2" /><circle cx="7" cy="7" r="1.2" /><circle cx="13" cy="13" r="1.2" /><path d="M8 7h3v3M7 8v3h3" /></>
      : mode === 'Case' ? <><path d="m10 2 7 4v8l-7 4-7-4V6z" /><path d="m3 6 7 4 7-4M10 10v8" /></>
        : mode === 'Library' ? <><path d="M4 3h9l3 3v11H4z" /><path d="M13 3v4h4M7 11h6M7 14h6" /></>
          : <><path d="M10 2v10" /><path d="M6.5 5.5a7 7 0 1 0 7 0" /></>}
</svg>;

export const BrandMark = () => <svg viewBox="0 0 30 30" aria-hidden="true"><path d="M4 4h22v22H4z" /><path d="m8 20 5-10 4 8 3-5 3 7" /><circle cx="13" cy="10" r="1.3" /></svg>;


export const ArrowIcon = () => <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8h9M8 4l4 4-4 4" /></svg>;

export const UndoIcon = () => <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M8 6 4 10l4 4M4 10h7a5 5 0 0 1 5 5" /></svg>;

export const RedoIcon = () => <svg viewBox="0 0 20 20" aria-hidden="true"><path d="m12 6 4 4-4 4m4-4H9a5 5 0 0 0-5 5" /></svg>;

export const FitIcon = () => <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M7 3H3v4M13 3h4v4M3 13v4h4M17 13v4h-4" /><path d="m3 7 5-4M17 7l-5-4M3 13l5 4m9-4-5 4" /></svg>;

export const CursorIcon = () => <svg viewBox="0 0 36 36" aria-hidden="true"><path d="m9 5 17 15-8 1 5 8-3 2-5-8-5 6z" /></svg>;
