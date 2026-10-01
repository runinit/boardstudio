import { partChoices, partCatalogSearchText, partCatalogLabel } from './partsCatalog';
import type { ModuleDefinition, PartDefinition } from '../../../contracts/src/index';

type AssemblyOption = { id: string; name: string; definitionId: string };
const categories: [PartDefinition['kind'] | 'utility', string][] = [['switch', 'Switches'], ['controller', 'Controllers'], ['connector', 'Connectors & sockets'], ['encoder', 'Encoders'], ['passive', 'Passives & LEDs'], ['utility', 'Utilities'], ['custom', 'Custom']];

export function PartsLibrary({ definitions, assemblies, modules = [], modulesPending, modulesError, query, selected, onSearch, onSelect, onAssembly, onCreate, onImport }: {
  definitions: PartDefinition[]; assemblies: AssemblyOption[]; query: string; selected: string;
  modules?: ModuleDefinition[]; modulesPending?: boolean; modulesError?: string;
  onSearch: (value: string) => void; onSelect: (id: string) => void; onAssembly: (id: string) => void;
  onCreate?: () => void; onImport?: (file: File) => void;
}) {
  const search = query.trim().toLowerCase();
  const matches = (text: string) => !search || text.toLowerCase().includes(search);
  const filteredAssemblies = assemblies.filter((item) => matches(`${item.name} key assembly`));
  const groups = categories.map(([kind, name]) => ({ name, items: partChoices(definitions).filter((item) => (item.kind as string) === kind && matches(`${partCatalogSearchText(item)} ${name}`)) })).filter((group) => group.items.length);
  const moduleGroups = [...new Set(modules.map(module => module.catalogueRow ?? module.id))].map(row => {const variants=modules.filter(module=>(module.catalogueRow ?? module.id)===row);return {row,name:variants[0].name,variants};}).filter(group => matches(`${group.name} ${group.row} VIK ${group.variants.map(variant => `${variant.family} ${variant.variant}`).join(' ')}`));
  return <div className="wb-parts-catalog">
    <h2>Parts library</h2>
    <label className="wb-library-search">Search parts<input type="search" aria-label="Search footprints" placeholder="Name or category" value={query} onChange={(event) => onSearch(event.target.value)} /></label>
    <div className="wb-catalog-scroll">
      <details open><summary>Key Assemblies <small>{filteredAssemblies.length}</small></summary>
        <div role="listbox" aria-label="Key assemblies">{filteredAssemblies.map((item) => <button role="option" aria-selected={selected === `assembly:${item.id}`} key={item.id} onClick={() => onAssembly(item.id)}>{item.name}</button>)}</div>
      </details>
      <details open><summary>Components <small>{groups.reduce((sum, group) => sum + group.items.length, 0)}</small></summary>
        <div role="listbox" aria-label="Footprint library">{groups.map((group) => <section key={group.name} aria-label={group.name}><h3>{group.name}</h3>{group.items.map((item) => <button key={item.id} role="option" title={item.generator?.source} aria-selected={selected === item.id} onClick={() => onSelect(item.id)}>{partCatalogLabel(item)}</button>)}</section>)}</div>
      </details>
      <details open><summary>VIK modules <small>{moduleGroups.length}</small></summary>
        {modulesPending && <p role="status">Loading module sources…</p>}
        {modulesError && <p role="alert">{modulesError}</p>}
        <div role="listbox" aria-label="VIK modules">{moduleGroups.map(group => <button key={group.row} role="option" title={group.row} aria-selected={group.variants.some(variant => selected === `module:${variant.id}`)} onClick={() => onSelect(`module:${group.variants[0].id}`)}>{group.name}<small>{moduleGroups.some(other=>other.row!==group.row&&other.name===group.name)?` · ${group.row}`:''}{group.variants.length > 1 ? ` · ${group.variants.length} variants` : ''}</small></button>)}</div>
      </details>
      {!filteredAssemblies.length && !groups.length && !moduleGroups.length && !modulesPending && <p>No parts match this search.</p>}
    </div>
    <div className="wb-catalog-actions">
      {onImport && <label className="wb-footprint-import">Import KiCad footprint<input type="file" accept=".kicad_mod" onChange={(event) => {
        const file = event.currentTarget.files?.[0];
        if (file) onImport(file);
        event.currentTarget.value = '';
      }} /></label>}
      {onCreate && <button className="wb-inspector-link" onClick={onCreate}>New custom component</button>}
    </div>
  </div>;
}
