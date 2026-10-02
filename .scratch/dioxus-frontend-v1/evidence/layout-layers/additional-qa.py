import json,subprocess,sys,pathlib
session,url=sys.argv[1:3];out=pathlib.Path(sys.argv[3]);out.mkdir(parents=True,exist_ok=True)
steps=[]
def run(*args):
 p=subprocess.run(['agent-browser','--session',session,*args,'--json'],text=True,capture_output=True)
 try:r=json.loads(p.stdout)
 except ValueError:r={'raw':p.stdout,'error':p.stderr}
 steps.append({'args':list(args),'exit':p.returncode,'result':r});(out/'steps.json').write_text(json.dumps(steps,indent=2))
 if p.returncode or not r.get('success',True):raise RuntimeError(str(steps[-1]))
 return r.get('data',r)
def probe(name,expr):
 r=run('eval',f'JSON.stringify({expr})');v=r.get('result',r)
 if isinstance(v,str):v=json.loads(v)
 (out/(name+'.json')).write_text(json.dumps(v,indent=2));return v
def wait(expr):run('wait','--fn',expr)
counts='({caps:document.querySelectorAll(".m1-keycap-overlay").length,parts:document.querySelectorAll(".m1-scene-part").length,ghosts:document.querySelectorAll(".m1-matrix-key").length,pads:document.querySelectorAll(".m1-part-pad").length,drills:document.querySelectorAll(".m1-part-drill").length,graphics:document.querySelectorAll(".m1-footprint-graphic").length,graphicErrors:document.querySelectorAll(".m1-footprint-graphics-error").length,board:document.querySelectorAll(".m1-outline").length,labels:document.querySelectorAll(".m1-part-label").length,viewBox:document.querySelector(".m1-canvas").getAttribute("viewBox"),revision:document.querySelector(".m1-editor-footer").textContent.match(/Revision (\\d+)/)?.[1],footprints:document.querySelector(".m1-footprints-toggle").getAttribute("aria-pressed")})'
# Dedicated subpath session already contains a saved REVIUNG41.
run('set','viewport','1440','900')
try:
 run('set','offline','on');run('reload');run('wait','.m1-canvas');run('click','.m1-footprints-toggle')
 wait('document.querySelectorAll(".m1-footprint-graphic").length===309 && !document.querySelector(".m1-footprint-graphics[data-status=loading]")')
 r=probe('offline-footprints',counts);assert r['pads']==193 and r['drills']==238 and r['graphicErrors']==0,r
finally:run('set','offline','off')
run('click','.m1-project-menu summary');run('upload','.m1-project-menu input[type=file]',str(pathlib.Path(__file__).with_name('cap-override.boardstudio')))
wait('document.querySelector(".m1-canvas-toolbar span").textContent.includes("Layout layer regression")')
expected=json.loads(pathlib.Path(__file__).with_name('cap-override-expected.json').read_text())
sel='[data-part-id='+json.dumps(expected['id'])+']'
cap=probe('override-cap',f'({{width:document.querySelector({json.dumps(sel+" .m1-keycap-overlay rect")}).getAttribute("width"),height:document.querySelector({json.dumps(sel+" .m1-keycap-overlay rect")}).getAttribute("height"),back:document.querySelector({json.dumps("[data-part-id="+json.dumps(expected["back_id"])+"]")}).getAttribute("transform")}})')
assert cap['width']=='27' and cap['height']=='18' and 'scale(-1 1)' in cap['back'],cap
# Click near the outer cap edge, beyond the physical switch courtyard.
hit=probe('cap-edge-hit',f'(()=>{{const e=document.querySelector({json.dumps(sel+" .m1-part-hit-area")});const p=new DOMPoint(-13.3,0).matrixTransform(e.getScreenCTM());return {{x:p.x,y:p.y,id:document.elementFromPoint(p.x,p.y).closest("[data-part-id]")?.dataset.partId}}}})()')
assert hit['id']==expected['id'],hit
run('mouse','move',str(round(hit['x'])),str(round(hit['y'])));run('mouse','down');run('mouse','up');run('wait','#m1-position-x')
assert probe('selected-cap','document.querySelector(".m1-keycap-overlay.is-selected").parentElement.dataset.partId')==expected['id']
run('click','.m1-project-menu summary');run('click','.m1-project-menu .m1-library button:nth-child(2)')
wait('!document.querySelector(".m1-project-menu").open')
wait('document.querySelector(".m1-canvas-toolbar span").textContent.toLowerCase().includes("sofle")')
boards=probe('sofle-boards','[...document.querySelector("select[aria-label=Board]").options].map(x=>({value:x.value,label:x.textContent}))')
for board in boards:
 run('select','select[aria-label="Board"]',board['value'])
 wait('!document.querySelector(".m1-footprint-graphics[data-status=loading]")')
 r=probe('sofle-'+board['value'],counts);assert r['caps']>0 and r['graphicErrors']==0,r
 probe('sofle-ids-'+board['value'],'[...document.querySelectorAll(".m1-scene-part")].map(x=>x.dataset.partId)')
print('PASS subpath offline generated footprints, authored27x18 cap, back reflection, outercap hit target, Sofle board scopes')
