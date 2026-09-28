import type { HardwareConfiguration, PhysicalBoardInstance, ProjectDoc } from '@boardstudio/v2-contracts';
import { createMechanicalConfiguration } from '../mechanicalPresets';

export function HardwareInstancesPanel({ document, boardId, selectedId, onSelect, onChange }: {
  document: ProjectDoc;
  boardId: string;
  selectedId?: string;
  onSelect: (id: string, boardId: string) => void;
  onChange: (hardware: HardwareConfiguration) => void;
}) {
  const hardware = document.hardware;
  const instances = hardware?.instances ?? [];
  const selected = instances.find(instance => instance.id === selectedId);
  const patch = (change: Partial<PhysicalBoardInstance>) => {
    if (hardware && selected) onChange({ ...hardware, instances: instances.map(instance => instance.id === selected.id ? { ...instance, ...change } : instance) });
  };
  const configure = (split: boolean) => {
    const mechanical = document.mechanical?.boardId === boardId ? document.mechanical : createMechanicalConfiguration(document, boardId);
    const make = (half: string, role: string, flipped: boolean): PhysicalBoardInstance => ({
      id: crypto.randomUUID(), name: half === 'unibody' ? 'Keyboard' : `${half === 'left' ? 'Left' : 'Right'} half`,
      boardId, half, role, flipped, constructionLinked: true, controllerPartId: null,
      mechanical: { ...mechanical, openings: flipped ? [] : mechanical.openings, mounts: flipped ? [] : mechanical.mounts },
    });
    const next = split ? [make('left', 'central', false), make('right', 'peripheral', true)] : [make('unibody', 'standalone', false)];
    onChange({ ...hardware, topology: split ? 'split' : 'unibody', transport: split ? 'wireless' : 'none', boards: hardware?.boards ?? [], instances: next, sharedConstruction: mechanical });
    onSelect(next[0].id, boardId);
  };
  if (!instances.length) return <section className="wb-hardware-instances" aria-label="Physical assembly">
    <h3>Physical assembly</h3>
    <p>Choose one keyboard or two physical halves. Each half keeps its own case and openings.</p>
    <div className="wb-row"><button className="wb-secondary" onClick={() => configure(false)}>One keyboard</button><button className="wb-secondary" onClick={() => configure(true)}>Split keyboard</button></div>
  </section>;
  return <section className="wb-hardware-instances" aria-label="Physical assembly">
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
  </section>;
}
