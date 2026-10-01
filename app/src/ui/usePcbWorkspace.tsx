import { useState } from 'react';
import type { EditCommand, JsonValue, Part, PartDefinition, ProjectDoc } from '@boardstudio/v2-contracts';
import { isErgogen, parameters as ergogenParameterSchema } from '@boardstudio/v2-ergogen';
import { assignNetPins } from './createWorkbenchEditActions';
import { InspectorSection } from './InspectorSection';
import { WiringPanel } from './WiringPanel';
import { ArrowIcon } from './WorkbenchIcons';
import { Measure } from './InspectorControls';
import { makeId } from './workbenchGeometry';
import type { Props } from './workbenchTypes';

type Inputs = {
  document: ProjectDoc; boardId: string; activePart?: Part; definitions: ReadonlyMap<string, PartDefinition>;
  wiring: Props['wiring']; onResolveWiring: Props['onResolveWiring']; onApplyWiring: Props['onApplyWiring']; onReviewWiring: Props['onReviewWiring'];
  chooseController: () => void; clearSelection: () => void; emit: (operation: EditCommand['operation'], ids: string[]) => unknown;
};

export function usePcbWorkspace({ document, boardId, activePart, definitions, wiring, onResolveWiring, onApplyWiring, onReviewWiring, chooseController, clearSelection, emit }: Inputs) {
  const [newNetName, setNewNetName] = useState('');
  const selectedBoard = document.boards.find(board => board.id === boardId);
  const updatePartGeneratorParameter = (key: string, value: JsonValue | undefined) => {
    if (!activePart) return;
    const generatorParameters = { ...(activePart.generatorParameters ?? {}) };
    if (value === undefined || value === '') delete generatorParameters[key]; else generatorParameters[key] = value;
    const next = { ...activePart, generatorParameters };
    emit({ kind: 'replace-document', document: { ...document, parts: document.parts.map((part) => part.id === next.id ? next : part) } }, [next.id]);
  };

  const assignNet = (padId: string, netId: string) => {
    if (!activePart) return;
    const nets = assignNetPins(document.nets, activePart.id, [padId], netId);
    emit({ kind: 'replace-document', document: { ...document, nets } }, [activePart.id, padId, netId]);
  };

  const assignTerminal = (terminal: string, padIds: string[], netId: string) => {
    if (!activePart) return;
    const nets = assignNetPins(document.nets, activePart.id, padIds, netId);
    emit({ kind: 'replace-document', document: { ...document, nets } }, [activePart.id, ...padIds, terminal]);
  };


  const addNet = () => {
    const name = newNetName.trim();
    if (!name) return;
    const net: ProjectDoc['nets'][number] = { id: makeId(), name, pins: [] };
    const boards = document.boards.map((board) => board.id === selectedBoard?.id ? { ...board, netIds: [...board.netIds, net.id] } : board);
    emit({ kind: 'replace-document', document: { ...document, nets: [...document.nets, net], boards } }, [net.id, selectedBoard?.id ?? ''].filter(Boolean));
    setNewNetName('');
  };

  const panel = () => {
      const boardNets = document.nets.filter(net => (selectedBoard ? selectedBoard.netIds.includes(net.id) || net.pins.some(pin => selectedBoard.partIds.includes(pin.partId)) : true));
      const activeDefinition = activePart ? definitions.get(activePart.definitionId) : undefined;
      const activeErgogenParams = activeDefinition?.generator && isErgogen(activeDefinition.generator.source) ? ergogenParameterSchema(activeDefinition.generator.source) : {};
      const builtinSwitch = activePart && activeDefinition?.kind === 'switch';
      if (wiring && (!activePart || activeDefinition?.kind === 'controller')) return <WiringPanel {...wiring} onAddController={chooseController} onResolve={onResolveWiring} onApply={onApplyWiring} onReview={onReviewWiring} onToggleLock={onReviewWiring} />;
      if (builtinSwitch && activePart && activeDefinition) return <><div className="wb-inspect-head"><h2>{activePart.reference} · {activeDefinition.name}</h2><span className="wb-mini-tag">{selectedBoard?.name ?? 'Board'} / PCB</span></div><p className="wb-empty-note">Switch wiring is inherited from its key assembly and resolved by the board wiring plan.</p><InspectorSection title="Named terminals" detail={`${Object.keys(activeDefinition.terminals ?? {}).length}`} defaultOpen><div className="wb-net-map">{Object.entries(activeDefinition.terminals ?? {}).map(([terminal, padIds]) => { const net = boardNets.find((entry) => entry.pins.some((pin) => pin.partId === activePart.id && padIds.includes(pin.padId))); return <div className="wb-net-map-row" key={terminal}><span>{terminal}</span><strong>{net?.name ?? 'Unmapped'}</strong></div>; })}</div></InspectorSection><button className="wb-inspector-link" onClick={clearSelection}>Edit board wiring <ArrowIcon /></button></>;
      const assigned = boardNets.reduce((total, net) => total + net.pins.filter((pin) => selectedBoard?.partIds.includes(pin.partId) ?? true).length, 0);
      return <>
        <div className="wb-inspect-head"><h2>{activePart && activeDefinition ? `${activePart.reference} · ${activeDefinition.name}` : 'Board setup'}</h2><span className="wb-mini-tag">{activePart && activeDefinition ? `${selectedBoard?.name ?? 'Board'} / PCB` : `${document.boards.length} board${document.boards.length === 1 ? '' : 's'}`}</span></div>
        {activePart && activeDefinition && (activeDefinition.inputProfile?.press || activeDefinition.generator?.source === 'ceoloide/rotary_encoder_ec11_ec12') && <InspectorSection title="Press input" defaultOpen>
          <label>Scan mode<select aria-label="Press scan mode" value={String(activePart.properties?.pressScanMode ?? (document.matrices.some(matrix => matrix.partIds.includes(activePart.id)) ? 'matrix' : 'direct'))} onChange={event => emit({ kind: 'set-input-scan-mode', partId: activePart.id, mode: event.target.value as 'matrix' | 'direct' | 'unassigned' }, [activePart.id])}>
            <option value="matrix" disabled={!document.matrices.some(matrix => matrix.partIds.includes(activePart.id)) || activeDefinition.inputProfile?.press?.independent === false}>Matrix key</option>
            <option value="direct">Direct GPIO</option><option value="unassigned">Unassigned</option>
          </select></label><p className="wb-empty-note">Rotation uses separate GPIOs. Apply the board wiring plan after changing the press connection.</p>
        </InspectorSection>}
        <InspectorSection title="Board details" detail={selectedBoard?.name}>
        {selectedBoard ? <dl className="wb-measure-list">
          <Measure label="Board" value={selectedBoard.name} />
          <Measure label="Thickness" value={`${selectedBoard.thickness.toFixed(2)} mm`} />
          <Measure label="Placed parts" value={`${selectedBoard.partIds.length}`} />
          <Measure label="Net assignments" value={`${assigned}`} />
        </dl> : <p className="wb-empty-note">Add a board in the project setup to begin mapping nets.</p>}
        </InspectorSection>
        <div className="wb-findings-head"><h3 className="wb-subtitle">Connections</h3><span>{activeDefinition?.pads.length ?? 0}</span></div>
        {activePart && activeDefinition ? <div className="wb-net-map">
          {Object.entries(activeDefinition.terminals ?? {}).map(([terminal, padIds]) => {
            const assigned = [...new Set(padIds.map((padId) => boardNets.find((net) => net.pins.some((pin) => pin.partId === activePart.id && pin.padId === padId))?.id).filter((id): id is string => Boolean(id)))];
            return <label className="wb-net-map-row" key={`terminal:${terminal}`}><span>{terminal} terminal</span><select aria-label={`Net for terminal ${terminal}`} value={assigned.length === 1 ? assigned[0] : ''} onChange={(event) => assignTerminal(terminal, padIds, event.target.value)}>
              <option value="">Unmapped</option>{boardNets.map((net) => <option key={net.id} value={net.id}>{net.name}</option>)}
            </select></label>;
          })}
          {activeDefinition.pads.filter((pad) => pad.plated !== false && pad.number !== '' && !isErgogen(activeDefinition.generator?.source) && !Object.values(activeDefinition.terminals ?? {}).some((padIds) => padIds.includes(pad.id))).map((pad) => {
          const assigned = boardNets.find((net) => net.pins.some((pin) => pin.partId === activePart.id && pin.padId === pad.id));
          return <label className="wb-net-map-row" key={pad.id}><span className="wb-pad-number">{pad.number}</span><select aria-label={`Net for pad ${pad.number}`} value={assigned?.id ?? ''} onChange={(event) => assignNet(pad.id, event.target.value)}>
            <option value="">Unmapped</option>{boardNets.map((net) => <option key={net.id} value={net.id}>{net.name}</option>)}
          </select></label>;
        })}</div> : <p className="wb-empty-note">Select a placed part to map its pads to nets.</p>}
        {activePart && Object.keys(activeErgogenParams).some((key) => activeErgogenParams[key].type === 'net' || activeErgogenParams[key].type === 'anchor') && <>
          <div className="wb-panel-rule" /><h3 className="wb-subtitle">Ergogen bindings</h3>
          <div className="wb-net-map">{Object.entries(activeErgogenParams).filter(([key, parameter]) => (parameter.type === 'net' && !activeDefinition?.terminals?.[key]) || parameter.type === 'anchor').map(([key, parameter]) => {
            const value = activePart.generatorParameters?.[key];
            if (parameter.type === 'net') return <label className="wb-net-map-row" key={key}><span>{key}</span><select aria-label={`Ergogen net ${key}`} value={typeof value === 'string' ? value : ''} onChange={(event) => updatePartGeneratorParameter(key, event.target.value || undefined)}><option value="">Default</option>{boardNets.map((net) => <option key={net.name} value={net.name}>{net.name}</option>)}</select></label>;
            const anchor = value && typeof value === 'object' && !Array.isArray(value) ? value as { x?: unknown; y?: unknown } : {};
            const updateAnchor = (axis: 'x' | 'y', raw: string) => {
              const next = { ...anchor };
              if (raw.trim() === '') delete next[axis];
              else {
                const coordinate = Number(raw);
                if (!Number.isFinite(coordinate)) return;
                next[axis] = coordinate;
              }
              updatePartGeneratorParameter(key, Object.keys(next).length ? next as JsonValue : undefined);
            };
            return <div className="wb-net-map-row wb-anchor-row" key={key}><span>{key}</span><label>{key} X<input type="number" aria-label={`Ergogen anchor ${key} X`} value={typeof anchor.x === 'number' ? anchor.x : ''} placeholder="Part X" onChange={(event) => updateAnchor('x', event.target.value)} /></label><label>{key} Y<input type="number" aria-label={`Ergogen anchor ${key} Y`} value={typeof anchor.y === 'number' ? anchor.y : ''} placeholder="Part Y" onChange={(event) => updateAnchor('y', event.target.value)} /></label></div>;
          })}</div>
        </>}
        <form className="wb-new-net" onSubmit={(event) => { event.preventDefault(); addNet(); }}>
          <input aria-label="New net name" placeholder="New net name" value={newNetName} onChange={(event) => setNewNetName(event.target.value)} />
          <button type="submit" disabled={!newNetName.trim()}>Add net</button>
        </form>
        <div className="wb-panel-rule" />
        <InspectorSection title="Electrical nets" detail={String(boardNets.length)}>
        <div className="wb-net-list">{boardNets.map((net) => <div className="wb-net-row" key={net.id}>
          <span className="wb-net-swatch" /> <span>{net.name}</span><small>{net.pins.filter((pin) => selectedBoard?.partIds.includes(pin.partId) ?? true).length} pins</small>
        </div>)}</div>
        </InspectorSection>
      </>;
    };
  return { panel };
}
