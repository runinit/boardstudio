from pathlib import Path
import json, subprocess, socket, time, sys, gzip, hashlib
p=Path(__file__).resolve().parent
prefix=sys.argv[1]
assert prefix in ('/','/boardstudio/')
name='boardstudio-p1-cad-20261001-'+('root' if prefix=='/' else 'subpath')
sock=socket.socket();sock.bind(('127.0.0.1',0));port=sock.getsockname()[1];sock.close()
log=(p.parent/'target'/(name+'-server.raw.log')).open('w')
server=subprocess.Popen(['python3','-m','http.server',str(port),'--bind','127.0.0.1','--directory',str(p/'site')],stdout=log,stderr=subprocess.STDOUT)
commands=[]
def browser(args):
    command=['agent-browser','--session',name,'--executable-path','/usr/bin/chromium','--json',*args]
    result=subprocess.run(command,capture_output=True,text=True,timeout=35)
    commands.append({'command':command,'exit_code':result.returncode,'stdout':result.stdout,'stderr':result.stderr})
    return result
verdict={'status':'failed','url':f'http://127.0.0.1:{port}{prefix}'}
try:
    time.sleep(.5)
    assert server.poll() is None,'task-owned server did not start'
    assert browser(['open',verdict['url']]).returncode==0,'browser open failed'
    result=None
    for attempt in range(90):
        output=browser(['eval',"(() => {const text=document.querySelector('#report')?.textContent;try{return JSON.parse(text)}catch{return null}})()"])
        assert output.returncode==0,'DOM evaluation failed'
        result=json.loads(output.stdout).get('data',{}).get('result')
        if isinstance(result,dict):break
        time.sleep(1)
    verdict['report']=result
    browser(['snapshot','-i'])
    errors=browser(['errors']);assert errors.returncode==0,'browser error inspection failed'
    assert json.loads(errors.stdout).get('data',{}).get('errors')==[],'unexpected page errors'
    assert isinstance(result,dict),'Rust report did not complete'
    assert result['status']=='passed',result
    assert result['prefix']==prefix and result['revision']==7
    assert result['sender_step_bytes_after_transfer']==0 and result['preview_transfer_bytes']>0 and result['export_transfer_bytes']>0
    assert result['rejected']==0
    for label in ('preview','export'):
        assert abs(result[label]['volume']-720)<.1 and result[label]['vertices']>0
    assert all(result[key]=='settled' for key in ['close','crash','init_failure'])
    step=bytes.fromhex(result.pop('step_hex'))
    artifact=p/(name+'.step');artifact.write_bytes(step)
    Path(str(artifact)+'.gz').write_bytes(gzip.compress(step,mtime=0))
    verdict['step']={'path':artifact.name,'archive':artifact.name+'.gz','bytes':len(step),'sha256':hashlib.sha256(step).hexdigest()}
    verdict['status']='passed'
except Exception as error:verdict['error']=str(error)
finally:
    try:browser(['close'])
    finally:
        server.terminate();server.wait(timeout=10);log.close()
        raw=Path(log.name).read_bytes();(p/(name+'-server.log.gz')).write_bytes(gzip.compress(raw,mtime=0))
        # Keep complete command results; compress large frame/STEP evidence separately.
        (p/(name+'-commands.json.gz')).write_bytes(gzip.compress((json.dumps(commands,indent=2)+'\n').encode(),mtime=0))
        (p/(name+'-browser.json')).write_text(json.dumps(verdict,indent=2)+'\n')
print(json.dumps(verdict,indent=2));sys.exit(0 if verdict['status']=='passed' else 1)
