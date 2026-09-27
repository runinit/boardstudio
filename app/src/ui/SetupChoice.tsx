import type { ReactNode } from 'react';

export function SetupChoice({ label, children }: { label: string; children: ReactNode }) {
  return <div className="wb-setup-choice"><span>{label}</span><div role="group" aria-label={label}>{children}</div></div>;
}

export function SetupIcon({ kind }: { kind: 'keyboard' | 'split' | 'wireless' | 'wired' }) {
  return <svg viewBox="0 0 32 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
    {kind === 'keyboard' ? <><rect x="2" y="5" width="28" height="16" rx="2"/><path d="M7 10h2m3 0h2m3 0h2m3 0h3M7 14h2m3 0h2m3 0h2m3 0h3M10 18h12"/></>
      : kind === 'split' ? <><rect x="1" y="5" width="12" height="16" rx="2"/><rect x="19" y="5" width="12" height="16" rx="2"/><path d="M5 10h4m-4 4h4m-2 4h3m13-8h4m-4 4h4m-5 4h3"/></>
      : kind === 'wireless' ? <><path d="M3 8a20 20 0 0 1 26 0M7 12a14 14 0 0 1 18 0M11 16a8 8 0 0 1 10 0"/><circle cx="16" cy="20" r="1"/></>
      : <><path d="M2 5h6v7H2zM24 12h6v7h-6zM8 8h4a4 4 0 0 1 4 4v0a4 4 0 0 0 4 4h4M4 2v3m2-3v3m20 14v3m2-3v3"/></>}
  </svg>;
}
