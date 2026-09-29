import type { HardwareConfiguration, PhysicalBoardInstance, ProjectDoc } from '@boardstudio/v2-contracts';
import { reversibleLayout } from '../projectConstruction';
import { SetupChoice, SetupIcon } from './SetupChoice';
import { createMechanicalConfiguration } from '../mechanicalPresets';

export function HardwareInstancesPanel({ document, boardId, selectedId, onSelect, onChange, onReversibleChange, context = 'setup' }: {
  context?: 'setup' | 'case';
  document: ProjectDoc;
  boardId: string;
  selectedId?: string;
  onSelect: (id: string, boardId: string) => void;
  onChange: (hardware: HardwareConfiguration) => void;
  onReversibleChange: (reversible: boolean) => void;
}) {
  const hardware = document.hardware;
  const instances = hardware?.instances ?? [];
  const selected = instances.find(instance => instance.id === selectedId);
  const patch = (changes: Partial<PhysicalBoardInstance>) => { if (hardware && selected) onChange({ ...hardware, instances: instances.map(instance => instance.id === selected.id ? { ...instance, ...changes } : instance) }); };
  const reversible = reversibleLayout(document);
  const configure = (split: boolean) => {
    const retained = selected ?? instances.find(instance => instance.boardId === boardId);
    const mechanical = retained?.mechanical ?? (document.mechanical?.boardId === boardId ? document.mechanical : createMechanicalConfiguration(document, boardId));
    const make = (half: string, role: string, flipped: boolean): PhysicalBoardInstance => ({
      id: crypto.randomUUID(), name: half === 'unibody' ? 'Keyboard' : `${half === 'left' ? 'Left' : 'Right'} half`,
      boardId, half, role, flipped, constructionLinked: true, controllerPartId: null,
      mechanical: { ...mechanical, openings: flipped ? [] : mechanical.openings, mounts: flipped ? [] : mechanical.mounts },
    });
    // Keep the chosen assembly's identity, controller, orientation, and case edits.
    const primary = retained ? { ...retained, half: split ? 'left' : 'unibody', role: split ? 'central' : 'standalone',
      name: ['Keyboard', 'Left half', 'Right half'].includes(retained.name) ? (split ? 'Left half' : 'Keyboard') : retained.name,
    } : make(split ? 'left' : 'unibody', split ? 'central' : 'standalone', false);
    const next = split ? [primary, make('right', 'peripheral', reversible)] : [primary];
    onChange({ ...hardware, topology: split ? 'split' : 'unibody', transport: split ? 'wireless' : 'none', boards: hardware?.boards ?? [], instances: next, sharedConstruction: hardware?.sharedConstruction ?? mechanical });
    onSelect(next[0].id, boardId);
  };
  return <section className="wb-hardware-instances" aria-label="Physical assembly">
    {context === 'case' && <>
    <p>Mechanical settings and closure hardware apply to all case assemblies. Select an assembly in Objects.</p>
    {selected && <>
      <p>{selected.name} · {instances.some(instance => instance.id !== selected.id && instance.boardId === selected.boardId) ? 'Shared PCB' : document.boards.find(board => board.id === selected.boardId)?.name} · {selected.role}</p>
      <details><summary>Assembly setup</summary>
        {hardware?.topology === 'split' && <label>Half connection<select value={hardware.transport} onChange={event => onChange({ ...hardware, transport: event.target.value as 'wireless' | 'wired' })}><option value="wireless">Wireless · local battery on each half</option><option value="wired">Wired serial · local power on each half</option></select></label>}
        {hardware?.transport === 'wired' && <p>Use a straight TRRS cable: tip and ring 2 carry crossed TX/RX, sleeve is ground, ring 1 is unused. Power both halves locally and unplug power before connecting.</p>}
        <label>PCB design<select value={selected.boardId} onChange={event => { patch({ boardId: event.target.value, mechanical: selected.mechanical && { ...selected.mechanical, boardId: event.target.value } }); onSelect(selected.id, event.target.value); }}>{document.boards.map(board => <option key={board.id} value={board.id}>{board.name}</option>)}</select></label>
        <label><input type="checkbox" checked={selected.flipped} onChange={event => patch({ flipped: event.target.checked })} />Turn PCB over for this half</label>
        {document.boardReferences?.some(reference => reference.boardId === selected.boardId) && <p>Imported routing is a reference. Review it after changing the assembly.</p>}
      </details>
    </>}
    </>}
    {context === 'setup' && <>
    <SetupChoice label="Keyboard configuration">
      <button type="button" aria-pressed={hardware?.topology === 'unibody'} onClick={() => { if (hardware?.topology !== 'unibody') configure(false); }}><SetupIcon kind="keyboard"/>One keyboard</button>
      <button type="button" aria-pressed={hardware?.topology === 'split'} onClick={() => { if (hardware?.topology !== 'split') configure(true); }}><SetupIcon kind="split"/>Split keyboard</button>
    </SetupChoice>
    {hardware?.topology === 'split' && <SetupChoice label="Half connection">
      <button type="button" aria-pressed={hardware.transport === 'wireless'} onClick={() => onChange({ ...hardware, transport: 'wireless' })}><SetupIcon kind="wireless"/>Wireless</button>
      <button type="button" aria-pressed={hardware.transport === 'wired'} onClick={() => onChange({ ...hardware, transport: 'wired' })}><SetupIcon kind="wired"/>Wired</button>
    </SetupChoice>}
    <label className="wb-reversible-choice"><input type="checkbox" checked={reversible} onChange={event => onReversibleChange(event.target.checked)}/>Reversible layout</label>
    {hardware?.topology === 'split' && <p className="wb-empty-note">{reversible ? 'Use the same PCB on both halves, turned over on the right.' : 'Both halves use front-facing assemblies.'}</p>}
    </>}
  </section>;
}
