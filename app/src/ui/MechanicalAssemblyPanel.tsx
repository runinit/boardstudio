import { insertSizes, resizeInsert } from '../gasketEditing';
import { defaultGasketLayout, defaultInternalGasket, gasketFoamPresets } from '../gasketEditing';
import { CaseIcon } from './CaseChoice';
import { stabilizerCandidates } from '../mechanicalDefaults';
import type { GenerationState } from '../generationState';
import React from 'react';
import type {
  Finding,
  InternalClosureHardware,
  InternalGasketConfiguration,
  CaseOpening,
  MechanicalAssembly,
  MechanicalCriticalFit,
  MechanicalBuiltinProfile,
  MechanicalConfiguration,
  MechanicalExtraction,
  MechanicalGeometry,
  MechanicalHardwareSpecification,
  MechanicalPartProfile,
  MechanicalPurposeMapping,
  MechanicalPurpose,
  MechanicalStabilizerOverride,
  MechanicalSwitchFamily,
  Mount,
  PartDefinition,
  ProjectDoc,
  Vec2,
} from '@boardstudio/v2-contracts';
import { FindingList } from './FindingList';
import { MechanicalDraft } from './mechanicalDraft';
import { InspectorSection } from './InspectorSection';
import { caseReadiness, mechanicalFindings, type CaseReadiness } from './caseReadiness';
import { CaseGenerationControls } from './CaseGenerationControls';
import {
  createMechanicalConfiguration,
  defaultPlateFoamThickness,
  defaultPlateThickness,
  inferSwitchFamily,
  initialSwitchFamily,
  materialForProcess,
  materialOptionsForProcess,
  plateToPcbGap,
  profileSwitchFamily,
  switchMountingDatum,
} from '../mechanicalPresets';
import './mechanical-assembly.css';
import { applyMechanicalExtraction, MechanicalProfileController } from './mechanicalProfileController';

const numericFields = [
  ['plateThickness', 'Plate thickness'],
  ['plateFoamThickness', 'Plate foam'],
  ['pcbThickness', 'PCB thickness'],
  ['bottomFoamThickness', 'Bottom foam'],
    ['bottomThickness', 'Bottom thickness'],
  ['wallThickness', 'Wall thickness'],
  ['clearance', 'Clearance'],
] as const;
const manufacturingMethods = {
  'pcb-fr4': 'PCB FR-4',
  printed: '3D printed',
  cnc: 'CNC machined',
  'cut-sheet': 'Cut sheet',
};
const supportedConstraintVersion = '2026-09-24';
const layerProcessIds = ['plate', 'plate-foam', 'bottom-foam', 'bottom'] as const;
const defaultProcess = (partId: string, method: MechanicalConfiguration['method'], thickness: number) => ({
  partId,
  method: partId.endsWith('foam') ? 'cut-sheet' as const : method,
  material: materialForProcess(partId, partId.endsWith('foam') ? 'cut-sheet' : method),
  thickness,
  constraintsVersion: supportedConstraintVersion,
});
type HardwareSpec = MechanicalHardwareSpecification;
type CriticalFit = MechanicalCriticalFit;

type Props = {
  readiness?: CaseReadiness;
  diagnosticsRequest?: number;
  onDiagnosticsShown?: () => void;
  generationTarget?: HTMLElement | null;
  onRevealDiagnostics?: () => void;
  document: ProjectDoc;
  boardId?: string;
  projectSession?: number;
  instanceId?: string;
  definitions: PartDefinition[];
  configuration?: MechanicalConfiguration;
  assembly?: MechanicalAssembly;
  onChange: (configuration: MechanicalConfiguration | null) => void;
  onResolve?: () => void;
  onCancel?: () => void;
  generation?: GenerationState;
  livePreview?: boolean;
  onLivePreviewChange?: (enabled: boolean) => void;
  onExport?: () => void;
  onShowFinding?: (finding: Finding) => void;
  selectedLayer?: string;
  onSelectLayer?: (id: string) => void;
  onMechanicalProfile?: (definitionId: string, source: MechanicalBuiltinProfile, plateToPcb: number) => Promise<MechanicalPartProfile>;
  onExtractMechanicalProfile?: (source: string, mappings: MechanicalPurposeMapping[]) => Promise<MechanicalExtraction>;
  onEditParts?: (definitionId: string) => void;
};

function MountList({ title, value, onChange, allowAdd = true }: { title: string; value: Mount[]; onChange: (value: Mount[]) => void; allowAdd?: boolean }) {
  const update = (id: string, patch: Partial<Mount>) => onChange(value.map((mount) => mount.id === id ? { ...mount, ...patch } : mount));
  return <InspectorSection title={title} detail={`${value.length}`}>
    {value.map((mount, index) => <div className="wb-mech-mount" key={mount.id}>
      <header><strong>{mount.kind === 'boss' ? 'Boss' : 'Hole'} {index + 1}</strong><button type="button" className="wb-mech-quiet" onClick={() => onChange(value.filter((entry) => entry.id !== mount.id))}>Remove</button></header>
      <label className="wb-mech-field"><span>Mount type</span><select value={mount.kind} onChange={(event) => update(mount.id, { kind: event.target.value as Mount['kind'] })}><option value="hole">Hole</option><option value="boss">Boss</option></select></label>
      <div className="wb-mech-numbers"><NumberField label="Position X" value={mount.at.x} min={-1000000} onCommit={(x) => update(mount.id, { at: { ...mount.at, x } })} /><NumberField label="Position Y" value={mount.at.y} min={-1000000} onCommit={(y) => update(mount.id, { at: { ...mount.at, y } })} /><NumberField label="Hole diameter" value={mount.holeDiameter} onCommit={(holeDiameter) => update(mount.id, { holeDiameter })} min={0.1} /><NumberField label="Boss diameter" value={mount.bossDiameter ?? 5} onCommit={(bossDiameter) => update(mount.id, { bossDiameter })} min={0.1} />{mount.kind === 'boss' && <NumberField label="Boss height" value={mount.height ?? 5} onCommit={(height) => update(mount.id, { height })} min={0.1} />}</div>
    </div>)}
    {allowAdd && <button type="button" className="wb-mech-quiet" onClick={() => onChange([...value, { id: crypto.randomUUID(), at: { x: 0, y: 0 }, kind: 'hole', holeDiameter: 2.5, bossDiameter: 5, height: 5 }])}>Add {title.toLowerCase().replace(/s$/, '')}</button>}
  </InspectorSection>;
}

function NumberField({ label, value, onCommit, min = 0, max, step = 0.1, unit = 'mm' }: {
  label: string;
  value: number;
  onCommit: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
}) {
  const [draft, setDraft] = React.useState(String(value));
  React.useEffect(() => setDraft(String(value)), [value]);
  const commit = () => {
    const number = Number(draft);
    if (!Number.isFinite(number) || number < min || (max !== undefined && number > max)) {
      setDraft(String(value));
      return;
    }
    if (number !== value) onCommit(number);
  };
  return <label className="wb-mech-number"><span>{label}</span><span className="wb-mech-number-input"><input type="number" min={min} max={max} step={step} value={draft} onChange={(event) => setDraft(event.target.value)} onBlur={commit} onKeyDown={(event) => {
    if (event.key === 'Enter') event.currentTarget.blur();
    if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); setDraft(String(value)); }
  }} /><small>{unit}</small></span></label>;
}

function PolygonListEditor({ title, polygons, onChange }: { title: string; polygons: Vec2[][]; onChange: (polygons: Vec2[][]) => void }) {
  const updatePoint = (polygonIndex: number, pointIndex: number, axis: 'x' | 'y', value: number) => onChange(polygons.map((polygon, i) => i === polygonIndex ? polygon.map((point, j) => j === pointIndex ? { ...point, [axis]: value } : point) : polygon));
  return <div className="wb-mech-polygon-list"><header><strong>{title}</strong><button type="button" className="wb-mech-quiet" onClick={() => onChange([...polygons, [{ x: -2.5, y: -2.5 }, { x: 2.5, y: -2.5 }, { x: 2.5, y: 2.5 }, { x: -2.5, y: 2.5 }]])}>Add polygon</button></header>
    {polygons.map((polygon, polygonIndex) => <div className="wb-mech-polygon" key={polygonIndex}><div className="wb-mech-polygon-title"><strong>Contour {polygonIndex + 1}</strong><button type="button" className="wb-mech-quiet" onClick={() => onChange(polygons.filter((_, i) => i !== polygonIndex))}>Remove</button></div>
      {polygon.map((point, pointIndex) => <div className="wb-mech-point" key={pointIndex}><span>V{pointIndex + 1}</span><input aria-label={`${title} contour ${polygonIndex + 1} vertex ${pointIndex + 1} X`} type="number" step="0.1" value={point.x} onChange={(event) => updatePoint(polygonIndex, pointIndex, 'x', Number(event.target.value))} /><input aria-label={`${title} contour ${polygonIndex + 1} vertex ${pointIndex + 1} Y`} type="number" step="0.1" value={point.y} onChange={(event) => updatePoint(polygonIndex, pointIndex, 'y', Number(event.target.value))} /><button type="button" aria-label={`Remove ${title} vertex ${pointIndex + 1}`} onClick={() => onChange(polygons.map((entry, i) => i === polygonIndex ? entry.filter((_, j) => j !== pointIndex) : entry))} disabled={polygon.length <= 3}>×</button></div>)}
      <button type="button" className="wb-mech-quiet" onClick={() => onChange(polygons.map((entry, i) => i === polygonIndex ? [...entry, { x: 0, y: 0 }] : entry))}>Add vertex</button>
    </div>)}
    {!polygons.length && <p className="wb-mech-hint">No contours assigned.</p>}
  </div>;
}

function OpeningListEditor({ title, openings, onChange }: { title: string; openings: CaseOpening[]; onChange: (openings: CaseOpening[]) => void }) {
  return <div className="wb-mech-polygon-list"><header><strong>{title}</strong><button type="button" className="wb-mech-quiet" onClick={() => onChange([...openings, { points: [{ x: -2.5, y: -2.5 }, { x: 2.5, y: -2.5 }, { x: 2.5, y: 2.5 }, { x: -2.5, y: 2.5 }], z: 0, height: 10 }])}>Add volume</button></header>
    {openings.map((opening, index) => <div className="wb-mech-polygon" key={index}><div className="wb-mech-polygon-title"><strong>Volume {index + 1}</strong><button type="button" className="wb-mech-quiet" onClick={() => onChange(openings.filter((_, i) => i !== index))}>Remove</button></div>
      <div className="wb-mech-numbers"><NumberField label="Bottom Z" value={opening.z} min={-1000000} onCommit={(z) => onChange(openings.map((entry, i) => i === index ? { ...entry, z } : entry))} /><NumberField label="Height" value={opening.height} min={0.1} onCommit={(height) => onChange(openings.map((entry, i) => i === index ? { ...entry, height } : entry))} /></div>
      <PolygonListEditor title="XY footprint" polygons={[opening.points]} onChange={(polygons) => onChange(openings.map((entry, i) => i === index ? { ...entry, points: polygons[0] ?? [] } : entry))} />
    </div>)}
    {!openings.length && <p className="wb-mech-hint">No volumes configured.</p>}
  </div>;
}

function ProfileEditor({ profile, definitions, onChange, onRemove, onExtract, onSelectSwitchFamily }: {
  profile: MechanicalPartProfile;
  definitions: PartDefinition[];
  onChange: (profile: MechanicalPartProfile) => void;
  onRemove: () => void;
  onExtract?: (mappings: MechanicalPurposeMapping[]) => Promise<MechanicalExtraction>;
  onSelectSwitchFamily?: (family: MechanicalSwitchFamily) => void;
}) {
  const [geometry, setGeometry] = React.useState<MechanicalGeometry>();
  const [selectedPrimitives, setSelectedPrimitives] = React.useState<Set<string>>(new Set());
  const [primitivePurposes, setPrimitivePurposes] = React.useState<Record<string, MechanicalPurpose>>({});
  const [extracting, setExtracting] = React.useState(false);
  const [extractError, setExtractError] = React.useState('');
  const definition = definitions.find((entry) => entry.id === profile.definitionId);
  const loadGeometry = async () => {
    if (!onExtract) return;
    setExtracting(true); setExtractError('');
    try {
      const result = await onExtract([]);
      setGeometry(result.geometry);
      setSelectedPrimitives(new Set(profile.sourceGeometry?.sourceIds ?? []));
      setPrimitivePurposes(Object.fromEntries((profile.sourceGeometry?.mappings ?? []).filter((mapping) => mapping.sourceId).map((mapping) => [mapping.sourceId!, mapping.purpose])));
    } catch (cause) {
      setExtractError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setExtracting(false);
    }
  };
  const applyGeometry = async () => {
    if (!geometry || !onExtract || !selectedPrimitives.size) return;
    const mappings = geometry.primitives.filter((primitive) => selectedPrimitives.has(primitive.id)).map((primitive) => ({ sourceId: primitive.id, purpose: primitivePurposes[primitive.id] ?? 'plate-cutout' }));
    setExtracting(true); setExtractError('');
    try {
      const result = await onExtract(mappings);
      onChange(applyMechanicalExtraction(profile, result, definition?.name));
    } catch (cause) {
      setExtractError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setExtracting(false);
    }
  };

  return <section className="wb-mech-profile" aria-label={`${definition?.name ?? profile.definitionId} plate profile`}>
    <header><strong>{definition?.name ?? profile.definitionId}</strong><button type="button" className="wb-mech-quiet" onClick={onRemove}>Remove profile</button></header>
    <p className="wb-mech-hint">Profile source: {profile.source}. Cutouts repeat for every matching placed part.</p>
    {definition?.kicadSource && onExtract && <div className="wb-mech-geometry-picker">
      <div className="wb-mech-geometry-head"><strong>Custom geometry layers</strong><button type="button" className="wb-mech-quiet" onClick={() => void loadGeometry()} disabled={extracting}>{extracting ? 'Reading…' : geometry ? 'Reload layers' : 'Read KiCad layers'}</button></div>
      {geometry && <>
        <p className="wb-mech-hint">Select exact primitives that define plate cutouts. The extracted contour will replace this profile’s cutouts.</p>
        <div className="wb-mech-geometry-list">{geometry.primitives.length ? geometry.primitives.map((primitive) => <div className="wb-mech-geometry-row" key={primitive.id}><label><input type="checkbox" checked={selectedPrimitives.has(primitive.id)} onChange={(event) => setSelectedPrimitives((old) => { const next = new Set(old); if (event.target.checked) next.add(primitive.id); else next.delete(primitive.id); return next; })} /><span><strong>{primitive.layer || primitive.kind}</strong><small>{primitive.kind} · {primitive.sourceGroupId}</small></span></label><select aria-label={`Purpose for ${primitive.layer || primitive.kind}`} value={primitivePurposes[primitive.id] ?? 'plate-cutout'} onChange={(event) => setPrimitivePurposes((old) => ({ ...old, [primitive.id]: event.target.value as MechanicalPurpose }))}><option value="plate-cutout">Plate cutout</option><option value="electrical-pcb-mounting-hole">PCB mounting hole</option><option value="clearance-envelope">Clearance envelope</option><option value="drawing-guide">Drawing guide</option></select></div>) : <p className="wb-mech-hint">No geometric primitives were found in this footprint.</p>}</div>
        <button type="button" className="wb-mech-quiet" onClick={() => void applyGeometry()} disabled={!selectedPrimitives.size || extracting}>Apply selected geometry</button>
      </>}
      {extractError && <p className="wb-mech-error" role="alert">{extractError}</p>}
    </div>}
    <div className="wb-mech-points-head"><span>Cutout vertices</span><span>X / Y · mm</span></div>
    <PolygonListEditor title="Plate cutout geometry" polygons={profile.cutouts} onChange={(cutouts) => onChange({ ...profile, cutouts })} />
    <PolygonListEditor title="Component clearance zones" polygons={profile.clearances ?? []} onChange={(clearances) => onChange({ ...profile, clearances })} />
    <OpeningListEditor title="Access openings" openings={profile.openings ?? []} onChange={(openings) => onChange({ ...profile, openings })} />
    <OpeningListEditor title="Clearance volumes" openings={profile.clearanceVolumes ?? []} onChange={(clearanceVolumes) => onChange({ ...profile, clearanceVolumes })} />
    {definition?.kind === 'switch' && <label className="wb-mech-field"><span>Switch fit family</span><select value={profile.switchFamily ?? ''} onChange={(event) => { if (event.target.value) onSelectSwitchFamily?.(event.target.value as MechanicalSwitchFamily); }}><option value="">Choose switch family…</option><option value="mx">MX</option><option value="choc-v1">Choc v1</option><option value="choc-v2">Choc v2</option></select></label>}
    <div className="wb-mech-profile-offset"><label className="wb-mech-field"><span>Plate underside to PCB top</span><output>{profile.plateToPcb.toFixed(2)} mm · derived from switch fit</output></label><label className="wb-mech-field"><span>Supported plate thickness</span><output>{profile.supportedThickness ? `${profile.supportedThickness.x.toFixed(2)}–${profile.supportedThickness.y.toFixed(2)} mm` : 'Supplier review needed'}</output></label></div>
  </section>;
}

const closureDimensions = [
  ['threadDiameter', 'Thread diameter'], ['pitch', 'Thread pitch'],
  ['headDiameter', 'Head diameter'], ['headHeight', 'Head height'],
  ['holeDiameter', 'Screw clearance hole'], ['insertDiameter', 'Insert outside diameter'],
  ['insertLength', 'Insert length'], ['seatDiameter', 'Insert seat diameter'],
  ['seatDepth', 'Insert seat depth'], ['engagement', 'Thread engagement'],
  ['threadStart', 'Thread start offset'], ['tipAllowance', 'Screw tip allowance'],
  ['bottomingClearance', 'Bottoming clearance'], ['roof', 'Roof above insert'],
  ['surround', 'Material around insert'], ['seatLeadDepth', 'Seat lead-in depth'],
  ['seatLeadDiameter', 'Seat lead-in diameter'], ['bearingThickness', 'Material above screw head'],
] as const;

function ScrewLengths({ value, onChange }: { value: number[]; onChange: (value: number[]) => void }) {
  const [draft, setDraft] = React.useState(value.join(', '));
  const [error, setError] = React.useState('');
  const saved = value.join(', ');
  React.useEffect(() => { setDraft(saved); setError(''); }, [saved]);
  const commit = () => {
    const lengths = draft.split(',').map(item => Number(item.trim()));
    if (!lengths.length || lengths.some(length => !Number.isFinite(length) || length <= 0)) {
      setError('Enter positive lengths separated by commas.');
      return;
    }
    setError('');
    if (draft !== saved) onChange([...new Set(lengths)]);
  };
  return <><label className="wb-mech-field"><span>Available screw lengths (mm)</span><input type="text" className="wb-mech-text" value={draft} aria-invalid={Boolean(error)} onChange={event => setDraft(event.target.value)} onBlur={commit} onKeyDown={event => {
    if (event.key === 'Enter') event.currentTarget.blur();
    if (event.key === 'Escape') { setDraft(saved); setError(''); }
  }} /></label>{error && <p className="wb-mech-error" role="alert">{error}</p>}</>;
}

function GasketControls({ configuration, assembly, onChange, section = 'all' }: {
  section?: 'all' | 'gaskets' | 'hardware';
  configuration: MechanicalConfiguration;
  assembly?: MechanicalAssembly;
  onChange: (patch: Partial<MechanicalConfiguration>) => void;
}) {
  const layout = configuration.gasketLayout ?? defaultGasketLayout();
  const settings = configuration.internalGasket ?? defaultInternalGasket();
  const hardware = settings.hardware;
  const automaticCount = settings.autoCount ?? settings.supportCount === 4;
  const resolvedCount = assembly?.gasketSupports.filter(support => support.regionId === assembly.gasketSupports[0]?.regionId).length || settings.supportCount;
  const update = (patch: Partial<MechanicalConfiguration>) => onChange({
    internalGasket: settings, integratedPlateFrame: false, bottomStyle: 'shell', middleFrame: false, ...patch,
  });
  const updateSettings = (patch: Partial<InternalGasketConfiguration>) => update({ internalGasket: { ...settings, ...patch } });
  const updateHardware = (patch: Partial<InternalClosureHardware>) => updateSettings({ hardware: { ...hardware, ...patch } });
  const chooseSize = (id: string) => {
    const preset = gasketFoamPresets.find(item => item.id === id);
    update({ gasketLayout: { ...layout, ...(preset ? { length: preset.length, width: preset.width, thickness: preset.thickness } : {}), presetId: preset?.id } });
  };
  return <>
    {section !== 'hardware' && <><InspectorSection title="Gasket supports" detail={`${assembly?.gasketSupports?.length ?? 0} supports`} defaultOpen>
      <p className="wb-mech-hint">Select a gasket in the assembly tree to edit its cut length and width. Upper and lower pads form one support pair.</p>
      <label className="wb-mech-check"><input type="checkbox" checked={automaticCount} onChange={event => updateSettings({ autoCount:event.target.checked, supportCount:resolvedCount })} /><span>Choose support count automatically</span></label>
      <p className="wb-mech-hint">Automatic placement centers pads on long sections and fills usable shorter sections. Edit the count below to choose your own number of pairs.</p>
      <label className="wb-mech-field"><span>Foam stock</span><select value={layout.presetId ?? 'custom'} onChange={event => chooseSize(event.target.value)}>
        <option value="custom">Custom</option>
        {gasketFoamPresets.map(preset => <option key={preset.id} value={preset.id}>{preset.id} · {preset.length} × {preset.width} × {preset.thickness} mm</option>)}
      </select></label>
      <label className="wb-mech-check"><input type="checkbox" checked={layout.autoSize ?? true} onChange={event => update({ gasketLayout: { ...layout, autoSize: event.target.checked } })} /><span>Fit cut lengths automatically on four sides</span></label>
      <p className="wb-mech-hint">Uses 80–10 mm cuts in 10 mm steps, then 5 mm where needed. Manually edited gaskets stay fixed.</p>
      {layout.autoSize === false && <NumberField label="Default cut length" value={layout.length} min={5} step={5} onCommit={length => update({ gasketLayout: { ...layout, length } })} />}
      <div className="wb-mech-numbers">
        {(['width', 'thickness'] as const).map(key => <NumberField key={key} label={`Gasket ${key}`} value={layout[key]} min={0.1} onCommit={value => update({ gasketLayout: { ...layout, [key]: value, presetId: undefined } })} />)}
        <NumberField label="Compression" unit="%" value={layout.compression * 100} min={0} max={49.9} onCommit={value => update({ gasketLayout: { ...layout, compression: value / 100 } })} />
        <NumberField label="Gasket travel" value={configuration.gasketTravel ?? 0.1} min={0} onCommit={gasketTravel => update({ gasketTravel })} />
        <NumberField label="Supports per region" unit="" value={automaticCount ? resolvedCount : settings.supportCount} min={4} max={64} step={1} onCommit={value => { if (Number.isInteger(value)) updateSettings({ supportCount: value, autoCount: false }); }} />
      </div>
      <button type="button" className="wb-mech-quiet" onClick={() => update({ gasketLayout: { ...layout, supports: [] } })}>Reset gasket placement</button>
      <p className="wb-mech-hint">Reset releases manually positioned supports. Closure positions stay fixed.</p>
    </InspectorSection>
    <InspectorSection title="Advanced gasket clearances" detail="mm" defaultOpen={false}>
      <div className="wb-mech-numbers">
        <NumberField label="PCB-to-support clearance" value={settings.supportClearance ?? configuration.clearance} min={0} onCommit={supportClearance => updateSettings({ supportClearance })} />
        <NumberField label="Minimum wall behind gasket pockets" value={settings.minimumWall ?? 2} min={0.1} onCommit={minimumWall => updateSettings({ minimumWall })} />
        <NumberField label="Gasket fit tolerance" value={settings.tolerance} min={0} onCommit={tolerance => updateSettings({ tolerance })} />
        <NumberField label="Adhesive thickness" value={layout.adhesiveThickness ?? 0} onCommit={adhesiveThickness => update({ gasketLayout: { ...layout, adhesiveThickness } })} />
      </div>
      <p className="wb-mech-hint">PCB-to-support clearance is the horizontal gap at the board edge. The minimum wall protects material behind each pocket; the case grows to fit.</p>
      <label className="wb-mech-field"><span>Gasket material</span><input type="text" className="wb-mech-text" value={layout.material ?? ''} placeholder="Material or grade, if known" onChange={event => update({ gasketLayout: { ...layout, material: event.target.value || undefined } })} /></label>
    </InspectorSection>
    </>}
    {section !== 'gaskets' && <><InspectorSection title="Closure hardware" detail={hardware.thread} defaultOpen>
      <p className="wb-mech-hint">Custom screw and insert dimensions. Review these against your hardware; the starting M2 dimensions are not a supplier preset.</p>
      <label className="wb-mech-field"><span>Insert size</span><select aria-label="Insert size" value={insertSizes.find(size => size.threadDiameter === hardware.threadDiameter && size.insertLength === hardware.insertLength && size.insertDiameter === hardware.insertDiameter)?.id ?? 'custom'} onChange={event => updateHardware(resizeInsert(hardware, event.target.value))}>
        <option value="custom">Custom dimensions</option>{insertSizes.map(size => <option key={size.id} value={size.id}>M{size.threadDiameter} × L{size.insertLength} × OD{size.insertDiameter}</option>)}
      </select></label>
      <label className="wb-mech-field"><span>Screw drive</span><select value={hardware.drive} onChange={event => updateHardware({ drive: event.target.value as InternalClosureHardware['drive'] })}><option value="hex">Hex socket</option><option value="torx">Torx</option></select></label>
      <label className="wb-mech-field"><span>Insert installation</span><select value={hardware.installation} onChange={event => updateHardware({ installation: event.target.value as InternalClosureHardware['installation'] })}><option value="heat-set">Heat-set · printed top</option><option value="tapped">Tapped · machined top</option></select></label>
      <ScrewLengths value={hardware.screwLengths} onChange={screwLengths => updateHardware({ screwLengths, fixedLength: undefined })} />
      <label className="wb-mech-field"><span>Screw length</span><select value={hardware.fixedLength ?? 'auto'} onChange={event => updateHardware({ fixedLength: event.target.value === 'auto' ? undefined : Number(event.target.value) })}><option value="auto">Automatic from available lengths</option>{hardware.screwLengths.map(length => <option value={length} key={length}>{length} mm · fixed</option>)}</select></label>
    </InspectorSection>
    <InspectorSection title="Advanced closure dimensions" detail="Custom · mm" defaultOpen={false}>
      <label className="wb-mech-field"><span>Thread designation</span><input type="text" className="wb-mech-text" value={hardware.thread} onChange={event => updateHardware({ thread: event.target.value })} /></label>
      <label className="wb-mech-field"><span>Screw head profile</span><select value={hardware.headProfile} onChange={event => updateHardware({ headProfile: event.target.value as InternalClosureHardware['headProfile'] })}><option value="flat">Flat bearing surface</option><option value="countersunk">Countersunk</option></select></label>
      <label className="wb-mech-field"><span>Screw length measured from</span><select value={hardware.lengthDatum} onChange={event => updateHardware({ lengthDatum: event.target.value as InternalClosureHardware['lengthDatum'] })}><option value="under-head">Under the head</option><option value="overall">Top of head · overall</option><option value="unresolved">Unknown</option></select></label>
      <div className="wb-mech-numbers">{closureDimensions.map(([key, label]) => <NumberField key={key} label={label} value={hardware[key]} min={0} onCommit={value => updateHardware({ [key]: value })} />)}</div>
      <p className="wb-mech-hint">Thread changes do not resize the insert. Countersunk lengths include the head. Incompatible dimensions appear in Mechanical diagnostics.</p>
    </InspectorSection>
    </>}
    {assembly?.generatedHardware?.length ? <InspectorSection title="Resolved hardware and pads" detail="Purchase / cut list">
      <ul>{assembly.generatedHardware.map(item => <li key={item.id}>{item.quantity} × {item.designation} · {item.thread} · {item.length} mm</li>)}
        {assembly.generatedMaterials?.map(item => <li key={item.id}>{item.quantity} × {item.presetId ?? 'Custom'} pad · {item.size.x} × {item.size.y} × {item.size.z} mm free thickness</li>)}
      </ul>
    </InspectorSection> : null}
  </>;
}

function HardwareAndFits({ configuration, assembly, onChange }: {
  configuration: MechanicalConfiguration;
  assembly?: MechanicalAssembly;
  onChange: (patch: Partial<MechanicalConfiguration>) => void;
}) {
  const hardware = configuration.hardware ?? [];
  const criticalFits = configuration.criticalFits ?? [];
  const bodyParts = assembly?.case.bodies.map((entry) => entry.body) ?? [];
  const plannedBodyIds = [
    'plate',
    ...(configuration.plateFoamThickness > 0 ? ['plate-foam'] : []),
    ...(configuration.bottomFoamThickness > 0 ? ['bottom-foam'] : []),
    'bottom',
    ...(configuration.bottomStyle === 'sheet' && configuration.middleFrame ? ['middle-frame'] : []),
  ];
  const bodyIds = [...new Set(assembly ? bodyParts.map((body) => body.id) : plannedBodyIds)];
  const mountChoices = bodyParts.flatMap((body) => (body.mounts ?? []).map((mount) => ({ partId: body.id, mount })));
  const configuredMountChoices = [
    ...(configuration.mount === 'gasket' ? [] : configuration.mounts.map((mount) => ({ partId: configuration.mount === 'rigid' ? 'plate' : 'bottom', mount }))),
    ...(configuration.closureMounts ?? []).map((mount) => ({ partId: 'bottom', mount })),
  ];
  const availableMounts = mountChoices.length ? mountChoices : configuredMountChoices;
  const bodyName = (id: string) => bodyParts.find((body) => body.id === id)?.name ?? id;
  const updateHardware = (index: number, patch: Partial<HardwareSpec>) => onChange({
    hardware: hardware.map((entry, i) => i === index ? { ...entry, ...patch } : entry),
  });
  const updateFit = (index: number, patch: Partial<CriticalFit>) => onChange({
    criticalFits: criticalFits.map((entry, i) => i === index ? { ...entry, ...patch } : entry),
  });

  return <InspectorSection title="Hardware & critical fits" detail={`${hardware.length} hardware · ${criticalFits.length} fits`} defaultOpen={hardware.length + criticalFits.length > 0}>
    <div className="wb-mech-spec-group">
      <header><strong>Hardware</strong><button type="button" className="wb-mech-quiet" disabled={!availableMounts.length} onClick={() => {
        const choice = availableMounts[0];
        if (!choice) return;
        onChange({ hardware: [...hardware, { id: crypto.randomUUID(), partId: choice.partId, featureId: choice.mount.id, designation: 'Socket screw', thread: 'M2 × 0.4', length: 6, quantity: 4, notes: '' }] });
      }}>Add hardware</button></header>
      {!availableMounts.length && <p className="wb-mech-hint">Add a suspension or closure mount to link a fastener specification to its generated feature.</p>}
      {hardware.map((item, index) => {
        const link = JSON.stringify([item.partId, item.featureId]);
        return <article className="wb-mech-spec-card" key={item.id}>
          <header><strong>{item.designation || 'Hardware item'}</strong><button type="button" className="wb-mech-quiet" onClick={() => onChange({ hardware: hardware.filter((_, i) => i !== index) })}>Remove</button></header>
          <label className="wb-mech-field"><span>Mount link</span><select aria-label={`Mount link for hardware ${index + 1}`} value={availableMounts.some((choice) => choice.partId === item.partId && choice.mount.id === item.featureId) ? link : ''} onChange={(event) => {
            if (!event.target.value) return;
            const [partId, featureId] = JSON.parse(event.target.value) as [string, string];
            updateHardware(index, { partId, featureId });
          }}><option value="">Choose generated mount…</option>{availableMounts.map((choice) => <option key={`${choice.partId}:${choice.mount.id}`} value={JSON.stringify([choice.partId, choice.mount.id])}>{bodyName(choice.partId)} · {choice.mount.kind} · {choice.mount.at.x.toFixed(1)}, {choice.mount.at.y.toFixed(1)} mm</option>)}</select></label>
          <div className="wb-mech-spec-fields"><label className="wb-mech-field"><span>Designation</span><input aria-label={`Designation for hardware ${index + 1}`} value={item.designation} onChange={(event) => updateHardware(index, { designation: event.target.value })} placeholder="Socket screw" /></label><label className="wb-mech-field"><span>Thread</span><input aria-label={`Thread for hardware ${index + 1}`} value={item.thread} onChange={(event) => updateHardware(index, { thread: event.target.value })} placeholder="M2 × 0.4" /></label><NumberField label="Length" value={item.length} min={0.1} onCommit={(length) => updateHardware(index, { length })} /><NumberField label="Quantity" value={item.quantity} min={1} step={1} unit="pcs" onCommit={(quantity) => updateHardware(index, { quantity: Math.round(quantity) })} /><label className="wb-mech-field"><span>Tolerance</span><input aria-label={`Tolerance for hardware ${index + 1}`} value={item.tolerance ?? ''} onChange={(event) => updateHardware(index, { tolerance: event.target.value })} placeholder="Optional callout" /></label></div>
          <label className="wb-mech-field"><span>Notes</span><textarea aria-label={`Notes for hardware ${index + 1}`} value={item.notes ?? ''} onChange={(event) => updateHardware(index, { notes: event.target.value })} rows={2} placeholder="Drive, head, washer, or assembly notes" /></label>
        </article>;
      })}
    </div>
    <div className="wb-mech-spec-group">
      <header><strong>Critical fits</strong><button type="button" className="wb-mech-quiet" onClick={() => onChange({ criticalFits: [...criticalFits, { id: crypto.randomUUID(), partId: bodyIds[0] ?? 'plate', label: 'Critical dimension', from: { x: 0, y: 0 }, to: { x: 10, y: 0 }, tolerance: '±0.2 mm' }] })}>Add fit dimension</button></header>
      <p className="wb-mech-hint">Endpoints are document XY coordinates. Length is calculated from the saved endpoints.</p>
      {criticalFits.map((fit, index) => <article className="wb-mech-spec-card" key={fit.id}>
        <header><strong>{fit.label || 'Fit dimension'}</strong><button type="button" className="wb-mech-quiet" onClick={() => onChange({ criticalFits: criticalFits.filter((_, i) => i !== index) })}>Remove</button></header>
        <div className="wb-mech-spec-fields"><label className="wb-mech-field"><span>Generated part</span><select aria-label={`Part for critical fit ${index + 1}`} value={fit.partId} onChange={(event) => updateFit(index, { partId: event.target.value })}>{bodyIds.map((id) => <option key={id} value={id}>{bodyName(id)}</option>)}</select></label><label className="wb-mech-field"><span>Dimension name</span><input aria-label={`Label for critical fit ${index + 1}`} value={fit.label} onChange={(event) => updateFit(index, { label: event.target.value })} placeholder="Case width" /></label><label className="wb-mech-field"><span>Tolerance</span><input aria-label={`Tolerance for critical fit ${index + 1}`} value={fit.tolerance} onChange={(event) => updateFit(index, { tolerance: event.target.value })} placeholder="±0.2 mm" /></label></div>
        <div className="wb-mech-fit-points"><NumberField label="From X" value={fit.from.x} min={-1000000} onCommit={(x) => updateFit(index, { from: { ...fit.from, x } })} /><NumberField label="From Y" value={fit.from.y} min={-1000000} onCommit={(y) => updateFit(index, { from: { ...fit.from, y } })} /><NumberField label="To X" value={fit.to.x} min={-1000000} onCommit={(x) => updateFit(index, { to: { ...fit.to, x } })} /><NumberField label="To Y" value={fit.to.y} min={-1000000} onCommit={(y) => updateFit(index, { to: { ...fit.to, y } })} /></div>
        <output className="wb-mech-fit-result">Measured {Math.hypot(fit.to.x - fit.from.x, fit.to.y - fit.from.y).toFixed(2)} mm · {fit.tolerance}</output>
      </article>)}
    </div>
  </InspectorSection>;
}

export function MechanicalAssemblyPanel({ readiness: suppliedReadiness, diagnosticsRequest = 0, onDiagnosticsShown, generationTarget, onRevealDiagnostics, document, boardId, projectSession, instanceId, definitions, configuration, assembly, onChange: commitConfiguration, onResolve, onCancel, generation, livePreview, onLivePreviewChange, onExport, onShowFinding, selectedLayer, onSelectLayer, onMechanicalProfile, onExtractMechanicalProfile, onEditParts }: Props) {
  const draftScope = `${document.id}:${boardId ?? configuration?.boardId ?? ''}:${projectSession ?? 0}:${instanceId ?? ''}`;
  const currentDraftScope = React.useRef(draftScope);
  currentDraftScope.current = draftScope;
  const draft = React.useRef(new MechanicalDraft(configuration, draftScope, document.revision));
  draft.current.receive(configuration, draftScope, document.revision);
  const [, redraw] = React.useReducer((value: number) => value + 1, 0);
  const config = draft.current.value;
  const profileController = React.useRef(new MechanicalProfileController());
  profileController.current.receive(config, draftScope);
  React.useEffect(() => () => profileController.current.invalidate(), [draftScope]);
  const onChange = (next: MechanicalConfiguration | null) => {
    draft.current.submit(next); redraw(); commitConfiguration(next);
  };
  const [profileSource, setProfileSource] = React.useState<MechanicalBuiltinProfile | ''>(() => {
    const family = (configuration && profileSwitchFamily(configuration))
      ?? initialSwitchFamily(document, boardId ?? configuration?.boardId ?? '');
    return family === 'choc-v1' ? 'choc-v1-switch' : family === 'choc-v2' ? 'choc-v2-switch' : family === 'mx' ? 'mx-switch' : '';
  });
  const [profilePending, setProfilePending] = React.useState(false);
  const [profileError, setProfileError] = React.useState('');
  React.useEffect(() => { setProfilePending(false); setProfileError(''); }, [draftScope]);
  const diagnosticsRef = React.useRef<HTMLDivElement>(null);
  React.useEffect(() => {
    const family = initialSwitchFamily(document, boardId ?? configuration?.boardId ?? '');
    setProfileSource(family === 'choc-v1' ? 'choc-v1-switch' : family === 'choc-v2' ? 'choc-v2-switch' : family === 'mx' ? 'mx-switch' : '');
  }, [boardId, configuration?.boardId, document.id]);
  const usedDefinitions = [...new Set(document.parts.filter(part => document.boards.find(board => board.id === (boardId ?? configuration?.boardId))?.partIds.includes(part.id)).map((part) => part.definitionId))].map((id) => definitions.find((definition) => definition.id === id)).filter((definition): definition is PartDefinition => Boolean(definition));
  const processTargets = [...layerProcessIds.map((partId) => ({ partId, name: partId.replace(/-/g, ' ') })), ...document.parts.map((part) => ({ partId: part.id, name: `${part.reference} · ${definitions.find((definition) => definition.id === part.definitionId)?.name ?? part.definitionId}` }))];
  const update = (patch: Partial<MechanicalConfiguration>) => {
    const current = draft.current.value ?? createMechanicalConfiguration(document, boardId);
    const family = profileSwitchFamily({ ...current, ...patch })
      ?? initialSwitchFamily(document, current.boardId);
    const previousGap = family ? plateToPcbGap(family, current.plateThickness) : current.plateToPcb;
    const next = { ...current, ...patch };
    if (patch.plateThickness !== undefined && family) {
      next.plateToPcb = plateToPcbGap(family, patch.plateThickness);
      next.profiles = next.profiles.map((profile) => profile.switchFamily === family
        ? { ...profile, plateToPcb: next.plateToPcb }
        : profile);
      if (Math.abs(current.plateFoamThickness - defaultPlateFoamThickness(previousGap)) < 0.001) {
        next.plateFoamThickness = defaultPlateFoamThickness(next.plateToPcb);
      }
    }
    const thicknessFor = (partId: string) => partId === 'plate' ? next.plateThickness
      : partId === 'plate-foam' ? next.plateFoamThickness
        : partId === 'bottom-foam' ? next.bottomFoamThickness
          : partId === 'bottom' ? next.bottomThickness : undefined;
    const priorProcesses = patch.partProcesses ?? current.partProcesses ?? [];
    const partProcesses = priorProcesses.map((process) => {
      const isLayer = layerProcessIds.includes(process.partId as (typeof layerProcessIds)[number]);
      const foam = process.partId.endsWith('foam');
      const method = foam ? 'cut-sheet' : process.partId === 'plate' || (patch.method && isLayer)
        ? next.method : process.method ?? next.method;
      const priorMethod = process.method ?? next.method;
      const priorMaterial = process.material ?? '';
      const methodChanged = method !== priorMethod || (patch.method !== undefined && isLayer && !foam);
      const material = methodChanged || !materialOptionsForProcess(process.partId, method).includes(priorMaterial)
        ? materialForProcess(process.partId, method) : priorMaterial;
      const thickness = thicknessFor(process.partId);
      return {
        ...process,
        ...(isLayer ? { method, material } : {}),
        ...(thickness !== undefined ? { thickness } : {}),
      };
    });
    onChange({ ...next, partProcesses });
  };
  const profileFeedback = { setPending: setProfilePending, setError: setProfileError, update };
  const assignProfile = async (definitionId: string) => {
    if (!onMechanicalProfile || !profileSource) return;
    await profileController.current.assign(definitionId, profileSource, onMechanicalProfile, profileFeedback);
  };
  const selectProfileFamily = (definitionId: string, family: MechanicalSwitchFamily) => {
    if (!config) return;
    const plateThickness = defaultPlateThickness(family);
    const plateToPcb = plateToPcbGap(family, plateThickness);
    const profiles = config.profiles.map((profile) => {
      if (profile.definitionId === definitionId) {
        return { ...profile, switchFamily: family, plateToPcb };
      }
      return profile.switchFamily
        ? { ...profile, plateToPcb: switchMountingDatum(profile.switchFamily) - plateThickness }
        : profile;
    });
    update({
      plateThickness,
      plateToPcb,
      plateFoamThickness: defaultPlateFoamThickness(plateToPcb),
      profiles,
    });
  };
  const unknownDefinitions = usedDefinitions.filter(definition => definition.kind === 'switch' && !definition.mechanicalProfile && !inferSwitchFamily(definition) && !config?.profiles.some(profile => profile.definitionId === definition.id));
  const candidates = config ? stabilizerCandidates(document, config.boardId) : [];
  const initializedScopes = React.useRef(new Set<string>());
  React.useEffect(() => {
    if (!config || config.closureMounts !== undefined || config.mount === 'gasket' || !assembly?.suggestedMounts.length || initializedScopes.current.has(draftScope)) return;
    initializedScopes.current.add(draftScope);
    onChange(config);
  }, [draftScope, config?.closureMounts, config?.mount, assembly?.revision]);
  const initializingMounts = configuration?.closureMounts === undefined && configuration?.mount !== 'gasket' && Boolean(assembly?.suggestedMounts.length);
  const closureScrews = () => (assembly?.suggestedMounts ?? []).map(mount => ({ ...mount, id: `auto-closure/${mount.id}`, kind: 'boss' as const, holeDiameter: 2.2, height: config!.plateToPcb + config!.pcbThickness + Math.max(config!.bottomFoamThickness, config!.batteryHeight) }));
  const layers = assembly?.stack ?? [];
  const findings = mechanicalFindings(assembly, document);
  const readiness = suppliedReadiness ?? caseReadiness({ revision: document.revision,
    sceneRevision: document.revision, previewRevision: generation?.revision,
    boardId: boardId ?? config?.boardId, configuredBoardId: config?.boardId, generation, assembly });
  const revealDiagnostics = () => {
    onRevealDiagnostics?.();
    requestAnimationFrame(() => {
      const details = diagnosticsRef.current?.querySelector('details');
      if (!details) return;
      details.open = true;
      details.querySelector('summary')?.focus({ preventScroll: true });
      details.scrollIntoView({ block: 'nearest' });
      onDiagnosticsShown?.();
    });
  };
  React.useEffect(() => { if (diagnosticsRequest) revealDiagnostics(); }, [diagnosticsRequest]);
  const profileSpacing = assembly?.stack.find((layer) => layer.id === 'plate')?.z
    ?? (config && (profileSwitchFamily(config) ?? initialSwitchFamily(document, config.boardId))
      ? plateToPcbGap(profileSwitchFamily(config) ?? initialSwitchFamily(document, config.boardId)!, config.plateThickness)
      : undefined);

  const selectedSupport = assembly?.gasketSupports.find(support => selectedLayer === `gasket:${support.id}:lower` || selectedLayer === `gasket:${support.id}:upper`);
  if (config && selectedLayer) {
    const body = assembly?.case.bodies.find(entry => entry.body.id === selectedLayer)?.body;
    const title = selectedSupport ? `Gasket ${(assembly?.gasketSupports.indexOf(selectedSupport) ?? 0)+1}` : selectedLayer === 'gaskets' ? 'Gaskets'
      : selectedLayer === 'pcb' ? 'PCB' : selectedLayer === 'retainer' ? 'Top case' : selectedLayer === 'bottom' ? 'Bottom case' : body?.name.replace(/^./,c => c.toUpperCase()) ?? selectedLayer.replace(/-/g,' ').replace(/^./,c => c.toUpperCase());
    const fields = numericFields.filter(([key]) => selectedLayer === 'plate' ? key === 'plateThickness'
      : selectedLayer === 'pcb' ? key === 'pcbThickness' : selectedLayer === 'plate-foam' ? key === 'plateFoamThickness'
        : selectedLayer === 'bottom-foam' ? key === 'bottomFoamThickness' : selectedLayer === 'bottom' ? ['bottomThickness','wallThickness','clearance'].includes(key) : selectedLayer === 'retainer' ? ['wallThickness','clearance'].includes(key) : false);
    const layout = config.gasketLayout ?? defaultGasketLayout();
    const resize = (patch: { length?: number; width?: number }) => {
      if (!selectedSupport) return;
      const supports = assembly?.gasketSupports.filter(s => s.id === selectedSupport.id || (!selectedSupport.unlinked && selectedSupport.pairId === s.id && !s.unlinked)) ?? [];
      const ids = new Set(supports.map(s => s.id));
      update({ gasketLayout: { ...layout, supports: [...layout.supports.filter(s => !ids.has(s.id)), ...supports.map(s => ({
        ...layout.supports.find(saved => saved.id === s.id), id:s.id, regionId:s.regionId, outlineKey:s.outlineKey, anchor:s.anchor,
        length:s.length, width:s.width, unlinked:s.unlinked, placement:'user' as const, ...patch,
      }))] } });
    };
    return <div className="wb-mechanical-panel">
      <button type="button" className="wb-mech-quiet" onClick={() => onSelectLayer?.('')}>Assembly settings</button>
      <div className="wb-inspect-head"><h2>{title}</h2></div>
      {configuration && <CaseGenerationControls livePreview={livePreview} onLivePreviewChange={onLivePreviewChange} target={generationTarget} generation={generation} readiness={initializingMounts ? { ...readiness, canExport: false, message: 'Saving mounting defaults…' } : readiness} onGenerate={initializingMounts ? undefined : onResolve} onCancel={onCancel} onExport={onExport} />}
      {selectedSupport ? <>
        <p className="wb-mech-hint">Matching upper and lower pads. Drag this gasket directly to another side in Edit gaskets.</p>
        {selectedSupport.fitError ? <p role="alert" className="wb-mech-error">{selectedSupport.fitError} Preview and export stay blocked until it fits.</p> : <p className="wb-mech-hint">Fits at this position.</p>}
        <div className="wb-mech-numbers">
          <NumberField label="Cut length" value={selectedSupport.length} min={5} step={5} onCommit={length => resize({ length })} />
          <NumberField label="Pad width" value={selectedSupport.width} min={0.5} step={0.5} onCommit={width => resize({ width })} />
        </div>
        <p className="wb-mech-hint">Prefer 10 mm cuts; use 5 mm increments for a tighter fit. Foam thickness and compression are shared by the floating stack.</p>
        <button type="button" className="wb-mech-quiet" onClick={() => onSelectLayer?.('gaskets')}>All gasket settings</button>
      </> : selectedLayer === 'gaskets' ? <GasketControls configuration={config} assembly={assembly} onChange={update} section="gaskets" /> : <>
        <div className="wb-mech-numbers">{fields.map(([key,label]) => <NumberField key={key} label={label} value={config[key]} onCommit={value => update({[key]:value})} />)}</div>
        {body && <p className="wb-mech-hint">Resolved thickness {body.thickness.toFixed(2)} mm.</p>}
        {(selectedLayer === 'retainer' || selectedLayer === 'bottom') && config.internalGasket && <button type="button" className="wb-mech-quiet" onClick={() => onSelectLayer?.('')}>Edit shared closure hardware</button>}
      </>}
      {findings.some(finding => finding.severity === 'error') && <InspectorSection title="Fit issues" defaultOpen><FindingList document={document} assembly={assembly} findings={findings.filter(f => f.severity === 'error')} onShow={onShowFinding ?? (() => {})} /></InspectorSection>}
    </div>;
  }

  return <div className="wb-mechanical-panel">
    {!config && <div className="wb-inspect-head"><h2>Case construction</h2></div>}
    {!config ? <div className="wb-mech-start"><p>Resolve the keyboard stack from assigned part profiles, plate settings, and the case outline.</p><button className="wb-primary" disabled={!document.boards.length} onClick={() => onChange(createMechanicalConfiguration(document, boardId))}>Configure mechanical stack</button></div> : <>
      {configuration && <CaseGenerationControls livePreview={livePreview} onLivePreviewChange={onLivePreviewChange} target={generationTarget} generation={generation} readiness={initializingMounts ? { ...readiness, canExport: false, message: 'Saving mounting defaults…' } : readiness} onGenerate={initializingMounts ? undefined : onResolve} onCancel={onCancel} onExport={onExport}>
        <span className="wb-mech-revision">{assembly ? `Configuration resolved · r${assembly.revision}` : 'Configuration resolving'}</span>
      </CaseGenerationControls>}
      <div ref={diagnosticsRef}><InspectorSection title="Mechanical diagnostics" detail={`${findings.length}`} defaultOpen={findings.some(finding => finding.severity === 'error')}>
        <FindingList document={document} assembly={assembly} findings={findings} onShow={onShowFinding ?? (() => {})} />
      </InspectorSection></div>
      <InspectorSection title="Construction" detail={manufacturingMethods[config.method]} defaultOpen>
        <label className="wb-mech-field"><span>Method</span><select value={config.method} onChange={(event) => update({ method: event.target.value as MechanicalConfiguration['method'] })}>{Object.entries(manufacturingMethods).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
        <label className="wb-mech-field"><span>Mount style</span><select value={config.mount} onChange={(event) => update({ mount: event.target.value as MechanicalConfiguration['mount'], ...(event.target.value === 'gasket' ? { closureMounts: config.closureMounts?.filter(mount => !mount.id.startsWith('auto-closure/')), gasketLayout: config.gasketLayout ?? defaultGasketLayout(), gasketTravel: config.internalGasket ? config.gasketTravel : 0.1, internalGasket: config.internalGasket ?? defaultInternalGasket(), integratedPlateFrame: false, bottomStyle: 'shell', middleFrame: false } : {}) })}><option value="tray">Tray</option><option value="rigid">Rigid mount</option><option value="gasket">Gasket mount</option></select></label>
        <label className="wb-mech-field"><span>Bottom construction</span><select disabled={config.mount === 'gasket'} value={config.bottomStyle ?? 'shell'} onChange={(event) => update({ bottomStyle: event.target.value as MechanicalConfiguration['bottomStyle'] })}><option value="shell">Tray shell</option><option value="sheet">Flat sheet</option></select></label>
        {config.bottomStyle === 'sheet' && <label className="wb-mech-check"><input type="checkbox" checked={config.middleFrame ?? false} onChange={(event) => update({ middleFrame: event.target.checked })} /><span>Add middle frame</span></label>}
        <label className="wb-mech-check"><input type="checkbox" disabled={config.mount === 'gasket'} checked={config.integratedPlateFrame} onChange={(event) => update({ integratedPlateFrame: event.target.checked })} /><span>Integrate plate frame into case</span></label>
      </InspectorSection>
      {unknownDefinitions.length > 0 && <InspectorSection title="Missing part profiles" detail={`${usedDefinitions.length} part types`} defaultOpen={false}>
        <p className="wb-mech-hint">Fit profiles are defined with each part in Parts and inherited here by every layout and case.</p>
        <div className="wb-mech-inherited-profiles">{unknownDefinitions.map((definition) => {
          const profile = definition.mechanicalProfile ?? config.profiles.find(entry => entry.definitionId === definition.id);
          const family = profile?.switchFamily ?? inferSwitchFamily(definition);
          const description = profile ? profile.source : family ? `${family.toUpperCase()} fit · automatic` : definition.kind === 'switch' ? 'Choose a fit family in Parts' : 'Uses footprint clearance';
          const gap = family ? plateToPcbGap(family, config.plateThickness) : profile?.plateToPcb;
          return <div className="wb-mech-inherited-profile" key={definition.id}><strong>{definition.name}</strong><span>{description}{gap !== undefined && ` · ${gap.toFixed(2)} mm`}</span>{onEditParts && <button type="button" className="wb-mech-quiet" onClick={() => onEditParts(definition.id)}>Edit in Parts</button>}</div>;
        })}</div>
      </InspectorSection>}
      {unknownDefinitions.length > 0 && <InspectorSection title="Advanced source geometry" detail="Parts library" className="wb-mech-source-profile-editor" defaultOpen={false}>
        <p className="wb-mech-hint">Switch mounting height and plate thickness come from the selected fit drawing. Unknown switch families must be selected before plate generation.</p>
        {usedDefinitions.length === 0 ? <p className="wb-mech-hint">Place parts to assign mechanical profiles.</p> : <div className="wb-mech-profile-add"><label className="wb-mech-field"><span>Library fit profile</span><select aria-label="Library fit profile" value={profileSource} onChange={(event) => setProfileSource(event.target.value as typeof profileSource)}><option value="">Choose a switch family…</option><option value="mx-switch">MX switch</option><option value="choc-v1-switch">Choc v1 switch</option><option value="choc-v2-switch">Choc v2 switch</option><option value="mx-stab2u">MX stabilizer · 2u</option><option value="mx-stab625u">MX stabilizer · 6.25u</option></select><select aria-label="Assign library fit profile to" value="" disabled={profilePending || !onMechanicalProfile || !profileSource} onChange={(event) => void assignProfile(event.target.value)}><option value="">Choose a placed part type…</option>{usedDefinitions.filter((definition) => !config.profiles.some((profile) => profile.definitionId === definition.id)).map((definition) => <option key={definition.id} value={definition.id}>{definition.name}</option>)}</select></label><label className="wb-mech-field"><span>Custom KiCad geometry</span><select aria-label="Assign custom geometry profile to" value="" onChange={(event) => { const definitionId = event.target.value; if (!definitionId) return; const definition = usedDefinitions.find((entry) => entry.id === definitionId); if (!definition) return; const family = inferSwitchFamily(definition); update({ profiles: [...config.profiles, { definitionId, source: `KiCad ${definition.name}`, cutouts: [], plateToPcb: family ? plateToPcbGap(family, config.plateThickness) : config.plateToPcb, ...(family ? { switchFamily: family } : {}) }] }); }}><option value="">Choose imported part type…</option>{usedDefinitions.filter((definition) => Boolean(definition.kicadSource) && !config.profiles.some((profile) => profile.definitionId === definition.id)).map((definition) => <option key={definition.id} value={definition.id}>{definition.name}</option>)}</select></label></div>}
        {profilePending && <p className="wb-mech-hint" role="status">Loading library profile…</p>}
        {profileError && <p className="wb-mech-error" role="alert">{profileError}</p>}
        {config.profiles.map((profile) => <ProfileEditor key={`${draftScope}:${profile.definitionId}`} profile={profile} definitions={definitions} onSelectSwitchFamily={(family) => selectProfileFamily(profile.definitionId, family)} onExtract={onExtractMechanicalProfile ? (mappings) => onExtractMechanicalProfile(definitions.find((definition) => definition.id === profile.definitionId)?.kicadSource?.source ?? '', mappings) : undefined} onChange={(next) => { if (currentDraftScope.current === draftScope && draft.current.value === config) update({ profiles: config.profiles.map((entry) => entry.definitionId === next.definitionId ? next : entry) }); }} onRemove={() => update({ profiles: config.profiles.filter((entry) => entry.definitionId !== profile.definitionId) })} />)}
      </InspectorSection>}
      <InspectorSection title="Dimensions & clearances" detail="mm" defaultOpen={false}>
        <div className="wb-mech-numbers">{numericFields.map(([key, label]) => <NumberField key={key} label={label} value={config[key]} onCommit={(value) => update({ [key]: value })} />)}</div>
        <p className="wb-mech-hint">Plate underside to PCB top: {profileSpacing?.toFixed(2) ?? 'Choose a supported switch family to resolve'}{profileSpacing !== undefined && ' mm, derived from the switch mounting dimensions.'}</p>
        <NumberField label="Radial opening allowance" value={config.openingAllowance ?? 0} min={-1} max={1} onCommit={(openingAllowance) => update({ openingAllowance })} />
      </InspectorSection>
      <InspectorSection title="Resolved stack" detail={`${layers.length} layers`} defaultOpen={false}>
        {layers.length ? <div className="wb-mech-stack" aria-label="Resolved mechanical stack">{layers.map((layer) => <button type="button" className={`wb-mech-layer${selectedLayer === layer.id ? ' is-selected' : ''}`} aria-pressed={selectedLayer === layer.id} onClick={() => onSelectLayer?.(selectedLayer === layer.id ? '' : layer.id)} key={layer.id}>{layer.id === 'battery' ? <CaseIcon kind="battery"/> : <span className="wb-mech-layer-swatch" />}<strong>{layer.id === 'retainer' && config.internalGasket ? 'Top case' : layer.id === 'pcb' ? 'PCB' : layer.id.replace(/-/g, ' ').replace(/^./, letter => letter.toUpperCase())}</strong><span>{layer.thickness.toFixed(2)} mm</span><small>Z {layer.z.toFixed(2)}</small></button>)}</div> : <p className="wb-mech-hint">The stack appears after the current revision resolves.</p>}
      </InspectorSection>
      <section className="wb-mech-option-group" aria-label="Openings and battery"><h3>Openings & battery</h3>
      <InspectorSection title="Case openings" detail={`${config.openings?.length ?? 0}`}>
        <OpeningListEditor title="Document-coordinate access openings" openings={config.openings ?? []} onChange={(openings) => update({ openings })} />
      </InspectorSection>
      <InspectorSection title="Battery" detail={document.hardware?.transport === 'wireless' ? 'Wireless' : config.battery ? 'Configured' : 'Optional'}>
        {document.hardware?.transport === 'wireless' && <p className="wb-mech-hint">Included for wireless. Set your cell dimensions; the battery shares the space below the PCB with bottom foam.</p>}
        {document.hardware?.transport !== 'wireless' && <label className="wb-mech-check"><input type="checkbox" checked={Boolean(config.battery)} onChange={(event) => update({ battery: event.target.checked ? { size: { x: 30, y: 20, z: 6 }, at: { x: 0, y: 0 }, cableExit: { x: 0, y: 0 }, cableWidth: 2 } : undefined })} /><span>Include a battery envelope</span></label>}
        {config.battery && <div className="wb-mech-numbers wb-mech-battery">
          <NumberField label="Width" value={config.battery.size.x} onCommit={(x) => update({ battery: { ...config.battery!, size: { ...config.battery!.size, x } } })} min={0.1} />
          <NumberField label="Depth" value={config.battery.size.y} onCommit={(y) => update({ battery: { ...config.battery!, size: { ...config.battery!.size, y } } })} min={0.1} />
          <NumberField label="Height" value={config.battery.size.z} onCommit={(z) => update({ battery: { ...config.battery!, size: { ...config.battery!.size, z } } })} min={0.1} />
          <NumberField label="Cable width" value={config.battery.cableWidth ?? 2} onCommit={(cableWidth) => update({ battery: { ...config.battery!, cableWidth } })} min={0.1} />
          <NumberField label="Position X" value={config.battery.at.x} min={-1000000} onCommit={(x) => update({ battery: { ...config.battery!, at: { ...config.battery!.at, x } } })} />
          <NumberField label="Position Y" value={config.battery.at.y} min={-1000000} onCommit={(y) => update({ battery: { ...config.battery!, at: { ...config.battery!.at, y } } })} />
          <NumberField label="Cable exit X" value={config.battery.cableExit.x} min={-1000000} onCommit={(x) => update({ battery: { ...config.battery!, cableExit: { ...config.battery!.cableExit, x } } })} />
          <NumberField label="Cable exit Y" value={config.battery.cableExit.y} min={-1000000} onCommit={(y) => update({ battery: { ...config.battery!, cableExit: { ...config.battery!.cableExit, y } } })} />
        </div>}
      </InspectorSection>
      </section>
      <section className="wb-mech-option-group" aria-label="Mounting and hardware"><h3>Mounting & hardware</h3>
      {config.mount === 'gasket' && <button type="button" className="wb-mech-quiet" onClick={() => onSelectLayer?.('gaskets')}>Edit gasket supports</button>}
      {config.internalGasket && <GasketControls configuration={config} assembly={assembly} onChange={update} section="hardware" />}
      {config.mount !== 'gasket' && <MountList title="Suspension mounts" value={config.mounts} onChange={(mounts) => update({ mounts })} />}
      <MountList title="Closure screws" value={config.closureMounts ?? []} onChange={(closureMounts) => update({ closureMounts })} allowAdd={!config.internalGasket} />
      <HardwareAndFits configuration={config} assembly={assembly} onChange={update} />
      <InspectorSection title="Suggested mount locations" detail={`${assembly?.suggestedMounts?.length ?? 0} candidates`}>
{config.internalGasket ? <>
        {assembly?.suggestedMounts?.length ? <><p className="wb-mech-hint">Candidates clear the current openings and battery envelope. Adopting them is explicit; later edits keep the chosen coordinates fixed.</p><button type="button" className="wb-mech-quiet" onClick={() => update(config.internalGasket ? { closureMounts: assembly.suggestedMounts } : { mounts: assembly.suggestedMounts })}>{config.internalGasket ? 'Adopt closure positions' : 'Adopt suggested mounts'}</button></> : <p className="wb-mech-hint">Resolve the current geometry to see clearance-tested mounting locations.</p>}
</> : <>
        {assembly?.suggestedMounts?.length ? <><p className="wb-mech-hint">Clearance-tested positions on this board. Adopt them as suspension mounts or closure screws.</p><div className="wb-mount-candidates">{assembly.suggestedMounts.map((mount, index) => <span key={mount.id}>{index + 1}: X {mount.at.x.toFixed(1)}, Y {mount.at.y.toFixed(1)} mm</span>)}</div><button type="button" className="wb-mech-quiet" onClick={() => update({ mounts: assembly.suggestedMounts })}>Adopt suggested mounts</button><button type="button" className="wb-mech-quiet" onClick={() => update({ closureMounts: [...(config.closureMounts ?? []), ...closureScrews()] })}>Add suggested closure screws</button></> : <p className="wb-mech-hint">No additional clear locations fit the current outline, openings and battery. Move obstructions or extend the outline to make room.</p>}
</>}
      </InspectorSection>
      </section>
      <section className="wb-mech-option-group" aria-label="Manufacturing overrides"><h3>Manufacturing overrides</h3>
      <InspectorSection title="Per-part process overrides" detail={`${config.partProcesses?.length ?? 0}`}>
        {processTargets.map(({ partId, name }) => {
          const process = config.partProcesses?.find((entry) => entry.partId === partId);
          const defaultThickness = partId === 'plate' ? config.plateThickness : partId === 'plate-foam' ? config.plateFoamThickness : partId === 'bottom-foam' ? config.bottomFoamThickness : partId === 'bottom' ? config.bottomThickness : config.plateThickness;
          const setProcess = (patch: Partial<NonNullable<MechanicalConfiguration['partProcesses']>[number]>) => {
            if (partId === 'plate' && patch.method) {
              update({ method: patch.method });
              return;
            }
            const base = process ?? defaultProcess(partId, config.method, defaultThickness);
            if (partId === 'plate' && patch.thickness !== undefined) {
              update({ plateThickness: patch.thickness });
              return;
            }
            if (partId === 'bottom' && patch.thickness !== undefined) {
              update({ bottomThickness: patch.thickness });
              return;
            }
            if (partId === 'plate-foam' && patch.thickness !== undefined) {
              update({ plateFoamThickness: patch.thickness });
              return;
            }
            if (partId === 'bottom-foam' && patch.thickness !== undefined) {
              update({ bottomFoamThickness: patch.thickness });
              return;
            }
            const baseMethod = base.method ?? config.method;
            const baseMaterial = base.material ?? '';
            const method = partId.endsWith('foam') ? 'cut-sheet' : patch.method ?? baseMethod;
            const methodChanged = patch.method !== undefined && patch.method !== baseMethod;
            const material = patch.material ?? (methodChanged || !materialOptionsForProcess(partId, method).includes(baseMaterial)
              ? materialForProcess(partId, method)
              : baseMaterial || materialForProcess(partId, method));
            const next = {
              ...base,
              ...patch,
              method,
              material,
              thickness: patch.thickness ?? base.thickness ?? defaultThickness,
              constraintsVersion: base.constraintsVersion || supportedConstraintVersion,
            };
            update({ partProcesses: [...(config.partProcesses ?? []).filter((entry) => entry.partId !== partId), next] });
          };
          return <div className="wb-mech-process" key={partId}><header><strong>{name}</strong>{process && <button type="button" className="wb-mech-quiet" onClick={() => update({ partProcesses: config.partProcesses?.filter((entry) => entry.partId !== partId) })}>Remove</button>}</header>
            {!process ? <button type="button" className="wb-mech-quiet" onClick={() => setProcess({})}>Add process override</button> : <>
              <div className="wb-mech-process-fields"><label className="wb-mech-field"><span>Method</span>{partId.endsWith('foam') ? <output>Cut sheet</output> : <select value={process.method ?? config.method} onChange={(event) => setProcess({ method: event.target.value as MechanicalConfiguration['method'] })}><option value="pcb-fr4">PCB FR-4</option><option value="printed">3D printed</option><option value="cnc">CNC machined</option><option value="cut-sheet">Cut sheet</option></select>}</label><label className="wb-mech-field"><span>Material</span><select value={process.material || materialForProcess(partId, process.method ?? config.method)} onChange={(event) => setProcess({ material: event.target.value })}>{materialOptionsForProcess(partId, process.method ?? config.method).map((material) => <option key={material} value={material}>{material}</option>)}</select></label><NumberField label="Finished thickness" value={process.thickness ?? defaultThickness} onCommit={(thickness) => setProcess({ thickness })} min={partId.endsWith('foam') ? 0 : 0.1}/></div>
              <div className="wb-mech-version">Constraint set <code>{process.constraintsVersion ?? supportedConstraintVersion}</code>{(process.constraintsVersion ?? supportedConstraintVersion) !== supportedConstraintVersion && <small role="alert">Unsupported constraint set. Diagnostics will block export.</small>}</div>
            </>}
          </div>;
        })}
        <p className="wb-mech-hint">Supported constraint set: {supportedConstraintVersion}.</p>
      </InspectorSection>
      <InspectorSection title="Stabilizer fit" detail={`${candidates.length} wide keys`}>
        {candidates.map(({ part, keyUnits, stabilizer }) => {
          const current = config.stabilizers?.find(entry => entry.partId === part.id) ?? stabilizer;
          return <div className="wb-mech-stabilizer" key={part.id}><strong>{part.reference} · {keyUnits}u</strong><label className="wb-mech-field"><span>Stabilizer</span><select aria-label={`Stabilizer for ${part.reference}`} value={current.kind} onChange={event => update({ stabilizers: [...(config.stabilizers ?? []).filter(entry => entry.partId !== part.id), { ...current, kind: event.target.value as MechanicalStabilizerOverride['kind'] }] })}><option value="pcb-mount">Cherry PCB mount · {stabilizer.units}u</option><option value="none">None</option>{current.profile && <option value="plate-mount">Custom plate mount</option>}</select></label><p className="wb-mech-hint">Size and orientation follow the key layout.</p></div>;
        })}
        {!candidates.length && <p className="wb-mech-hint">No keys in this layout need stabilizers.</p>}
      </InspectorSection>

      </section>
      <InspectorSection title="Configuration management">
        <button type="button" className="wb-mech-quiet" onClick={() => onChange(null)}>Disable mechanical stack</button>
      </InspectorSection>
    </>}
  </div>;
}
