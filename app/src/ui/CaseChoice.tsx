import type { ReactNode } from 'react';

const paths: Record<string, ReactNode> = {
  printed: <><path d="M3 18V3h18v15M9 7h6l-3 4zM6 19h12M7 15h10v4"/></>,
  cnc: <><path d="M3 4h18M12 4v7m-2-3h4M4 20v-6h16v6zM10 14l2-3 2 3"/></>,
  'cut-sheet': <><path d="M3 8h18v12H3zM7 3l5 9 5-9M8 15h8"/></>,
  'pcb-fr4': <><rect x="3" y="4" width="18" height="16" rx="1"/><path d="M7 8h4v8m2-8h4v8M7 12h10"/></>,
  tray: <path d="M3 6v14h18V6M3 12h18M7 12v8m10-8v8"/>,
  rigid: <path d="M3 7h18M3 18h18M7 4v17m10-17v17M5 4h4m6 0h4"/>,
  gasket: <><path d="M3 5v15h18V5M6 12h12M5 8h3m8 0h3M5 16h3m8 0h3"/></>,
  shell: <path d="M3 6v14h18V6M6 6v11h12V6"/>,
  sheet: <path d="M3 8h18v4H3zM3 16h18v4H3z"/>,
  battery: <><rect x="3" y="6" width="17" height="12" rx="2"/><path d="M20 10h2v4h-2M11 8l-3 5h5l-3 3"/></>,
};
export function CaseIcon({ kind }: { kind: string }) {
  return <svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">{paths[kind]}</svg>;
}
export function CaseChoice({ label, value, options, onChange }: { label: string; value: string; options: Record<string, string>; onChange: (value: string) => void }) {
  return <div className="wb-case-choice"><span>{label}</span><div role="group" aria-label={label}>{Object.entries(options).map(([id, name]) => <button type="button" key={id} aria-pressed={value === id} onClick={() => onChange(id)}><CaseIcon kind={id}/><span>{name}</span></button>)}</div></div>;
}
