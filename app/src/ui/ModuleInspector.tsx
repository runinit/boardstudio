import { useEffect, useState } from 'react';
import type { EditOperation, ModuleConnection, ModuleDefinition, ModuleSupport, MountedModule, PartDefinition, ProjectDoc, VikSignal } from '../../../contracts/src/index';
import { DraftInput } from './InspectorControls';
import { InspectorSection } from './InspectorSection';
import { HardwareReadiness } from './HardwareReadiness';
import { ModuleProfileEditor } from './ModuleProfileEditor';
import { makeId } from './workbenchGeometry';
import './module-workspace.css';

const signalNames: [VikSignal, string][] = [['sclk','SPI clock'],['miso','SPI MISO'],['cs','Chip select'],['gpio2','GPIO 2'],['mosi','SPI MOSI'],['gpio1','GPIO 1'],['v5','5V supply'],['rgb','RGB data'],['scl','I²C clock'],['sda','I²C data'],['gnd','Ground'],['v3v3','3.3V supply']];
type Props = {
  document: ProjectDoc; definition: ModuleDefinition; variants: ModuleDefinition[];
  boardId: string; onSelect: (id: string) => void;
  onEdit: (operation: EditOperation, targets: string[]) => unknown;
  onPlacePart: (definition: PartDefinition) => void;
};
function initialPlacement(definition: ModuleDefinition, boardId: string): MountedModule {
  return {id:`module/${makeId()}`,definitionId:definition.id,hostBoardId:boardId,hostFace:'front',facingFace:'back',at:{x:0,y:0},rotation:0,gap:3,attachment:'board',detached:false,serviceClearance:0,mountSupports:[]};
}

export function ModuleInspector({ document, definition, variants, boardId, onSelect, onEdit, onPlacePart }: Props) {
  const instances = (document.modules ?? []).filter(instance => instance.definitionId === definition.id);
  const [selected, setSelected] = useState(() => instances[0]?.id ?? '');
  const [draft, setDraft] = useState(() => instances[0] ?? initialPlacement(definition, boardId));
  const [joins, setJoins] = useState<Record<string,string>>({});
  const [error, setError] = useState('');
  const [pending, setPending] = useState(false);
  const [connectionEnabled, setConnectionEnabled] = useState(Boolean(draft.connection));
  const [supportDraft, setSupportDraft] = useState({mountId:'',outerDiameter:'',holeDiameter:'',z:'',height:''});
  const selectedInstance = instances.find(instance => instance.id === selected);
  const savedPlacement = JSON.stringify(selectedInstance ?? null);
  useEffect(() => {
    if (savedPlacement === 'null') return;
    const instance = JSON.parse(savedPlacement) as MountedModule;
    setDraft({...instance,mountSupports:instance.mountSupports ?? []});
    setConnectionEnabled(Boolean(instance.connection));
    setError('');
  }, [savedPlacement]);
  const chooseInstance = (id: string) => {
    const instance = instances.find(item => item.id === id);
    setSelected(id); setDraft(instance ? {...instance,mountSupports:instance.mountSupports ?? []} : initialPlacement(definition, boardId));
    setSupportDraft({mountId:'',outerDiameter:'',holeDiameter:'',z:'',height:''});
    setConnectionEnabled(Boolean(instance?.connection)); setError('');
  };
  const commit = async (operation: EditOperation, targets: string[]) => {
    setPending(true); setError('');
    try { await onEdit(operation, targets); }
    catch (failure) { setError(String(failure instanceof Error ? failure.message : failure)); }
    finally { setPending(false); }
  };
  const connection: ModuleConnection = draft.connection ?? {hostConnectorPartId:'',modulePortId:definition.interfaces.find(port=>port.role==='module')?.id ?? '',busId:'',assignments:{},cableType:'type-a-12-0.5',railVoltages:{}};
  const updateConnection = (patch: Partial<ModuleConnection>) => setDraft({...draft,connection:{...connection,...patch}});
  const connectors = document.parts.filter(part => document.boards.find(board=>board.id===draft.hostBoardId)?.partIds.includes(part.id) && document.definitions.find(def=>def.id===part.definitionId)?.hardwareProfile?.vikRole==='host');
  const instancesForBoard = document.hardware?.instances?.filter(instance => instance.boardId === draft.hostBoardId) ?? [];
  const addMountSupport = () => {
    const sourceMount = definition.mounts.find(mount=>mount.sourceId===supportDraft.mountId);
    const fields = [supportDraft.outerDiameter,supportDraft.holeDiameter,supportDraft.z,supportDraft.height];
    const values = fields.map(Number);
    if (!sourceMount || fields.some(value=>!value.trim()) || values.some(value=>!Number.isFinite(value)) || values[0] <= values[1] || values[1] < sourceMount.diameter || values[3] <= 0) {
      setError('Select a source mounting hole and enter finite dimensions; outer diameter must exceed the hole, and the hole must clear the source drill.'); return;
    }
    const support:ModuleSupport={mountId:sourceMount.sourceId,outerDiameter:values[0],holeDiameter:values[1],z:values[2],height:values[3]};
    setDraft({...draft,mountSupports:[...(draft.mountSupports ?? []),support]});
    setSupportDraft({mountId:'',outerDiameter:'',holeDiameter:'',z:'',height:''});setError('');
  };
  const embeddingGroups = (document.embeddedCircuits ?? []).filter(group=>group.definitionId===definition.id);
  return <div className="wb-module-inspector">
    <div className="wb-inspector-heading"><div><h2>{definition.name}</h2><small>VIK · {definition.family.replaceAll('-', ' ')}</small></div></div>
    {document.parameters.demo === 'vik-module-review' && <aside className="wb-module-review-note" role="note" aria-label="Review fixture assumptions">
      <strong>Review fixture · assumptions</strong>
      <p>Module anchors and 3 mm face gaps are illustrative. Source outlines and board thicknesses come from the pinned snapshot; component, cable, support and actuator envelopes are incomplete. Inspect the above/below transforms and output findings in the app. These placements are not manufacturing-ready.</p>
    </aside>}
    <label>Variant<select aria-label="Module variant" value={definition.id} onChange={event=>onSelect(event.target.value)}>{variants.map(variant=><option key={variant.id} value={variant.id}>{variant.variant}</option>)}</select></label>
    <HardwareReadiness source={definition.source} gates={definition.gates}/>
    <ModuleProfileEditor definition={definition} onSave={snapshot=>onEdit({kind:'set-module-definition',definition:snapshot},[snapshot.id])}/>
    <InspectorSection title="Mounted module" detail={`${instances.length} placed`} defaultOpen>
      <label>Placement<select aria-label="Module placement" value={selected} onChange={event=>chooseInstance(event.target.value)}><option value="">New placement</option>{instances.map(instance=><option key={instance.id} value={instance.id}>{document.boards.find(board=>board.id===instance.hostBoardId)?.name} · {instance.hostFace} · {instance.id.split('/').at(-1)}</option>)}</select></label>
      <form onSubmit={event=>{event.preventDefault();const instance={...draft,connection:connectionEnabled?connection:undefined};void commit({kind:'set-mounted-module',instance,definition},[instance.id]);setSelected(instance.id);}}>
        <label>Host board<select aria-label="Module host board" value={draft.hostBoardId} onChange={event=>setDraft({...draft,hostBoardId:event.target.value,hostInstanceId:undefined})}>{document.boards.map(board=><option key={board.id} value={board.id}>{board.name}</option>)}</select></label>
        <label>Physical instance<select aria-label="Module physical instance" value={draft.hostInstanceId ?? ''} onChange={event=>setDraft({...draft,hostInstanceId:event.target.value||undefined})}><option value="">Every instance of this board</option>{instancesForBoard.map(instance=><option key={instance.id} value={instance.id}>{instance.name}</option>)}</select></label>
        <div className="wb-module-fields"><label>Host face<select aria-label="Module host face" value={draft.hostFace} onChange={event=>setDraft({...draft,hostFace:event.target.value as MountedModule['hostFace']})}><option value="front">Above PCB</option><option value="back">Below PCB</option></select></label>
          <label>Facing surface<select aria-label="Module facing surface" value={draft.facingFace} onChange={event=>setDraft({...draft,facingFace:event.target.value as MountedModule['facingFace']})}><option value="back">Module back</option><option value="front">Module front</option></select></label></div>
        <div className="wb-module-fields">{(['x','y'] as const).map(axis=><label key={axis}>{axis.toUpperCase()} · mm<DraftInput ariaLabel={`Module ${axis.toUpperCase()}`} type="number" step="0.1" value={draft.at[axis]} onCommit={value=>setDraft({...draft,at:{...draft.at,[axis]:Number(value)}})}/></label>)}
          <label>Yaw · °<DraftInput ariaLabel="Module yaw" type="number" step="1" value={draft.rotation} onCommit={value=>setDraft({...draft,rotation:Number(value)})}/></label>
          <label>Surface gap · mm<DraftInput ariaLabel="Module surface gap" type="number" min="0" step="0.1" value={draft.gap} onCommit={value=>setDraft({...draft,gap:Number(value)})}/></label></div>
        <label>Attachment<select aria-label="Module attachment" value={draft.attachment} onChange={event=>setDraft({...draft,attachment:event.target.value as MountedModule['attachment']})}><option value="board">Travels with PCB</option><option value="case">Fixed to case</option></select></label>
        <label>Extra service clearance · mm<DraftInput ariaLabel="Module service clearance" type="number" min="0" step="0.1" value={draft.serviceClearance} onCommit={value=>setDraft({...draft,serviceClearance:Number(value)})}/></label>
        {draft.attachment==='case' && <InspectorSection title="Case support rings" detail={`${draft.mountSupports?.length ?? 0} specified`}>
          <p className="wb-empty-note">Choose a source PCB hole and designer-selected ring dimensions. Z and height are module-midplane millimetres. The generator checks PCB and case contact; these values are not vendor specifications.</p>
          {(draft.mountSupports ?? []).map((support,index)=><div className="wb-module-existing" key={`${support.mountId}/${index}`}><span>{support.mountId} · OD {support.outerDiameter} / ID {support.holeDiameter} · Z {support.z} · height {support.height} mm</span><button type="button" className="wb-inspector-link" onClick={()=>setDraft({...draft,mountSupports:(draft.mountSupports ?? []).filter((_,i)=>i!==index)})}>Remove</button></div>)}
          <label>Source mounting hole<select aria-label="Support source mounting hole" value={supportDraft.mountId} onChange={event=>setSupportDraft({...supportDraft,mountId:event.target.value})}><option value="">Choose module hole…</option>{definition.mounts.map(mount=><option key={mount.sourceId} value={mount.sourceId}>{mount.sourceId} · source drill {mount.diameter} mm</option>)}</select></label>
          <div className="wb-module-fields">{(['outerDiameter','holeDiameter','z','height'] as const).map(key=><label key={key}>{({outerDiameter:'Outer diameter',holeDiameter:'Hole diameter',z:'Z from midplane',height:'Ring height'} as const)[key]} · mm<DraftInput ariaLabel={`Support ${key}`} type="number" step="0.1" value={supportDraft[key]} onCommit={value=>setSupportDraft({...supportDraft,[key]:value})}/></label>)}</div>
          <button type="button" onClick={addMountSupport} disabled={!definition.mounts.length}>Add specified ring</button>
          {!definition.mounts.length && <p className="wb-empty-note">This module snapshot has no source mounting holes for ring placement.</p>}
        </InspectorSection>}
        <label className="wb-module-check"><input type="checkbox" checked={draft.detached} onChange={event=>setDraft({...draft,detached:event.target.checked})}/>Detached from assembly</label>
        <InspectorSection title="VIK connection" detail={connectionEnabled?'Connected':'Unassigned'}>
          <label className="wb-module-check"><input type="checkbox" checked={connectionEnabled} onChange={event=>setConnectionEnabled(event.target.checked)}/>Assign host connection</label>
          {connectionEnabled && <>
            <label>Host connector<select aria-label="Module host connector" value={connection.hostConnectorPartId} onChange={event=>updateConnection({hostConnectorPartId:event.target.value})}><option value="">Select VIK host connector</option>{connectors.map(part=><option key={part.id} value={part.id}>{part.reference}</option>)}</select></label>
            {!connectors.length && <p className="wb-empty-note">Place a VIK host connector on this board first. Splitter constituents include host footprints.</p>}
            <label>Module port<select aria-label="Module input port" value={connection.modulePortId} onChange={event=>updateConnection({modulePortId:event.target.value})}>{definition.interfaces.filter(port=>port.role==='module').map(port=><option key={port.id} value={port.id}>{port.id}</option>)}</select></label>
            <label>Bus name<DraftInput ariaLabel="Module bus name" value={connection.busId} onCommit={value=>updateConnection({busId:value})}/></label>
            <p className="wb-empty-note">12 contacts · 0.5 mm pitch · Type A cable · 3.3V logic. Enter actual MCU terminals; shared buses are checked by their wiring.</p>
            <div className="wb-module-signals">{signalNames.map(([signal,label])=><label key={signal}>{label}<DraftInput ariaLabel={`VIK ${label} terminal`} value={connection.assignments[signal] ?? ''} onCommit={value=>{const assignments={...connection.assignments};if(value.trim())assignments[signal]=value.trim();else delete assignments[signal];updateConnection({assignments});}}/></label>)}</div>
            <div className="wb-module-fields">{(['v3v3','v5','gnd'] as const).map(signal=><label key={signal}>{signalNames.find(([name])=>name===signal)?.[1]} · V<DraftInput ariaLabel={`VIK ${signal} voltage`} type="number" step="0.1" value={connection.railVoltages[signal] ?? ''} onCommit={value=>{const railVoltages={...connection.railVoltages};if(value.trim())railVoltages[signal]=Number(value);else delete railVoltages[signal];updateConnection({railVoltages});}}/></label>)}</div>
            <label>Supply budget · mA<DraftInput ariaLabel="VIK supply current budget" type="number" min="0" value={connection.supplyCurrentMa ?? ''} onCommit={value=>updateConnection({supplyCurrentMa:value.trim()?Number(value):undefined})}/></label>
          </>}
        </InspectorSection>
        <button className="wb-primary" disabled={pending || !draft.hostBoardId} type="submit">{selectedInstance?'Save placement':'Attach module'}</button>
      </form>
      {selected && <button className="wb-inspector-link" disabled={pending} onClick={()=>{void commit({kind:'remove-mounted-module',id:selected},[selected]);chooseInstance('');}}>Remove placement</button>}
    </InspectorSection>
    {definition.circuit && <InspectorSection title="Use circuit on PCB" detail={`${definition.circuit.parts.length} components`}>
      <p className="wb-empty-note">Creates an independent, editable copy. Choose host nets for explicit joins; other nets remain local to this copy.</p>
      {definition.constituents.some(part=>part.purchased)&&<p className="wb-empty-note">This adapter includes a purchased assembly. Its footprint and external contacts are available; its internal electronics are absent from the source.</p>}
      {Object.keys(definition.circuit.ports).map(port=><label key={port}>{port.toUpperCase()} joins<select aria-label={`Circuit ${port} host net`} value={joins[port] ?? ''} onChange={event=>{const next={...joins};if(event.target.value)next[port]=event.target.value;else delete next[port];setJoins(next);}}><option value="">Separate local net</option>{document.nets.filter(net=>document.boards.find(board=>board.id===draft.hostBoardId)?.netIds.includes(net.id)).map(net=><option key={net.id} value={net.id}>{net.name}</option>)}</select></label>)}
      <button className="wb-primary" disabled={pending || !draft.hostBoardId} onClick={()=>{const id=`circuit/${makeId()}`;void commit({kind:'embed-module-circuit',id,definition,hostBoardId:draft.hostBoardId,pose:{at:draft.at,rotation:draft.rotation},side:draft.hostFace,joins},[id]);}}>Copy circuit to PCB</button>
      {definition.circuit.adaptations.length>0 && <ul>{definition.circuit.adaptations.map(adaptation=><li key={adaptation}>{adaptation}</li>)}</ul>}
      {embeddingGroups.map(group=><div className="wb-module-existing" key={group.id}><span>{group.partIds.length} components · {group.id.split('/').at(-1)}</span><button className="wb-inspector-link" disabled={pending} onClick={()=>void commit({kind:'remove-embedded-circuit',id:group.id},[group.id])}>Remove copy</button></div>)}
    </InspectorSection>}
    <InspectorSection title="Individual components" detail={`${definition.constituents.length}`}>
      <p className="wb-empty-note">Place available source footprints independently. Complete assembly models and component heights have separate readiness reviews.</p>
      {definition.constituents.map((part,index)=>{const footprint=definition.circuit?.definitions.find(item=>item.id===part.definitionId);return <div className="wb-module-constituent" key={`${part.reference}/${index}`}><div><strong>{part.reference} · {part.name}</strong><small>{part.purchased?'Purchased assembly · footprint and external contacts only':part.footprint}</small></div><button disabled={!footprint} onClick={()=>footprint&&onPlacePart(footprint)}>Place</button></div>;})}
    </InspectorSection>
    <InspectorSection title="Available 3D models" detail={`${definition.candidateModels?.length ?? 0}`}>
      <p className="wb-empty-note">Displayed dimensions are imported source bounds in asset axes. They help review scale; board alignment and selected assembly population remain unverified.</p>
      {(definition.candidateModels ?? []).map(model=>{const extent=['x','y','z'].map(axis=>model.boundsMax[axis as 'x'|'y'|'z']-model.boundsMin[axis as 'x'|'y'|'z']);const bounds=`Source bounds · ${extent.map(value=>value.toFixed(3)).join(' × ')} mm`;return <div className="wb-module-constituent" key={model.assetId}><div><strong>{model.name}</strong><small>{bounds} · {model.verifiedAlignment?'Aligned':'Alignment unreviewed'} · {model.source.license}</small></div><a href={`${model.source.repository}/blob/${model.source.revision}/${model.source.path.split('/').map(encodeURIComponent).join('/')}`} target="_blank" rel="noreferrer">Source ↗</a></div>;})}
    </InspectorSection>
    {error && <p role="alert">{error}</p>}
  </div>;
}
