import { useEffect, useState } from 'react';
import type { ModuleDefinition, ModuleVolume, PartModel, RotaryProfile } from '../../../contracts/src/index';
import { DraftInput } from './InspectorControls';
import { InspectorSection } from './InspectorSection';
import { makeId } from './workbenchGeometry';

type ShapeDraft = { x:string; y:string; width:string; depth:string; z:string; height:string; source:string; purpose:string; qualified:boolean };
const emptyShape = ():ShapeDraft=>({x:'0',y:'0',width:'',depth:'',z:'',height:'',source:'',purpose:'occupied',qualified:false});
const identity = {offset:{x:0,y:0,z:0},rotation:{x:0,y:0,z:0},scale:{x:1,y:1,z:1}};
type RotaryDraft = {a:string;b:string;common:string;steps:string;triggersPerRotation:string;driver:string};
const rotaryDraft = (profile?:RotaryProfile):RotaryDraft=>({a:profile?.a??'',b:profile?.b??'',common:profile?.common??'',steps:profile?.steps===undefined?'':String(profile.steps),triggersPerRotation:profile?.triggersPerRotation===undefined?'':String(profile.triggersPerRotation),driver:profile?.driver??''});
export function ModuleProfileEditor({definition,onSave}:{definition:ModuleDefinition;onSave:(definition:ModuleDefinition)=>unknown}) {
  const [volumes,setVolumes]=useState(definition.volumes);
  const [openings,setOpenings]=useState(definition.openings);
  const [models,setModels]=useState(definition.models);
  const [shape,setShape]=useState(emptyShape);
  const [reviewed,setReviewed]=useState(false);
  const [rotary,setRotary]=useState(()=>rotaryDraft(definition.electrical.rotaryProfile));
  const [error,setError]=useState('');
  const savedProfile = JSON.stringify({id:definition.id,volumes:definition.volumes,openings:definition.openings,models:definition.models,rotaryProfile:definition.electrical.rotaryProfile});
  useEffect(()=>{
    const profile = JSON.parse(savedProfile) as Pick<ModuleDefinition,'id'|'volumes'|'openings'|'models'> & {rotaryProfile?:RotaryProfile};
    setVolumes(profile.volumes);setOpenings(profile.openings);setModels(profile.models);setRotary(rotaryDraft(profile.rotaryProfile));setReviewed(false);setError('');
  },[savedProfile]);
  const addShape=()=>{
    const values=Object.fromEntries((['x','y','width','depth','z','height'] as const).map(key=>[key,Number(shape[key])])) as Record<'x'|'y'|'width'|'depth'|'z'|'height',number>;
    if (Object.values(values).some(value=>!Number.isFinite(value)) || !shape.z.trim() || values.width<=0 || values.depth<=0 || values.height<=0 || !shape.source.trim()) {setError('Enter measured dimensions, a Z position and source evidence.');return;}
    const {x,y,width,depth,z,height}=values;
    const volume:ModuleVolume={id:`volume/${makeId()}`,purpose:shape.purpose,source:shape.source.trim(),qualified:shape.qualified,geometry:{points:[{x:x-width/2,y:y-depth/2},{x:x+width/2,y:y-depth/2},{x:x+width/2,y:y+depth/2},{x:x-width/2,y:y+depth/2}],z,height}};
    if(shape.purpose==='opening')setOpenings([...openings,volume]);else setVolumes([...volumes,volume]);
    setShape(emptyShape());setError('');
  };
  const updateModel=(index:number,vector:'offset'|'rotation'|'scale',axis:'x'|'y'|'z',value:string)=>setModels(models.map((model,i)=>i===index?{...model,[vector]:{...model[vector],[axis]:Number(value)}}:model));
  const save=async()=>{
    if(models.some(model=>[...Object.values(model.offset),...Object.values(model.rotation),...Object.values(model.scale)].some(value=>!Number.isFinite(value)) || Object.values(model.scale).some(value=>value<=0))) {setError('Model transforms need finite coordinates and positive scale.');return;}
    const geometryReviewed=reviewed&&volumes.length>0&&[...volumes,...openings].every(volume=>volume.qualified);
    try{await onSave({...definition,volumes,openings,models,gates:definition.gates.filter(gate=>!(geometryReviewed&&gate.output==='mechanical'&&gate.code==='assembled-envelope'))});setError('');}
    catch(failure){setError(String(failure instanceof Error?failure.message:failure));}
  };
  const saveRotary=async()=>{
    const steps=Number(rotary.steps),triggersPerRotation=Number(rotary.triggersPerRotation);
    if(!rotary.a.trim()||!rotary.b.trim()||!rotary.common.trim()||new Set([rotary.a.trim(),rotary.b.trim(),rotary.common.trim()]).size<3||rotary.driver!=='ec11'||!rotary.steps.trim()||!rotary.triggersPerRotation.trim()||!Number.isInteger(steps)||steps<=0||!Number.isInteger(triggersPerRotation)||triggersPerRotation<=0){setError('Enter distinct A, B and common terminals, choose the driver, and provide positive whole-number pulses and actions per rotation.');return;}
    try{await onSave({...definition,electrical:{...definition.electrical,rotaryProfile:{a:rotary.a.trim(),b:rotary.b.trim(),common:rotary.common.trim(),steps,triggersPerRotation,driver:'ec11'}}});setError('');}
    catch(failure){setError(String(failure instanceof Error?failure.message:failure));}
  };
  return <InspectorSection title="Assembly geometry" detail={`${volumes.length} volumes · ${openings.length} openings`}>
    {definition.electrical.rotaryProfile && <InspectorSection title="Rotary encoder profile" detail={definition.electrical.rotaryProfile.driver ?? 'Driver not selected'}>
      <p className="wb-empty-note">The source names the A/B/common terminals and EC11 driver. Pulses and actions per rotation are not supplied; enter confirmed values before expecting firmware export to qualify this module.</p>
      <div className="wb-module-fields">{(['a','b','common'] as const).map(key=><label key={key}>{key==='a'?'A terminal':key==='b'?'B terminal':'Common terminal'}<DraftInput ariaLabel={`Rotary ${key} terminal`} value={rotary[key]} onCommit={value=>setRotary({...rotary,[key]:value})}/></label>)}</div>
      <label>Driver<select aria-label="Rotary driver" value={rotary.driver} onChange={event=>setRotary({...rotary,driver:event.target.value})}><option value="">Choose driver…</option><option value="ec11">EC11</option></select></label>
      <div className="wb-module-fields"><label>Pulses per rotation<DraftInput ariaLabel="Rotary pulses per rotation" type="number" min="1" step="1" value={rotary.steps} onCommit={value=>setRotary({...rotary,steps:value})}/></label><label>Actions per rotation<DraftInput ariaLabel="Rotary actions per rotation" type="number" min="1" step="1" value={rotary.triggersPerRotation} onCommit={value=>setRotary({...rotary,triggersPerRotation:value})}/></label></div>
      <button className="wb-primary" onClick={()=>void saveRotary()}>Save rotary profile</button>
    </InspectorSection>}
    <p className="wb-empty-note">Coordinates use the module PCB midplane. Positive Z is toward its front face. Enter measured component, mounting and cable space; unknown dimensions stay unqualified.</p>
    {[...volumes,...openings].map(volume=><div className="wb-module-existing" key={volume.id}><span>{volume.purpose} · {volume.geometry.height} mm · {volume.qualified?'Reviewed':'Unreviewed'}</span><button className="wb-inspector-link" onClick={()=>{setVolumes(volumes.filter(item=>item.id!==volume.id));setOpenings(openings.filter(item=>item.id!==volume.id));}}>Remove</button></div>)}
    <label>Volume purpose<select aria-label="Module volume purpose" value={shape.purpose} onChange={event=>setShape({...shape,purpose:event.target.value})}><option value="occupied">Component body</option><option value="support">Mounting hardware</option><option value="service">Cable / service space</option><option value="opening">Functional opening</option></select></label>
    <div className="wb-module-fields">{(['x','y','width','depth','z','height'] as const).map(key=><label key={key}>{key==='z'?'Z from midplane':key[0].toUpperCase()+key.slice(1)} · mm<DraftInput ariaLabel={`Module volume ${key}`} type="number" step="0.1" value={shape[key]} onCommit={value=>setShape({...shape,[key]:value})}/></label>)}</div>
    <label>Dimension evidence<DraftInput ariaLabel="Module volume evidence" value={shape.source} onCommit={value=>setShape({...shape,source:value})}/></label>
    <label className="wb-module-check"><input type="checkbox" checked={shape.qualified} onChange={event=>setShape({...shape,qualified:event.target.checked})}/>Dimensions and datum reviewed</label>
    <button onClick={addShape}>Add measured volume</button>
    {definition.candidateModels?.length ? <label>Attach candidate model<select aria-label="Module candidate model" value="" onChange={event=>{if(event.target.value)setModels([...models,{assetId:event.target.value,...structuredClone(identity)} as PartModel]);}}><option value="">Select source model…</option>{definition.candidateModels.map(model=><option key={model.assetId} value={model.assetId} disabled={models.some(binding=>binding.assetId===model.assetId)}>{model.name}</option>)}</select></label>:null}
    {models.map((model,index)=><fieldset className="wb-module-model" key={`${model.assetId}/${index}`}><legend>{model.assetId.split('/').at(-1)}</legend>{(['offset','rotation','scale'] as const).map(vector=><div className="wb-module-fields" key={vector}>{(['x','y','z'] as const).map(axis=><label key={axis}>{vector} {axis.toUpperCase()}<DraftInput ariaLabel={`Module model ${index+1} ${vector} ${axis}`} type="number" step="0.1" value={model[vector][axis]} onCommit={value=>updateModel(index,vector,axis,value)}/></label>)}</div>)}<button className="wb-inspector-link" onClick={()=>setModels(models.filter((_,i)=>i!==index))}>Remove model</button></fieldset>)}
    <label className="wb-module-check"><input type="checkbox" checked={reviewed} onChange={event=>setReviewed(event.target.checked)}/>Complete assembly, mounts, functional openings and cable clearance reviewed</label>
    <p className="wb-empty-note">Saving a candidate model retains its alignment review. Electrical repairs, driver support and other missing evidence keep their own blockers.</p>
    <button className="wb-primary" onClick={()=>void save()}>Save project module profile</button>
    {error&&<p role="alert">{error}</p>}
  </InspectorSection>;
}
