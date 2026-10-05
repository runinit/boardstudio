#!/usr/bin/env python3
"""One bounded public adopted-path journey on the default pnpm-start app."""
from __future__ import annotations
import argparse, datetime as dt, hashlib, json, subprocess, time, zipfile
from pathlib import Path

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--url',required=True)
    p.add_argument('--session',default='f97-adoption-20261005')
    p.add_argument('--fixture',type=Path,required=True)
    p.add_argument('--output-dir',type=Path,required=True)
    a=p.parse_args(); out=a.output_dir.resolve(); out.mkdir(parents=True,exist_ok=True)
    if any(out.iterdir()): raise SystemExit(f'output directory is not empty: {out}')
    fixture=a.fixture.resolve(); exported=out/'renamed-project-copy.boardstudio'; steps=[]
    report={'status':'running','url':a.url,'session':a.session,'fixture':str(fixture),
            'fixture_sha256':hashlib.sha256(fixture.read_bytes()).hexdigest(),
            'no_screenshots':True,'phases':[]}
    def run(*args, timeout=150):
        command=['agent-browser','--session',a.session,*args,'--json']
        try:
            r=subprocess.run(command,text=True,capture_output=True,timeout=timeout)
            item={'command':command,'exit':r.returncode,'stdout':r.stdout,'stderr':r.stderr,
                  'time_utc':dt.datetime.now(dt.timezone.utc).isoformat()}
        except subprocess.TimeoutExpired as exc:
            item={'command':command,'exit':'timeout','stdout':exc.stdout or '',
                  'stderr':exc.stderr or '', 'time_utc':dt.datetime.now(dt.timezone.utc).isoformat()}
            steps.append(item); save_steps(); raise RuntimeError(f'command timed out: {args[0]}')
        steps.append(item); save_steps()
        try: result=json.loads(r.stdout) if r.stdout.strip() else {}
        except json.JSONDecodeError: result={'unparsed_stdout':r.stdout}
        if r.returncode or not result.get('success',True):
            raise RuntimeError(f'{args[0]} failed: {result} {r.stderr[-500:]}')
        return result.get('data',result)
    def save_steps():
        (out/'browser-steps.json').write_text(json.dumps(steps,indent=2)+'\n',encoding='utf-8')
    def evaluate(expr):
        wrapped=f'(async()=>JSON.stringify(await ({expr})))()'
        result=run('eval',wrapped)
        value=result.get('result',result)
        return json.loads(value) if isinstance(value,str) else value
    def wait(predicate,timeout=120000):
        return run('wait','--fn',f'Boolean({predicate})','--timeout',str(timeout))
    def click(selector): return run('click',selector)
    def upload(selector,path): return run('upload',selector,str(path))
    def expected_name(name): return json.dumps(name)
    failure=None
    try:
        run('open',a.url)
        wait("document.querySelector('.m1-workbench') || document.querySelector('.m1-library-landing')")
        wait("navigator.serviceWorker.controller && new URL(navigator.serviceWorker.controller.scriptURL).pathname.replace(/\\/$/,'') === '/service-worker.js'")
        worker=evaluate("(async()=>{const rs=await navigator.serviceWorker.getRegistrations();const r=rs.find(x=>x.scope===location.origin+'/');return {controller:navigator.serviceWorker.controller?.scriptURL??null,registration:r?{scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null}:null}})()")
        if not worker['registration'] or not (worker['registration']['active'] or '').endswith('/service-worker.js') or worker['registration']['waiting'] or worker['registration']['installing']:
            raise RuntimeError(f'ordinary M1 worker was not settled: {worker}')
        report['phases'].append({'m1_shell_and_ordinary_worker':worker})

        wait("Boolean(document.querySelector(\"input[type=file][accept='.boardstudio']\"))")
        upload("input[type='file'][accept='.boardstudio']",fixture)
        wait("document.querySelector('.m1-project-name')?.textContent.trim() === 'Sofle v2'")
        wait("document.querySelector('.m1-save-state[data-state=\"saved\"]')")
        before=evaluate("({name:document.querySelector('.m1-project-name')?.textContent.trim()??null,active:localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root'),saved:document.querySelector('.m1-save-state')?.getAttribute('data-state')??null,revision:document.querySelector('.m1-project-current-status')?.textContent??null})")
        report['phases'].append({'fixture_import_saved':before})
        if before['active']!='m1-sofle-v2-copy' or before['name']!='Sofle v2': raise RuntimeError(f'fixture import did not select expected project: {before}')

        menu=evaluate("({exists:!!document.querySelector('details.m1-project-menu'),open:document.querySelector('details.m1-project-menu')?.open??false})")
        if not menu['exists']: raise RuntimeError(f'project menu details missing: {menu}')
        if not menu['open']:
            click('details.m1-project-menu > summary')
            wait("document.querySelector('details.m1-project-menu')?.open === true")
        wait("Boolean(document.querySelector('input[aria-label=\"Project name\"]'))")
        run('fill','input[aria-label="Project name"]','Sofle v2 F9.7 adoption')
        run('press','Enter')
        wait("document.querySelector('input[aria-label=\"Project name\"]')?.value === 'Sofle v2 F9.7 adoption'")
        # The rename is committed on Enter-triggered blur; durability is verified
        # by saved state, reload, and portable archive fields below.
        wait("document.querySelector('.m1-save-state[data-state=\"saved\"]')")
        wait("document.querySelector('.m1-project-name')?.textContent.trim() === 'Sofle v2 F9.7 adoption'")
        renamed=evaluate("({name:document.querySelector('.m1-project-name')?.textContent.trim()??null,active:localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root'),saved:document.querySelector('.m1-save-state')?.getAttribute('data-state')??null,revision:document.querySelector('.m1-project-current-status')?.textContent??null})")
        report['phases'].append({'rename_committed':renamed})
        if renamed['name']!='Sofle v2 F9.7 adoption' or renamed['active']!=before['active']:
            raise RuntimeError(f'rename did not update the selected project: {renamed}')
        menu=evaluate("({open:document.querySelector('details.m1-project-menu')?.open??false})")
        if menu['open']:
            click('details.m1-project-menu > summary')
            wait("document.querySelector('details.m1-project-menu')?.open === false")
        run('reload')
        wait("document.querySelector('.m1-project-name')?.textContent.trim() === 'Sofle v2 F9.7 adoption' && document.querySelector('.m1-save-state[data-state=\"saved\"]') && localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root') === 'm1-sofle-v2-copy'")
        persisted=evaluate("({url:location.href,name:document.querySelector('.m1-project-name')?.textContent.trim()??null,active:localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root'),saveState:document.querySelector('.m1-save-state')?.getAttribute('data-state')??null,controller:navigator.serviceWorker.controller?.scriptURL??null})")
        report['phases'].append({'rename_reload_readback':persisted})

        tabs=['Layout','PCB','Keymap','Keycaps','Case','Parts']
        for tab in tabs:
            selector=f'#m1-tab-{tab}'
            click(selector)
            wait(f"document.querySelector({json.dumps(selector)})?.getAttribute('aria-selected') === 'true'")
            observed=evaluate(f"({{tab:{json.dumps(tab)},selected:document.querySelector({json.dumps(selector)})?.getAttribute('aria-selected')??null,panel:document.querySelector('#m1-workspace-panel')?.getAttribute('aria-labelledby')??null}})")
            if observed['selected']!='true' or observed['panel']!=selector[1:]:
                raise RuntimeError(f'workbench tab did not select its panel: {observed}')
            report['phases'].append({'workbench_navigation':observed})

        menu=evaluate("({open:document.querySelector('details.m1-project-menu')?.open??false})")
        if not menu['open']:
            click('details.m1-project-menu > summary')
            wait("document.querySelector('details.m1-project-menu')?.open === true")
        hit=evaluate("(()=>{const b=document.querySelector('button.m1-project-copy-action');if(!b)return null;const r=b.getBoundingClientRect(),t=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return {detailsOpen:document.querySelector('details.m1-project-menu')?.open??false,disabled:b.disabled,hitIsCopy:t===b||b.contains(t),center:{x:r.x+r.width/2,y:r.y+r.height/2}}})()")
        if not hit or not hit['detailsOpen'] or hit['disabled'] or not hit['hitIsCopy']:
            raise RuntimeError(f'portable copy control was not hit-testable: {hit}')
        report['phases'].append({'portable_copy_button_hit_test':hit})
        run('download','button.m1-project-copy-action',str(exported))
        with zipfile.ZipFile(fixture) as source, zipfile.ZipFile(exported) as copy:
            before_doc=json.loads(source.read('project.json')); after_doc=json.loads(copy.read('project.json'))
            before_cmp=dict(before_doc); after_cmp=dict(after_doc)
            before_cmp.pop('name',None); after_cmp.pop('name',None)
            before_cmp.pop('revision',None); after_cmp.pop('revision',None)
            if before_cmp!=after_cmp: raise RuntimeError('portable copy changed project fields beyond name/revision')
            if after_doc.get('id')!='m1-sofle-v2-copy' or after_doc.get('name')!='Sofle v2 F9.7 adoption':
                raise RuntimeError(f'portable copy identity/name mismatch: {after_doc.get("id")}, {after_doc.get("name")}')
            before_assets={name:hashlib.sha256(source.read(name)).hexdigest() for name in source.namelist() if name.startswith('assets/')}
            after_assets={name:hashlib.sha256(copy.read(name)).hexdigest() for name in copy.namelist() if name.startswith('assets/')}
            if before_assets!=after_assets: raise RuntimeError('portable copy assets differ from imported source')
        report['phases'].append({'portable_copy_comparison':{'source_project':{k:before_doc.get(k) for k in ('id','name','revision','format')},'exported_project':{k:after_doc.get(k) for k in ('id','name','revision','format')},'same_non_name_revision_fields':True,'asset_paths_equal':set(before_assets)==set(after_assets),'asset_hashes_equal':before_assets==after_assets,'asset_count':len(after_assets),'export_sha256':hashlib.sha256(exported.read_bytes()).hexdigest()}})
        wait("document.querySelector('.m1-save-state[data-state=\"saved\"]')")
        run('reload')
        wait("document.querySelector('.m1-project-name')?.textContent.trim() === 'Sofle v2 F9.7 adoption' && document.querySelector('.m1-save-state[data-state=\"saved\"]')")
        final=evaluate("({url:location.href,title:document.title,name:document.querySelector('.m1-project-name')?.textContent.trim()??null,active:localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root'),saveState:document.querySelector('.m1-save-state')?.getAttribute('data-state')??null,controller:navigator.serviceWorker.controller?.scriptURL??null,registrations:(await navigator.serviceWorker.getRegistrations()).map(r=>({scope:r.scope,active:r.active?.scriptURL??null,waiting:r.waiting?.scriptURL??null,installing:r.installing?.scriptURL??null}))})")
        report['phases'].append({'final_reload_readback':final})
        errors=run('errors'); console=run('console')
        report['console_errors']=errors
        report['console_log']=console
        if errors.get('errors') or errors.get('data'):
            # `agent-browser errors` returns errors under `data` on success;
            # preserve it for review and fail only on actual JS exceptions.
            error_payload=errors.get('errors',errors.get('data'))
            if error_payload: raise RuntimeError(f'page JavaScript errors after adopted-path journey: {error_payload}')
        report['status']='completed-adopted-path-public-journey'
    except Exception as exc:
        failure=repr(exc); report['status']='stopped-at-first-failed-boundary'; report['error']=failure
        try:
            report['failure_state']=evaluate("({url:location.href,title:document.title,projectName:document.querySelector('.m1-project-name')?.textContent.trim()??null,active:localStorage.getItem('boardstudio-m1-active-project:boardstudio-m1-root'),saveState:document.querySelector('.m1-save-state')?.getAttribute('data-state')??null,controller:navigator.serviceWorker.controller?.scriptURL??null})")
        except Exception as diag: report['failure_state_error']=repr(diag)
    finally:
        report['browser_session_closed']=False
        report['exported_archive']=str(exported) if exported.exists() else None
        (out/'report.json').write_text(json.dumps(report,indent=2,sort_keys=True)+'\n',encoding='utf-8')
    return 1 if failure else 0

if __name__=='__main__': raise SystemExit(main())
