import { useRef } from 'react';
import type { DemoId } from '../demos/keyboards';
import { BrandMark } from './WorkbenchIcons';
import { ProjectLibrary, ProjectLibraryIcon } from './ProjectLibrary';
import { useWorkbenchTheme } from './useWorkbenchTheme';

type Props = {
  error: string;
  onNew: () => void;
  onOpen: (id: string) => void;
  onOpenDemo: (id: DemoId) => void;
  onImport: (file: File) => void;
};

export function ProjectStart({ error, onNew, onOpen, onOpenDemo, onImport }: Props) {
  const file = useRef<HTMLInputElement>(null);
  useWorkbenchTheme();

  return <main className="wb-root wb-unified wb-project-start">
    <header className="wb-topbar"><span className="wb-project-start-brand"><BrandMark />Board Studio</span></header>
    <section id="wb-project-dropdown" className="wb-project-dropdown" aria-label="Project menu">
      <div className="wb-project-menu-heading"><h1>Keyboards</h1></div>
      <div className="wb-keyboard-library-actions">
        <button className="wb-library-new" onClick={onNew}><ProjectLibraryIcon name="new" />New project</button>
        <button className="wb-open-project" onClick={() => file.current?.click()}><ProjectLibraryIcon name="open" />Open project…</button>
      </div>
      <input ref={file} className="wb-project-file-input" type="file" accept=".boardstudio" tabIndex={-1} onChange={event => {
        const selected = event.currentTarget.files?.[0];
        if (selected) onImport(selected);
        event.currentTarget.value = '';
      }} />
      {error && <p className="wb-project-start-error" role="alert">{error}</p>}
      <ProjectLibrary onNew={onNew} onOpen={onOpen} onOpenDemo={onOpenDemo} />
    </section>
  </main>;
}
