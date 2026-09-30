import { keyBindingChoices as choices } from './keyBindingChoices';
import './firmware-keymap-panel.css';

type FirmwareKeymapKey = { id: string; label: string };

type Props = {
  keys: FirmwareKeymapKey[];
  bindings: Record<string, string>;
  onChange: (keyId: string, binding: string) => void;
};

export function FirmwareKeymapPanel({ keys, bindings, onChange }: Props) {
  const assigned = keys.filter(key => bindings[key.id] && bindings[key.id] !== '&none').length;
  return <details className="firmware-keymap-panel">
    <summary><span>Firmware keymap</span><span className="firmware-keymap-summary">{assigned}/{keys.length} assigned</span></summary>
    <div className="firmware-keymap-body">
      <div className="firmware-keymap-grid">
        {keys.map(key => {
          const id = `firmware-key-${key.id}`;
          return <label className="firmware-keymap-row" htmlFor={id} key={key.id}>
            <span>{key.label}</span>
            <select id={id} aria-label={`Binding for ${key.label}`} value={bindings[key.id] ?? '&none'} onChange={event => onChange(key.id, event.target.value)}>
              {choices.map(([value, label]) => <option value={value} key={value}>{label}</option>)}
            </select>
          </label>;
        })}
      </div>
      <div className="firmware-keymap-preview" aria-label="Firmware keymap preview">
        {keys.map(key => <div className="firmware-keymap-preview-row" key={key.id}><span>{key.label}</span><code>{bindings[key.id] ?? '&none'}</code></div>)}
      </div>
    </div>
  </details>;
}
