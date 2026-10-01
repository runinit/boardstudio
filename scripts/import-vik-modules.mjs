// Pinned board sources -> Rust-owned snapshots. No browser-side PCB parser.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root=resolve(dirname(fileURLToPath(import.meta.url)), '..');
const manifestPath=resolve(root,'app/src/modules/import-manifest.json');
const outputPath=resolve(root,'app/src/modules/imported-modules.json');
const manifest=JSON.parse(readFileSync(manifestPath,'utf8'));
const assets=JSON.parse(readFileSync(resolve(root,'app/src/modules/asset-ledger.json'),'utf8')).models;
if (manifest.formatVersion!==1 || !Array.isArray(manifest.entries)) throw new Error('Expected a version 1 module manifest');
const driver=process.env.BOARDSTUDIO_ARTIFACT_DRIVER ?? resolve(root,'core/target/debug/examples/artifact_request');
const ids=new Set();
const variantsByRow=new Map();
for (const entry of manifest.entries) {
  const variants=variantsByRow.get(entry.row) ?? new Set();
  if (variants.has(entry.variant)) throw new Error(`Duplicate source variant: ${entry.row} / ${entry.variant}`);
  variants.add(entry.variant);
  variantsByRow.set(entry.row,variants);
}
for (const asset of assets) {
  const path=resolve(root,asset.bundledFile);
  const bytes=readFileSync(path);
  if (createHash('sha256').update(bytes).digest('hex')!==asset.sha256) throw new Error(`Model hash mismatch: ${asset.path}`);
  const bounds=asset.nativeBoundsMm;
  for (const [axis,min,max,length] of [['X','XMin','XMax','XLength'],['Y','YMin','YMax','YLength'],['Z','ZMin','ZMax','ZLength']]) {
    const lower=bounds?.[min], upper=bounds?.[max], extent=bounds?.[length];
    if (![lower,upper,extent].every(Number.isFinite) || upper<=lower || Math.abs((upper-lower)-extent)>0.00001) {
      throw new Error(`Invalid ${axis} bounds in model ledger: ${asset.path}`);
    }
  }
  if (asset.appliesToVariants!==undefined) {
    if (!Array.isArray(asset.appliesToVariants) || !asset.appliesToVariants.length) throw new Error(`Empty model variant applicability: ${asset.path}`);
    const known=variantsByRow.get(asset.catalogRow) ?? new Set();
    for (const variant of asset.appliesToVariants) if (!known.has(variant)) throw new Error(`Unknown ${asset.catalogRow} model variant ${variant}: ${asset.path}`);
  }
}
function importEntry(entry, repair) {
  for (const field of ['row','name','file','sha256','repository','revision','sourcePath','license','family','variant']) {
    if (typeof entry[field]!=='string' || !entry[field].trim()) throw new Error(`Missing ${field}`);
  }
  const variant=repair ? `${entry.variant} · 3V3 pullups, JP1 bridged` : entry.variant;
  const id=`vik:${entry.row.replace(/[^a-z0-9]+/giu,'-')}:${variant.replace(/[^a-z0-9]+/giu,'-')}`;
  if (ids.has(id)) throw new Error(`Duplicate module identity: ${id}`);
  ids.add(id);
  const source=readFileSync(resolve(dirname(manifestPath),entry.file),'utf8');
  if (createHash('sha256').update(source).digest('hex')!==entry.sha256) throw new Error(`Source hash mismatch: ${entry.file}`);
  const input={id:'module-import',kind:'import-module-board',definitionId:id,name:entry.name,source,
    provenance:{repository:entry.repository,revision:entry.revision,path:entry.sourcePath,license:entry.license,upstreamStatus:entry.status},family:entry.family,variant, ...(repair ? {repair} : {})};
  const result=spawnSync(driver,{input:JSON.stringify(input)+'\n',encoding:'utf8',maxBuffer:64*1024*1024});
  if (result.error) throw result.error;
  if (result.status!==0) throw new Error(result.stderr || `Importer failed for ${entry.file}`);
  const reply=JSON.parse(result.stdout.trim());
  if (reply.kind!=='import-module-board') throw new Error(`${entry.file}: ${reply.error?.message ?? 'Unexpected module reply'}`);
  const definition=reply.result;
  definition.catalogueRow=entry.row;
  for (const constituent of definition.constituents) constituent.purchased=(entry.purchasedReferences ?? []).includes(constituent.reference);
  definition.candidateModels=assets.filter(asset=>(asset.catalogRow===entry.row || asset.catalogRow==='shared connector')
    && (!asset.appliesToVariants || asset.appliesToVariants.includes(entry.variant))).map(asset=>{
    const bounds=asset.nativeBoundsMm;
    return {assetId:asset.assetId,name:asset.path.split('/').at(-1),source:{repository:`https://github.com/${asset.repository}`,revision:asset.revision,path:asset.path,license:asset.declaredLicense,sha256:asset.sha256},boundsMin:{x:bounds.XMin,y:bounds.YMin,z:bounds.ZMin},boundsMax:{x:bounds.XMax,y:bounds.YMax,z:bounds.ZMax},verifiedAlignment:false};
  });
  const alignedAsset=assets.find(asset=>asset.catalogRow===entry.row && asset.appliesToVariants?.includes(entry.variant) && asset.boardToModelTransform);
  if (alignedAsset) {
    const transform=alignedAsset.boardToModelTransform;
    definition.models=[{assetId:alignedAsset.assetId,offset:transform.offset,rotation:transform.rotation,scale:transform.scale}];
    const profile=alignedAsset.nominalProfile;
    if (profile) {
      definition.volumes=[{id:`${id}/nominal-occupied`,geometry:profile.occupied,source:`${alignedAsset.path} · source-derived native bounds transformed into PCB midplane`,purpose:'nominal occupied',qualified:false}];
      definition.openings=[{id:`${id}/nominal-viewing-aperture`,geometry:profile.viewingAperture,source:profile.viewingApertureSource,purpose:'nominal viewing aperture',qualified:false}];
      definition.gates.push({output:'mechanical',code:'nominal-display-assumptions',message:'Review the manufacturer active-area rectangle and its centering on the source display-body bounds in the app before qualifying case output.'});
    }
  }
  definition.electrical=entry.electrical;
  if (entry.electrical.rotaryProfile && entry.row==='ec11-evqwgd001') {
    definition.gates=definition.gates.filter(g=>!(g.output==='firmware' && g.code==='module-driver'));
  }
  if (entry.electrical.protocol==='pass-through') definition.gates=definition.gates.filter(g=>g.output!=='firmware');
  if (entry.notice && !repair) definition.gates.push({output:'electrical',code:'source-caveat',message:entry.notice});
  if (entry.electrical.protocol==='nonstandard') definition.gates.push({output:'electrical',code:'nonstandard-interface',message:'Select and qualify the source-specific signal mapping, analog or half-duplex capabilities and isolated bus resources before electrical handoff.'});
  if (entry.row==='haptic-drv2605l') {
    definition.gates.push({output:'mechanical',code:'actuator',message:'Choose and qualify the separate actuator body, mounting, current and calibration.'});
  }
  return {row:entry.row,definition};
}
const modules=manifest.entries.flatMap(entry=>entry.row==='haptic-drv2605l'
  ? [importEntry(entry),importEntry(entry,'drv2605l-pullups3v3')]
  : [importEntry(entry)]);
const output=JSON.stringify({formatVersion:1,modules},null,2)+'\n';
if (process.argv.includes('--check')) {
  if (readFileSync(outputPath,'utf8')!==output) throw new Error('Module catalogue drift: regenerate and review the snapshots');
  console.log(`Verified ${modules.length} module variants`);
} else {
  writeFileSync(outputPath,output);
  console.log(`Imported ${modules.length} module variants`);
}
