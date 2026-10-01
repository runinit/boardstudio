import type { HardwareGate, HardwareOutput, HardwareSource } from '../../../contracts/src/index';
import { InspectorSection } from './InspectorSection';

const outputs: [HardwareOutput, string][] = [['footprint', 'Footprint'], ['electrical', 'Electrical'], ['mechanical', 'Case'], ['model', '3D model'], ['firmware', 'Firmware']];
export function HardwareReadiness({ source, gates }: { source: HardwareSource; gates: HardwareGate[] }) {
  const url = `${source.repository.replace(/\/$/u, '')}/blob/${encodeURIComponent(source.revision)}/${source.path.split('/').map(encodeURIComponent).join('/')}`;
  return <InspectorSection title="Hardware readiness" detail={`${gates.length} reviews`} defaultOpen>
    <table className="wb-module-readiness"><tbody>{outputs.map(([output, label]) => {
      const pending = gates.filter(gate => gate.output === output);
      return <tr key={output}><th scope="row">{label}</th><td>{pending.length ? `${pending.length} to review` : 'No recorded blockers'}</td></tr>;
    })}</tbody></table>
    {gates.length > 0 && <details><summary>Review {gates.length} remaining {gates.length===1?'item':'items'}</summary><ul className="wb-module-gates">{gates.map((gate, index) => <li key={`${gate.output}/${gate.code}/${index}`}><strong>{outputs.find(([output]) => output === gate.output)?.[1]}</strong> · {gate.message}</li>)}</ul></details>}
    <a href={url} target="_blank" rel="noreferrer">Pinned source ↗</a>
    <p className="wb-empty-note">{source.license} · {source.revision.slice(0, 12)}{source.upstreamStatus ? ` · Upstream: ${source.upstreamStatus}` : ''}</p>
  </InspectorSection>;
}
