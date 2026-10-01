from pathlib import Path
import subprocess,sys,json,gzip,hashlib,os
p=Path(__file__).resolve().parent.parent
root=p.parents[2]
name=sys.argv[1];records=[]
results=p/"evidence/findings-repair"
results.mkdir(exist_ok=True)
if name=='release':
    jobs=[(['wasm-pack','build',str(p),'--target','web','--release','--out-dir','worker-pkg','--locked','--no-default-features','--features','worker'],root,'worker-release')]
    for prefix,label in [('/','root'),('/boardstudio/','subpath')]:
        jobs.extend([(['dx','build','--web','--release','--base-path',prefix,'--cargo-args=--locked'],p,label+'-host-release'),(['python3','build.py',str(p/'target/dx/boardstudio-p1-cad/release/web/public'),prefix],p,label+'-stage'),(['python3','evidence/verify-browser.py',prefix],p,label+'-browser')])
elif name=='final':
    jobs=[(['cargo','test','--locked','--all-targets'],p,'native-tests'),(['cargo','build','--locked'],p,'native-build'),(['cargo','fmt','--check'],p,'fmt'),(['cargo','clippy','--locked','--all-targets','--','-D','warnings'],p,'native-clippy'),(['cargo','clippy','--locked','--all-targets','--target','wasm32-unknown-unknown','--','-D','warnings'],p,'host-clippy'),(['cargo','clippy','--locked','--lib','--target','wasm32-unknown-unknown','--no-default-features','--features','worker','--','-D','warnings'],p,'worker-clippy'),(['pnpm','--dir','cad','exec','tsc','--noEmit'],root,'cad-typecheck'),(['pnpm','--dir','cad','test'],root,'cad-regressions')]
else:raise ValueError(name)
for command,cwd,label in jobs:
    print('RUN '+label,flush=True)
    raw=p/'target'/(name+'-'+label+'.raw.log');raw.parent.mkdir(exist_ok=True)
    with raw.open('wb') as stream:r=subprocess.run(command,cwd=cwd,stdout=stream,stderr=subprocess.STDOUT)
    data=raw.read_bytes();log=name+'-'+label+'.log.gz';(results/log).write_bytes(gzip.compress(data,mtime=0))
    record={'command':command,'cwd':str(cwd),'exit_code':r.returncode,'log':log,'raw_sha256':hashlib.sha256(data).hexdigest()};records.append(record)
    (results/(name+'-checks.json')).write_text(json.dumps(records,indent=2)+'\n');print(json.dumps(record),flush=True)
    if r.returncode:print(data.decode(errors='replace')[-7000:],flush=True);sys.exit(r.returncode)
