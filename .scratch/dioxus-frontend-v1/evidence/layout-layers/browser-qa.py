import json,subprocess,sys,pathlib,os
session,url=sys.argv[1:3];out=pathlib.Path(sys.argv[3]);out.mkdir(parents=True,exist_ok=True)
steps=[]
def run(*args):
 if args[0]=='screenshot' and os.environ.get('CAPTURE_LAYOUT_SCREENSHOTS')=='0':return {'skipped':'reuse reviewed equivalent visual fixture'}
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
run('open',url);run('set','viewport','1440','900');run('wait','.m1-library')
run('click','.m1-library-landing .m1-library button:first-child');run('wait','.m1-canvas');wait('document.querySelector(".m1-save-state").textContent.includes("Saved")')
run('select','select[aria-label="Theme"]','dark')
base=probe('default',counts);assert base['caps']==41 and base['pads']==0 and base['graphics']==0 and base['board']>0,base
run('click','#m1-layers-trigger');wait('document.querySelector("#m1-layers-trigger").getAttribute("aria-expanded")==="true"')
states=probe('layer-states','[...document.querySelectorAll(".m1-layer-list button")].map(x=>[x.getAttribute("aria-label"),x.getAttribute("aria-pressed")])')
assert states==[[f'Hide {x}','true'] for x in ['Keys','Components','Keycaps']]+[['Show Footprints','false'],['Hide Board','true']],states
run('screenshot',str(out/'desktop-dark.png'))
for layer in ['Keys','Components','Keycaps','Board']:
 run('click',f'.m1-layer-list button[aria-label="Hide {layer}"]');r=probe('hide-'+layer.lower(),counts)
 assert r['revision']==base['revision'] and r['viewBox']==base['viewBox'],r
 if layer=='Keys':assert r['caps']==0 and r['parts']==base['parts']-41 and r['ghosts']==0,r
 if layer=='Components':assert r['parts']==41 and r['caps']==41,r
 if layer=='Keycaps':assert r['caps']==0 and r['parts']==base['parts'] and r['labels']==base['parts'],r
 if layer=='Board':assert r['board']==0 and r['caps']==41,r
 run('click',f'.m1-layer-list button[aria-label="Show {layer}"]')
run('click','.m1-footprints-toggle');wait('document.querySelectorAll(".m1-footprint-graphic").length>0 && !document.querySelector(".m1-footprint-graphics[data-status=loading]")')
r=probe('footprints',counts);assert r['pads']==193 and r['drills']==238 and r['graphics']>100 and r['graphicErrors']==0 and r['footprints']=='true' and r['revision']==base['revision'],r
wait('document.querySelector(".m1-layer-list button:nth-child(4)").getAttribute("aria-pressed")==="true"')
run('screenshot',str(out/'footprints-dark.png'))
run('click','.m1-layer-list button[aria-label="Hide Footprints"]');r=probe('footprints-off',counts);assert r['pads']==0 and r['graphics']==0 and r['footprints']=='false',r
run('click','.m1-layer-list button[aria-label="Show Footprints"]');run('click','#m1-tab-PCB');run('click','#m1-tab-Layout');wait('document.querySelectorAll(".m1-footprint-graphic").length>0')
r=probe('workspace-retention',counts);assert r['footprints']=='true' and r['revision']==base['revision'],r
run('click','.m1-footprints-toggle')
run('click','#m1-layers-trigger');run('focus','.m1-layer-list button[aria-label="Hide Keys"]');run('press','Escape');wait('document.activeElement.id==="m1-layers-trigger" && document.querySelector("#m1-layers-trigger").getAttribute("aria-expanded")==="false"')
# Selection and authoritative edits carry the actual cap drawing with them.
target=probe('key-target','({id:document.querySelector(".m1-keycap-overlay").parentElement.dataset.partId,transform:document.querySelector(".m1-keycap-overlay").parentElement.getAttribute("transform")})')
selector='[data-part-id='+json.dumps(target['id'])+']'
run('click',selector+' .m1-part-hit-area');run('wait','#m1-position-x')
x=float(probe('before-edit','document.querySelector("#m1-position-x").value'))
run('fill','#m1-position-x',str(x+3));run('press','Enter');wait('document.querySelector(".m1-save-state").textContent.includes("Saved")')
wait(f'document.querySelector({json.dumps(selector)}).getAttribute("transform")!=={json.dumps(target["transform"])}')
edit=probe('edited-cap',f'document.querySelector({json.dumps(selector)}).getAttribute("transform")');assert edit!=target['transform'],edit
run('click','button[aria-label="Undo"]');wait('Math.abs(Number(document.querySelector("#m1-position-x").value)-'+str(x)+')<0.0001')
wait(f'document.querySelector({json.dumps(selector)}).getAttribute("transform")==={json.dumps(target["transform"])}')
assert probe('cap-after-undo',f'document.querySelector({json.dumps(selector)}).getAttribute("transform")')==target['transform']
run('select','select[aria-label="Theme"]','light');run('click','#m1-layers-trigger');run('screenshot',str(out/'desktop-light.png'))
run('set','viewport','390','844');run('screenshot',str(out/'compact-light.png'))
compact=probe('compact-bounds','({scroll:document.documentElement.scrollWidth,width:innerWidth,buttonHeight:document.querySelector(".m1-layer-list button").getBoundingClientRect().height,panel:document.querySelector(".m1-layers").getBoundingClientRect().toJSON(),canvas:document.querySelector(".m1-canvas").getBoundingClientRect().toJSON(),closeVisible:getComputedStyle(document.querySelector(".m1-layers-close")).display})')
assert compact['scroll']<=compact['width'] and compact['buttonHeight']>=44 and compact['closeVisible']!='none',compact
assert compact['panel']['bottom']<=compact['canvas']['bottom'] and compact['panel']['top']>=compact['canvas']['top'],compact
run('select','select[aria-label="Theme"]','dark');run('screenshot',str(out/'compact-dark.png'));run('click','.m1-layers-close');wait('document.activeElement.id==="m1-layers-trigger"')
run('set','viewport','1440','900');run('click','#m1-layers-trigger')
errors=run('errors');(out/'errors.json').write_text(json.dumps(errors,indent=2));assert not errors.get('errors'),errors
print('PASS default keycaps, five layers, footprint details/shared state, camera/history, cap selection, cap edit/Undo, workspace retention, desktop/compact themes and focus')
