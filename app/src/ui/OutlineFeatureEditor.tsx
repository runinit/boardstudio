import { useEffect, useRef } from 'react';
import type { OutlineFeature, Part } from '@boardstudio/v2-contracts';
import { CaseNumber } from './InspectorControls';
import { InspectorSection } from './InspectorSection';
import { OutlineGridControl, OutlineToolIcon } from './OutlineTools';
import { attachOutline, insertOutlinePoint, moveOutlinePoint, outlinePoints, removeOutlinePoint } from './outlineEditing';

type Props = {
  feature: OutlineFeature; connectionId?: string; title: string; parts: Part[];
  selectedPoint: number; onSelectPoint: (index: number) => void; grid: number; gridSelection?:number; onGrid: (grid: number) => void;
  onChange: (feature: OutlineFeature) => void; onClose: () => void;
  fixed?: boolean; createsCopy?: boolean;
};
const measurement = (value: number) => Number(value.toFixed(3));

export function OutlineFeatureEditor({ feature, connectionId, title, parts, selectedPoint, onSelectPoint, grid, gridSelection, onGrid, onChange, onClose, fixed, createsCopy }: Props) {
  const points = outlinePoints(feature, parts, connectionId);
  const index = Math.min(selectedPoint, points.length - 1);
  const point = points[index];
  const connection = feature.kind === 'part-envelope' ? feature.connections?.find(item => item.id === connectionId) : undefined;
  const list = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const container = list.current;
    const row = container?.querySelector<HTMLElement>('[aria-pressed="true"]');
    if (!container || !row) return;
    if (row.offsetTop < container.scrollTop + 28) container.scrollTop = Math.max(0, row.offsetTop - 28);
    else if (row.offsetTop + row.offsetHeight > container.scrollTop + container.clientHeight) container.scrollTop = row.offsetTop + row.offsetHeight - container.clientHeight;
  }, [index]);
  const attachmentId = connection ? connection.points[index]?.partId : feature.kind !== 'part-envelope' ? feature.anchorPartId : undefined;
  const missingAttachment = attachmentId && !parts.some(part => part.id === attachmentId);
  const canRemove = points.length > (connection ? 2 : 3);
  const insert = () => { onChange(insertOutlinePoint(feature, index, parts, connectionId)); onSelectPoint(index + 1); };
  const remove = () => { onChange(removeOutlinePoint(feature, index, connectionId)); onSelectPoint(Math.max(0, index - 1)); };
  return <div className="wb-outline-editor">
    <div className="wb-inspect-head"><h2>{title}</h2><button className="wb-secondary" onClick={onClose}>Done</button></div>
    <p className="wb-inspector-description">{createsCopy ? 'Your first change creates and activates a fixed copy. Generated stays available.' : fixed ? 'This outline stays fixed when components move. Changes save as you edit.' : connection ? 'A generated connection that follows its attached components.' : 'Changes save as you edit.'}</p>
    {feature.kind === 'part-envelope' && connection && <CaseNumber label="Connection width" unit="mm" validation="positive" value={connection.width} onCommit={width => onChange({ ...feature, connections: feature.connections?.map(item => item.id === connection.id ? { ...item, width } : item) })} />}
    {point && <>
      <div className="wb-outline-point-heading"><h3>{feature.kind === 'rect' ? 'Center position' : `Point ${index + 1}`}<small>{feature.kind !== 'rect' && ` of ${points.length}`}</small></h3><OutlineGridControl value={grid} selection={gridSelection} onChange={onGrid} /></div>
      <div className="wb-outline-coordinates">{(['x', 'y'] as const).map(axis => <CaseNumber key={`${index}-${axis}`} label={`Point ${index + 1} ${axis.toUpperCase()}`} unit="mm" validation="finite" value={measurement(point[axis])} onCommit={value => onChange(moveOutlinePoint(feature, index, { ...point, [axis]: value }, parts, connectionId))} />)}</div>
      {!fixed && !createsCopy && <label className="wb-outline-select"><span>{connection ? 'Point attachment' : 'Shape attachment'}</span><select aria-label={connection ? `Point ${index + 1} attachment` : 'Shape attachment'} value={attachmentId ?? ''} onChange={event => onChange(attachOutline(feature, event.target.value || undefined, parts, connectionId, index))}>
        <option value="">Fixed on board</option>
        {missingAttachment && <option value={attachmentId}>Missing component</option>}
        {parts.map(part => <option key={part.id} value={part.id}>{part.reference}</option>)}
      </select></label>}
      <p className={`wb-outline-attachment-note${missingAttachment ? ' is-error' : ''}`}>{missingAttachment ? 'This component is missing. Choose another attachment or keep the shape fixed.' : attachmentId ? `Follows ${parts.find(part => part.id === attachmentId)?.reference}. Detaching keeps the current position.` : 'Keeps its position when components move.'}</p>
      {feature.kind !== 'rect' && <div className="wb-outline-point-actions">
        <button className="wb-secondary" aria-label={`Insert after ${index + 1}`} disabled={!!connection && index === points.length - 1} onClick={insert}><OutlineToolIcon kind="add-point" />Insert after</button>
        <button className="wb-secondary" aria-label={`Remove point ${index + 1}`} disabled={!canRemove} title={!canRemove ? `Keep at least ${connection ? 'two' : 'three'} points.` : 'Remove the selected point'} onClick={remove}><OutlineToolIcon kind="remove" />Remove point</button>
      </div>}
    </>}
    {feature.kind === 'rect' && <InspectorSection title="Rectangle dimensions" defaultOpen>
      <CaseNumber label="Rectangle width" unit="mm" validation="positive" value={feature.size.x} onCommit={x => onChange({ ...feature, size: { ...feature.size, x } })} />
      <CaseNumber label="Rectangle height" unit="mm" validation="positive" value={feature.size.y} onCommit={y => onChange({ ...feature, size: { ...feature.size, y } })} />
      <CaseNumber label="Corner radius" unit="mm" validation="nonnegative" value={feature.radius} onCommit={radius => onChange({ ...feature, radius })} />
    </InspectorSection>}
    {feature.kind !== 'rect' && <div className="wb-outline-point-list" ref={list} role="group" aria-label="Outline points">
      <div className="wb-outline-point-columns" aria-hidden="true"><span>Point</span><span>X · mm</span><span>Y · mm</span><span /></div>
      {points.map((position, i) => <button key={i} aria-label={`Select outline point ${i + 1}`} aria-pressed={i === index} onClick={() => onSelectPoint(i)}>
        <span>{i + 1}</span><span>{measurement(position.x)}</span><span>{measurement(position.y)}</span><span>{connection?.points[i].partId && <OutlineToolIcon kind="attached" />}</span>
      </button>)}
    </div>}
    <InspectorSection title="Point editing shortcuts">
      <p className="wb-empty-note">Select a numbered point on the canvas or in the list. Drag to move it. Use a midpoint handle to insert a point.</p>
      <dl className="wb-outline-shortcuts"><div><dt>Arrow keys</dt><dd>Move one snap step</dd></div><div><dt>Shift + arrow</dt><dd>Move ten steps</dd></div><div><dt>Alt + drag</dt><dd>Move freely</dd></div><div><dt>Delete</dt><dd>Remove selected point</dd></div><div><dt>Escape</dt><dd>Cancel a drag</dd></div></dl>
    </InspectorSection>
  </div>;
}
