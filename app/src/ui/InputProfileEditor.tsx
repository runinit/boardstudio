import type { InputProfile, PartDefinition } from '@boardstudio/v2-contracts';
import { DraftInput } from './InspectorControls';
import { InspectorSection } from './InspectorSection';

export function InputProfileEditor({ definition, onChange }: { definition: PartDefinition; onChange: (profile: InputProfile) => void }) {
  const legacy = definition.generator?.source === 'ceoloide/rotary_encoder_ec11_ec12';
  const profile: InputProfile = definition.inputProfile ?? {
    ...(definition.matrixTerminals ? { press: { ...definition.matrixTerminals, independent: true } } : {}),
    ...(legacy ? { press: { row: 'S1', column: 'S2', independent: true }, rotary: { a: 'A', b: 'C', common: 'B', steps: 80, triggersPerRotation: 20, driver: 'ec11' } } : {}),
  };
  const terminals = [...new Set([...Object.keys(definition.terminals ?? {}), ...definition.pads.filter(pad => pad.plated !== false).map(pad => pad.number)])];
  const terminal = (label: string, value: string, commit: (value: string) => void) => <label className="wb-script-select-label" key={label}>{label}<select aria-label={label} value={value} onChange={event => commit(event.target.value)}>{[...new Set([value, ...terminals])].map(id => <option key={id} value={id}>{id || 'Choose contact'}</option>)}</select></label>;
  const positive = (value: string) => { const parsed = Number(value); return value.trim() && Number.isInteger(parsed) && parsed > 0 && parsed <= 65535 ? parsed : undefined; };
  return <InspectorSection title="Input capabilities" defaultOpen={Boolean(profile.rotary)}>
    <label><input type="checkbox" aria-label="Independent press input" checked={Boolean(profile.press)} onChange={event => onChange({ ...profile, press: event.target.checked ? { row: terminals[0] ?? '', column: terminals[1] ?? '', independent: true } : undefined })} /> Press input</label>
    {profile.press && <>
      {terminal('Press row contact', profile.press.row, row => onChange({ ...profile, press: { ...profile.press!, row } }))}
      {terminal('Press column contact', profile.press.column, column => onChange({ ...profile, press: { ...profile.press!, column } }))}
      <label><input type="checkbox" aria-label="Press contacts are independent" checked={profile.press.independent} onChange={event => onChange({ ...profile, press: { ...profile.press!, independent: event.target.checked } })} /> Contacts independent of rotation</label>
    </>}
    <label><input type="checkbox" aria-label="Rotary input" checked={Boolean(profile.rotary)} onChange={event => onChange({ ...profile, rotary: event.target.checked ? { a: terminals[0] ?? '', b: terminals[1] ?? '', common: terminals[2] ?? '' } : undefined })} /> Rotary input</label>
    {profile.rotary && <>
      {(['a', 'b', 'common'] as const).map(role => terminal(`Rotary ${role === 'common' ? 'common' : role.toUpperCase()} contact`, profile.rotary![role], value => onChange({ ...profile, rotary: { ...profile.rotary!, [role]: value } })))}
      <label className="wb-script-select-label">Driver<select aria-label="Encoder driver" value={profile.rotary.driver ?? ''} onChange={event => onChange({ ...profile, rotary: { ...profile.rotary!, driver: event.target.value === 'ec11' ? 'ec11' : undefined } })}><option value="">Unverified</option><option value="ec11">EC11 quadrature</option></select></label>
      <label>Pulses per rotation<DraftInput ariaLabel="Encoder pulses per rotation" type="number" min="1" placeholder="Unknown" value={profile.rotary.steps ?? ''} onCommit={value => onChange({ ...profile, rotary: { ...profile.rotary!, steps: positive(value) } })} /></label>
      <label>Actions per rotation<DraftInput ariaLabel="Encoder actions per rotation" type="number" min="1" placeholder="Unknown" value={profile.rotary.triggersPerRotation ?? ''} onCommit={value => onChange({ ...profile, rotary: { ...profile.rotary!, triggersPerRotation: positive(value) } })} /></label>
      {(!profile.rotary.driver || !profile.rotary.steps || !profile.rotary.triggersPerRotation) && <p className="wb-empty-note">Confirm the selected hardware’s driver and rotation settings before exporting firmware.</p>}
    </>}
  </InspectorSection>;
}
