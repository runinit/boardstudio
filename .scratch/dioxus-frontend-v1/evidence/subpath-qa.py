import json,subprocess,sys,pathlib
session=sys.argv[1];url=sys.argv[2];out=pathlib.Path(sys.argv[3]);out.mkdir(parents=True,exist_ok=True)
steps=[]
def run(*args):
 p=subprocess.run(['agent-browser','--session',session,*args,'--json'],text=True,capture_output=True)
 try:r=json.loads(p.stdout)
 except ValueError:r={'raw':p.stdout,'error':p.stderr}
 steps.append({'args':list(args),'exit':p.returncode,'result':r});(out/'browser-steps.json').write_text(json.dumps(steps,indent=2))
 if p.returncode or not r.get('success',True):raise RuntimeError(str(steps[-1]))
 return r.get('data',r)
def probe(name,expr):
 r=run('eval',f'JSON.stringify({expr})');v=r.get('result',r)
 if isinstance(v,str):v=json.loads(v)
 (out/(name+'.json')).write_text(json.dumps(v,indent=2));return v
def wait(expr):run('wait','--fn',expr)
run('open',url);run('set','viewport','390','844');run('wait','.m1-library-landing');run('click','.m1-library-landing .m1-library button:first-child');run('wait','.m1-canvas')
run('select','select[aria-label="Theme"]','dark');wait('document.documentElement.dataset.theme==="dark"')
wait('navigator.serviceWorker.controller !== null')
probe('subpath-online','({title:document.title,url:location.href,worker:navigator.serviceWorker.controller.scriptURL,theme:document.documentElement.dataset.theme,workspaceWidth:document.querySelector(".m1-workspace-select").getBoundingClientRect().width,scrollWidth:document.documentElement.scrollWidth,viewport:innerWidth})')
try:
 run('set','offline','on');run('reload');run('wait','.m1-canvas');wait('document.documentElement.dataset.theme==="dark"')
 probe('subpath-offline','({title:document.title,url:location.href,online:navigator.onLine,theme:document.documentElement.dataset.theme,objects:document.querySelectorAll(".m1-component-list button").length,worker:navigator.serviceWorker.controller.scriptURL})')
finally:run('set','offline','off')
print('PASS subpath compact load/theme and offline reopening')
