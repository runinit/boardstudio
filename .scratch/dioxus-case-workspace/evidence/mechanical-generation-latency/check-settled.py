import subprocess,time,json,re,sys
from pathlib import Path
out=Path(__file__).parent
cmd=['agent-browser','--session',sys.argv[1] if len(sys.argv)>1 else 'mechanical-ui-layout-0cad7577','snapshot','-i']
t=time.time();r=subprocess.run(cmd,text=True,capture_output=True,timeout=40);elapsed=time.time()-t
(out/'settled-snapshot.txt').write_text(r.stdout+r.stderr)
result={'command':cmd,'utc_epoch':t,'elapsed_seconds':elapsed,'returncode':r.returncode,'cancel_disabled':any('Cancel generation' in x and re.search(r'\[[^\]]*\bdisabled\b',x) for x in r.stdout.splitlines()),'preview_present':'Interactive 3D Case preview' in r.stdout}
(out/'settled-observation.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
assert r.returncode==0 and result['cancel_disabled'] and result['preview_present'], 'generation did not settle to completed visible preview'
