// Test tooling only. Each invocation creates a fresh public CAD package lifetime.
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {buildAssembly,previewAssembly} from '../../../../../cad/src/index.ts';
const prepared=JSON.parse(readFileSync(new URL('../../fixtures/prepared.json',import.meta.url),'utf8'));
const sequence=process.argv[2].split(',');
const outputs=[];
for(const operation of sequence){
    const result=operation==='preview'?await previewAssembly(prepared,()=>{}):await buildAssembly(prepared);
    const positions=Array.from(result.mesh.positions),normals=Array.from(result.mesh.normals);
    const hash=array=>createHash('sha256').update(new Uint8Array(array.buffer,array.byteOffset,array.byteLength)).digest('hex');
    const hex=array=>Buffer.from(array.buffer,array.byteOffset,array.byteLength).toString('hex');
    outputs.push({operation,revision:result.revision,bodyIds:result.bodies.map(b=>b.id),mesh:{positions,normals,positionsHex:hex(result.mesh.positions),normalsHex:hex(result.mesh.normals),positionHash:hash(result.mesh.positions),normalHash:hash(result.mesh.normals)},step:operation==='export'?Buffer.from(result.step).toString('base64'):null});
}
process.stdout.write(JSON.stringify(outputs));
