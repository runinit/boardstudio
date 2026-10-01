import type { AssemblyDefinition, JsonValue, PartDefinition, ProjectDoc } from '@boardstudio/v2-contracts';
import type { Props as WorkbenchProps } from './workbenchTypes';
import type { createLibraryActions } from './createLibraryActions';
import { assemblyName, matrixPresetDefinitions, type MatrixPresetId } from './assemblyCatalog';
import { assemblyPreset, type SwitchOrientation } from './assemblyPresets';
import { GeneratorFields } from './GeneratorFields';
import { DraftInput, ModelVector, OrientationControl } from './InspectorControls';
import { InspectorSection } from './InspectorSection';
import { InputProfileEditor } from './InputProfileEditor';
import { matrixInputAvailable } from './inputCapabilities';
import { DefinitionKeycapControls } from './OutlineInspector';
import { partCatalogLabel } from './partsCatalog';
import { ArrowIcon } from './WorkbenchIcons';
import { courtyardSize, definitionIssues, formatSize, makeId } from './workbenchGeometry';

type Props = {
  document: ProjectDoc;
  libraryAssembly: MatrixPresetId | null;
  libraryDefinitions: PartDefinition[];
  assemblyOrientation: SwitchOrientation;
  setAssemblyOrientation: (orientation: SwitchOrientation) => void;
  setEditingAssembly: (assembly: AssemblyDefinition) => void;
  onPlaceAssembly: (preset: MatrixPresetId) => void;
  onPlaceDefinition: (definition: PartDefinition) => void;
  applyToKey: boolean;
  selectedLibraryDefinition?: PartDefinition;
  editDefinition?: PartDefinition;
  activeModelDefinition?: PartDefinition;
  onImportModel?: WorkbenchProps['onImportModel'];
  definitionError: string;
  actions: ReturnType<typeof createLibraryActions>;
  generator: {
    enabled: boolean;
    edits: Record<string, JsonValue>;
    error?: string;
    compilePending: boolean;
    compileError?: string;
    onChange: (parameter: string, value: JsonValue) => void;
    onSave: () => void;
  };
};

export function PartsInspectorPanel({ document, libraryAssembly, libraryDefinitions, assemblyOrientation, setAssemblyOrientation, setEditingAssembly, onPlaceAssembly, onPlaceDefinition, applyToKey, selectedLibraryDefinition, editDefinition, activeModelDefinition, onImportModel, definitionError, actions, generator }: Props) {
  const { attachModel, updateModel, updateDefinition, updateCourtyard, updatePad, commitPadField, commitPadNumber, addDefinitionPad, removeDefinitionPad } = actions;
  const { enabled: ergogenGenerator, edits: libraryParameters, compilePending: previewCompilePending, compileError: previewCompileError, onChange: updateGenerator, onSave: saveGenerator } = generator;
  return <>
    <section aria-label="Saved assemblies"><h2>Assemblies</h2><button onClick={() => setEditingAssembly({ id: makeId(), name: 'New assembly', members: [] })}>New assembly</button>{(document.assemblies ?? []).map(assembly => <div key={assembly.id}><button onClick={() => setEditingAssembly(assembly)}>{assembly.name}</button><button aria-label={`Duplicate ${assembly.name}`} onClick={() => setEditingAssembly({ ...structuredClone(assembly), id: makeId(), name: `${assembly.name} copy` })}>Duplicate</button></div>)}</section>
    <div className="wb-inspect-head"><h2>{libraryAssembly ? assemblyName(libraryAssembly) : (selectedLibraryDefinition ? partCatalogLabel(selectedLibraryDefinition) : 'Select a part')}</h2></div>
    {libraryAssembly && <section aria-label="Assembly settings"><p className="wb-empty-note">{matrixPresetDefinitions[libraryAssembly].led ? 'SK6812 MINI-E mounted underneath, shining through the PCB into the switch. The LED sits opposite the socket or solder pins.' : 'Switch footprint with an underside diode.'}</p><OrientationControl value={assemblyOrientation} onChange={setAssemblyOrientation} /><button className="wb-primary" onClick={() => onPlaceAssembly(libraryAssembly)}>Place key assembly</button><button className="wb-secondary" onClick={() => setEditingAssembly(assemblyPreset(libraryAssembly, libraryDefinitions, assemblyOrientation))}>Customize 3D assembly</button></section>}
    {!libraryAssembly && selectedLibraryDefinition && <section className="wb-library-preview" aria-label="Selected footprint settings">
      <p className="wb-inspector-description">{selectedLibraryDefinition.kind === 'switch' ? 'Switch footprint' : selectedLibraryDefinition.kind === 'custom' ? 'Custom component' : 'Component footprint'} · {formatSize(selectedLibraryDefinition.courtyard)}</p>
      <button className="wb-primary wb-place-part" disabled={Boolean(generator.error) || (applyToKey && !matrixInputAvailable(selectedLibraryDefinition))} onClick={() => onPlaceDefinition(selectedLibraryDefinition)}>{applyToKey ? 'Apply to selected key' : 'Place component'} <ArrowIcon /></button>
      {applyToKey && !matrixInputAvailable(selectedLibraryDefinition) && <p className="wb-empty-note">This footprint needs an independent press contact pair to replace a matrix key. Clear the key selection to place it as a standalone component.</p>}
      {ergogenGenerator && <fieldset className="wb-generator-settings">
        <legend>Part options</legend>
        {ergogenGenerator && <GeneratorFields definition={selectedLibraryDefinition} edits={libraryParameters} onChange={updateGenerator} onImportModel={onImportModel} error={generator.error} />}
        {generator.error && <p className="wb-generator-error" role="alert">{generator.error}</p>}
        {previewCompilePending && <p role="status">Compiling footprint preview…</p>}
        {previewCompileError && <p className="wb-generator-error" role="alert">{previewCompileError}</p>}
        <button className="wb-secondary" disabled={Boolean(generator.error || previewCompilePending || previewCompileError)} onClick={saveGenerator}>Apply generator settings</button>
      </fieldset>}
      {selectedLibraryDefinition.envelopeNotice && <p className="wb-empty-note">{selectedLibraryDefinition.envelopeNotice}</p>}
      <InputProfileEditor definition={selectedLibraryDefinition} onChange={inputProfile => updateDefinition({ inputProfile })} />
      {!ergogenGenerator && <InspectorSection title="Keycap & outline" defaultOpen={selectedLibraryDefinition.kind === 'switch'}>
        <DefinitionKeycapControls definition={selectedLibraryDefinition} onChange={(keycap) => updateDefinition({ keycap })} />
        {selectedLibraryDefinition.kind !== 'switch' && <p className="wb-empty-note">The component courtyard defines its outline contribution. Adjust the edge margin after placing it.</p>}
      </InspectorSection>}
    </section>}
    {!libraryAssembly && editDefinition && !editDefinition.generator && <InspectorSection title="Edit footprint" detail="Custom geometry" defaultOpen={editDefinition.pads.length === 0}>
    {editDefinition && <section className="wb-definition-editor" aria-label="Custom component definition editor">
      <label>Name<DraftInput ariaLabel="Definition name" value={editDefinition.name} onCommit={(value) => updateDefinition({ name: value })} /></label>
      <label>Kind<select aria-label="Definition kind" value={editDefinition.kind} onChange={(event) => updateDefinition({ kind: event.target.value as PartDefinition['kind'] })}>
        <option value="switch">Switch</option><option value="controller">Controller</option><option value="connector">Connector</option><option value="encoder">Encoder</option><option value="passive">Passive</option><option value="custom">Custom</option>
      </select></label>
      <div className="wb-definition-dimensions"><strong>Rectangular courtyard <small>mm</small></strong>
        <label>Width<DraftInput ariaLabel="Courtyard width" type="number" min="0.01" step="0.1" value={courtyardSize(editDefinition.courtyard).x} onCommit={(value) => updateCourtyard('x', value)} /></label>
        <label>Height<DraftInput ariaLabel="Courtyard height" type="number" min="0.01" step="0.1" value={courtyardSize(editDefinition.courtyard).y} onCommit={(value) => updateCourtyard('y', value)} /></label>
      </div>
      <div className="wb-definition-pad-heading"><strong>Pads <small>{editDefinition.pads.length}</small></strong><button type="button" disabled={Boolean(editDefinition.kicadSource)} onClick={addDefinitionPad}>+ Add pad</button></div>
      {editDefinition.kicadSource && <p className="wb-empty-note">Imported pad geometry stays linked to its original KiCad source. Edit the footprint envelope, reference, nets, and attached models here.</p>}
      {editDefinition.pads.map((pad, index) => <fieldset className="wb-definition-pad" key={index} disabled={Boolean(editDefinition.kicadSource)}>
        <legend>Pad {index + 1}</legend>
        <div className="wb-definition-pad-grid">
          <label>ID<DraftInput ariaLabel={`Pad ${index + 1} ID`} value={pad.id} onCommit={(value) => commitPadField(pad.id, 'id', value)} /></label>
          <label>Number<DraftInput ariaLabel={`Pad ${index + 1} number`} value={pad.number} onCommit={(value) => commitPadField(pad.id, 'number', value)} /></label>
          <label>X<DraftInput ariaLabel={`Pad ${index + 1} X`} type="number" step="0.1" value={pad.at.x} onCommit={(value) => commitPadNumber(pad.id, 'atX', value)} /></label>
          <label>Y<DraftInput ariaLabel={`Pad ${index + 1} Y`} type="number" step="0.1" value={pad.at.y} onCommit={(value) => commitPadNumber(pad.id, 'atY', value)} /></label>
          <label>Width<DraftInput ariaLabel={`Pad ${index + 1} width`} type="number" min="0.01" step="0.1" value={pad.size.x} onCommit={(value) => commitPadNumber(pad.id, 'sizeX', value)} /></label>
          <label>Height<DraftInput ariaLabel={`Pad ${index + 1} height`} type="number" min="0.01" step="0.1" value={pad.size.y} onCommit={(value) => commitPadNumber(pad.id, 'sizeY', value)} /></label>
          <label>Shape<select aria-label={`Pad ${index + 1} shape`} value={pad.shape} onChange={(event) => updatePad(pad.id, { shape: event.target.value as PartDefinition['pads'][number]['shape'] })}>
            <option value="circle">Circle</option><option value="oval">Oval</option><option value="rect">Rectangle</option><option value="roundrect">Rounded rectangle</option>
          </select></label>
          <label>Drill<DraftInput ariaLabel={`Pad ${index + 1} drill`} type="number" min="0.01" step="0.1" placeholder="None" value={pad.drill ?? ''} onCommit={(value) => commitPadNumber(pad.id, 'drill', value)} /></label>
        </div>
        <button className="wb-definition-remove-pad" type="button" onClick={() => removeDefinitionPad(pad.id)}>Remove pad</button>
      </fieldset>)}
      <ul className="wb-definition-validation" aria-live="polite">{definitionIssues(editDefinition).map((issue) => <li key={issue}>{issue}</li>)}</ul>
      {definitionError && <p className="wb-definition-error" role="alert">{definitionError}</p>}
      <p className="wb-model-bound">Definitions in use stay in the library; edits apply to every placed instance.</p>
    </section>}
    </InspectorSection>}
    {!libraryAssembly && !ergogenGenerator && onImportModel && <InspectorSection title="3D model" detail={activeModelDefinition?.models?.[0] ? "Attached" : "Optional"}>
      <section className="wb-model-import" aria-label="3D model binding">
      {activeModelDefinition?.models?.[0] ? <p className="wb-model-bound">Bound asset: {document.assets.find((asset) => asset.id === activeModelDefinition.models![0]?.assetId)?.name ?? activeModelDefinition.models![0].assetId}</p> : <p className="wb-model-bound">No model attached to this definition.</p>}
      <label className="wb-footprint-import">Attach STEP / STL / WRL model<input type="file" accept=".step,.stp,.stl,.wrl,model/step,model/vrml" disabled={!activeModelDefinition} onChange={(event) => {
        const file = event.currentTarget.files?.[0];
        if (file) attachModel(file);
        event.currentTarget.value = '';
      }} /></label>
      {activeModelDefinition?.models?.[0] && <div className="wb-model-transforms">
        <h4>Model alignment</h4>
        <ModelVector title="Offset" value={activeModelDefinition.models![0].offset} unit="mm" validation="finite" onCommit={(axis, value) => updateModel('offset', axis, value)} />
        <ModelVector title="Rotation" value={activeModelDefinition.models![0].rotation} unit="°" validation="finite" onCommit={(axis, value) => updateModel('rotation', axis, value)} />
        <ModelVector title="Scale" value={activeModelDefinition.models![0].scale} unit="×" validation="positive" onCommit={(axis, value) => updateModel('scale', axis, value)} />
      </div>}
    </section></InspectorSection>}
  </>;
}
