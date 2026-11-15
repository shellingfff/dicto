"""Exercise the real Tauri/WebKit app with generated dictionaries.

Run after cargo build -p dicto:
  python3 tests/webview_smoke.py --binary target/debug/dicto
Uses a temporary config/data directory and never changes personal settings.
"""
import argparse
import base64
import json
import os
import shutil
import signal
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from make_dictionary_fixture import create

parser = argparse.ArgumentParser()
parser.add_argument('--binary', type=Path, required=True)
parser.add_argument('--first-run', action='store_true')
parser.add_argument('--screenshots', action='store_true', help='Capture screenshots (requires a visible, rendered window)')
parser.add_argument('--dictionary-root', type=Path, help='Inspect hello in existing OALECD dictionaries instead of fixtures')
parser.add_argument('--skip-audio', action='store_true', help='Skip playback when system codecs are unavailable')
parser.add_argument('--port', type=int, default=4444)
args = parser.parse_args()
root = Path(tempfile.mkdtemp(prefix='dicto-webview-'))
dictionary_root = args.dictionary_root.resolve() if args.dictionary_root else create(root / 'dictionaries')
config = root / 'config' / 'dicto'
config.mkdir(parents=True)
(config / 'settings.toml').write_text(f'dictionary_directories = {json.dumps([] if args.first_run else [str(dictionary_root)])}\ndictionaries = []\n')
env = os.environ | {'TAURI_WEBVIEW_AUTOMATION': 'true', 'XDG_CONFIG_HOME': str(root / 'config'), 'XDG_DATA_HOME': str(root / 'data'), 'XDG_CACHE_HOME': str(root / 'cache')}
log = open(root / 'webdriver.log', 'w')
process = subprocess.Popen(['WebKitWebDriver', f'--port={args.port}', '--host=127.0.0.1'], env=env, stdout=log, stderr=log, start_new_session=True)
base = f'http://127.0.0.1:{args.port}'
session = None

def call(method, path, body=None):
    request = urllib.request.Request(base + path, data=json.dumps(body).encode() if body is not None else None,
                                     headers={'Content-Type': 'application/json'}, method=method)
    try:
        with urllib.request.urlopen(request, timeout=45) as response:
            result = json.load(response)['value']
    except urllib.error.HTTPError as error:
        raise RuntimeError(error.read().decode()) from error
    if isinstance(result, dict) and 'error' in result:
        raise RuntimeError(result)
    return result

def wd(method, path, body=None):
    return call(method, f'/session/{session}{path}', body)

def js(script, *arguments):
    return wd('POST', '/execute/sync', {'script': script, 'args': list(arguments)})

def wait(predicate, label, seconds=30):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        value = predicate()
        if value:
            return value
        time.sleep(.2)
    raise AssertionError(f'Timed out: {label}')

def element(selector):
    return wd('POST', '/element', {'using': 'css selector', 'value': selector})

def click(selector):
    el = element(selector)
    key = el['element-6066-11e4-a52e-4f735466cecf']
    wd('POST', f'/element/{key}/click', {})

def press(key, control=False, shift=False):
    modifiers = (["\ue009"] if control else []) + (["\ue008"] if shift else [])
    actions = [{'type':'keyDown','value':value} for value in modifiers]
    actions += [{'type':'keyDown','value':key},{'type':'keyUp','value':key}]
    actions += [{'type':'keyUp','value':value} for value in reversed(modifiers)]
    wd('POST','/actions',{'actions':[{'type':'key','id':'keyboard','actions':actions}]})

def frame():
    wd('POST', '/frame', {'id': element('#definition')})

def parent():
    wd('POST', '/frame', {'id': None})

try:
    for _ in range(50):
        try:
            call('GET', '/status'); break
        except (OSError, RuntimeError):
            time.sleep(.1)
    result = call('POST', '/session', {'capabilities': {'alwaysMatch': {'browserName': 'wry', 'webkitgtk:browserOptions': {'binary': str(args.binary.resolve()), 'args': []}}}})
    session = result['sessionId']
    wait(lambda: js("return !!window.__TAURI__ && document.querySelector('#search') !== null"), 'app startup')
    if args.first_run:
        wait(lambda: js("return document.querySelector('#settings').open"), 'first-run setup')
        js("draft.dictionary_directories.push(arguments[0]);renderSettings();document.querySelector('#save').click()",str(dictionary_root))
        wait(lambda: js("return !document.querySelector('#settings').open"), 'first-run folder save')
    wait(lambda: js("return document.querySelector('#status').textContent === ''"), 'indexing')
    js("document.querySelector('#search').value='hello'; document.querySelector('#search').dispatchEvent(new Event('input',{bubbles:true}));")
    wait(lambda: js("return document.querySelectorAll('#tabs button').length===2"), 'two dictionaries')
    if args.dictionary_root:
        js("[...document.querySelectorAll('#tabs button')].find(b=>b.textContent.includes('牛津')).click()")
        frame()
        wait(lambda: js("return document.getElementsByTagName('xhtml:ol').length>=3"), 'real OALECD senses')
        assert js("return [...document.getElementsByTagName('xhtml:ol')].slice(0,3).map(n=>Number(n.getAttribute('start')))") == [1,2,3]
        assert js("return getComputedStyle(document.querySelector('xr-gs[firstinblock=n]')).display") == 'inline'
        assert js("return [...document.styleSheets].some(s=>s.href?.includes('oalecd9.css'))")
        assert js("return document.querySelectorAll('ol').length") == 0
        assert js("return getComputedStyle(document.getElementsByTagName('xhtml:ol')[0],'::before').content.includes('1')")
        assert js("return typeof toggle_infl") == 'function'
        assert js("try { return !!parent.document; } catch { return false; }") is False
        parent()
        if args.screenshots: (root / 'oalecd-hello.png').write_bytes(base64.b64decode(wd('GET','/screenshot')))
        print(json.dumps({'result':'real OALECD passed','artifacts':str(root)},indent=2),flush=True)
        raise SystemExit(0)
    js("document.querySelectorAll('#tabs button')[1].click()")
    frame()
    wait(lambda: js("return document.querySelector('#script-status')?.textContent==='Dictionary JavaScript loaded'"), 'dictionary JavaScript', seconds=8)
    assert js("return [...document.querySelectorAll('ol')].map(n=>n.start)") == [1,2,3]
    assert js("return getComputedStyle(document.querySelector('xr-gs')).display") == 'inline'
    assert js("return getComputedStyle(document.querySelector('def')).color") == 'rgb(12, 34, 56)'
    wait(lambda: js("return document.querySelector('#test-image').naturalWidth===80"), 'MDD image')
    wait(lambda: js("return document.querySelector('#root-image').naturalWidth===80"), 'root-relative MDD image')
    assert js("try { return !!parent.document; } catch { return false; }") is False
    assert js("return typeof window.__TAURI__ === 'undefined' && typeof window.__TAURI_INTERNALS__ === 'undefined'")
    if not args.skip_audio:
        js("window.__mediaTest={}; const play=HTMLMediaElement.prototype.play; HTMLMediaElement.prototype.play=function(){window.__mediaTest.element=this; return play.call(this).then(()=>window.__mediaTest.ok=true).catch(e=>{window.__mediaTest.error=String(e);throw e})}")
        click('#test-sound')
        wait(lambda: js("return window.__mediaTest.ok || window.__mediaTest.error"), 'audio playback')
        assert js("return window.__mediaTest.ok") is True
    parent()
    # Zoom changes only definition content and survives dictionary navigation.
    frame()
    original_image_width = js("return document.querySelector('#test-image').getBoundingClientRect().width")
    parent()
    click('#search')
    original_search_height = js("return document.querySelector('#search').getBoundingClientRect().height")
    press('=', control=True)
    frame()
    wait(lambda: js("return Number(getComputedStyle(document.documentElement).zoom) === 1.1"), 'zoom in')
    assert abs(js("return document.querySelector('#test-image').getBoundingClientRect().width") - original_image_width * 1.1) < 1
    click('#test-image')
    press('-', control=True)
    wait(lambda: js("return Number(getComputedStyle(document.documentElement).zoom) === 1"), 'zoom out from definition')
    parent()
    click('#search')
    press('=', control=True)
    assert js("return document.querySelector('#search').getBoundingClientRect().height") == original_search_height
    print('Rendering, zoom, resource isolation and audio passed', flush=True)
    if args.screenshots:
        (root / 'hello.png').write_bytes(base64.b64decode(wd('GET', '/screenshot')))
    # The definition frame forwards dictionary shortcuts to the shell.
    js("window.__keys=[];document.addEventListener('keydown',e=>window.__keys.push([e.key,e.code,e.keyCode,e.shiftKey,e.ctrlKey,e.target.tagName]),true)")
    frame()
    press("\ue004")
    parent()
    wait(lambda: js("return document.querySelector('#tabs button[aria-selected=true]').textContent==='Second Dictionary'"), 'Tab dictionary navigation')
    frame()
    wait(lambda: js("return Number(getComputedStyle(document.documentElement).zoom) === 1.1"), 'zoom after dictionary switch')
    parent()
    click('#search')
    press('0', control=True)
    frame()
    wait(lambda: js("return Number(getComputedStyle(document.documentElement).zoom) === 1"), 'reset zoom')
    parent()
    click("#search")
    press("\ue004",shift=True)
    wait(lambda: js("return document.querySelector('#tabs button[aria-selected=true]').textContent==='测试词典'"), 'Shift+Tab navigation')
    # Prefixes and typos must not silently substitute a different headword.
    for query in ['hell', 'hellp', 'missingword']:
        js("document.querySelector('#search').value=arguments[0];document.querySelector('#search').dispatchEvent(new Event('input',{bubbles:true}))", query)
        wait(lambda: js("return document.querySelector('#word').textContent===arguments[0] && document.querySelector('#status').textContent===''", query), 'exact missing lookup')
        assert js("return !document.querySelector('#empty').hidden && document.querySelector('#empty').textContent==='No definition found.' && document.querySelector('#definition').hidden && document.querySelectorAll('#tabs button').length===0")
        press("\ue007")
        wait(lambda: js("return document.querySelector('#status').textContent===''"), 'exact Enter lookup')
        assert js("return document.querySelector('#word').textContent===arguments[0] && document.querySelector('#definition').hidden", query)
    js("document.querySelector('#search').value='plum';document.querySelector('#search').dispatchEvent(new Event('input',{bubbles:true}))")
    wait(lambda: js("return document.querySelector('#word').textContent==='plum' && document.querySelectorAll('#tabs button').length===1"), 'duplicate headword')
    frame()
    wait(lambda: js("return document.body.textContent.includes('A purple fruit.')"), 'original definition wins over later alias')
    assert js("return !document.body.textContent.includes('Measure depth.')")
    parent()
    js("document.querySelector('#search').value='HELLO';document.querySelector('#search').dispatchEvent(new Event('input',{bubbles:true}))")
    wait(lambda: js("return document.querySelector('#word').textContent==='HELLO' && document.querySelectorAll('#tabs button').length===2"), 'case-insensitive exact match')
    js("document.querySelectorAll('#tabs button')[1].click()")
    frame()
    click('a[href="entry://greeting"]')
    parent()
    wait(lambda: js("return document.querySelector('#word').textContent==='greeting'"), 'entry link lookup')
    assert js("return document.querySelectorAll('#tabs button').length") == 1
    assert js("return getComputedStyle(document.querySelector('#tabs')).display") != 'none'
    frame()
    wait(lambda: js("return document.body.textContent.includes('A friendly welcome.')"), 'linked definition loaded')
    click('p')
    press("f",control=True)
    parent()
    wait(lambda: js("let s=document.querySelector('#search');return document.activeElement===s && s.selectionStart===0 && s.selectionEnd===s.value.length"), 'focus search from definition')
    click('#settings-open')
    wait(lambda: js("return document.querySelector('#settings').open"), 'settings')
    assert js("return document.querySelectorAll('.dictionary').length") == 2
    js("document.querySelector('.dictionary input[type=checkbox]').click(); document.querySelector('button[value=cancel]').click()")
    click('#settings-open')
    wait(lambda: js("return document.querySelector('#settings').open"), 'reopened settings')
    assert js("return document.querySelector('.dictionary input[type=checkbox]').checked")
    # Reorder through the actual settings controls; names are automatic.
    assert js("return document.querySelectorAll('.dictionary input[type=text]').length") == 0
    js("document.querySelectorAll('.dictionary button')[1].click();document.querySelector('#save').click()")
    wait(lambda: js("return !document.querySelector('#settings').open"), 'reorder save')
    wait(lambda: js("return document.querySelector('#tabs button')?.textContent==='测试词典'"), 'automatic tab name after reorder')
    click('#settings-open')
    wait(lambda: js("return document.querySelector('#settings').open"), 'settings after reorder')
    js("document.querySelector('.dictionary input[type=checkbox]').click();document.querySelector('#save').click()")
    wait(lambda: js("return !document.querySelector('#settings').open"), 'settings save')
    assert 'enabled = false' in (config / 'settings.toml').read_text()
    assert list((root / 'dictionaries').rglob('*.fst')), 'indexes must live beside source files'
    assert not list(config.rglob('*.mdx')), 'source dictionaries must not be copied'
    print('Navigation and settings passed',flush=True)
    if shutil.which('hyprctl') and os.environ.get('HYPRLAND_INSTANCE_SIGNATURE'):
        def clients():
            return json.loads(subprocess.check_output(['hyprctl','clients','-j'],text=True))
        candidates=[]
        for client in clients():
            try:
                if Path(os.readlink(f"/proc/{client['pid']}/exe")) == args.binary.resolve(): candidates.append(client)
            except OSError: pass
        assert candidates, 'test window is registered with Hyprland'
        client=max(candidates,key=lambda c:c['pid'])
        assert client['class']=='io.github.shellingfff.dicto', f"Desktop icon identity mismatch: {client['class']}"
        pid=client['pid']
        closed=subprocess.run(['hyprctl','dispatch','closewindow',f'address:{client["address"]}'],capture_output=True,text=True)
        if closed.returncode and 'dispatch in lua' in closed.stdout + closed.stderr:
            closed=subprocess.run(['hyprctl','dispatch',f'hl.dsp.window.close({{ window = "address:{client["address"]}" }})'],capture_output=True,text=True)
        assert closed.returncode==0, closed.stdout + closed.stderr
        wait(lambda:not any(c['pid']==pid and c.get('mapped',True) for c in clients()),'close to tray')
        os.kill(pid,0)
        subprocess.run([str(args.binary.resolve())],env=env,check=True,timeout=10,capture_output=True)
        wait(lambda:any(c['pid']==pid and c.get('mapped',True) for c in clients()),'single-instance reopen')
        wait(lambda:js("let s=document.querySelector('#search');return document.activeElement===s && s.selectionStart===0 && s.selectionEnd===s.value.length"),'select search on native focus')
    print(json.dumps({'result':'passed','artifacts':str(root)}, indent=2), flush=True)
except Exception:
    if session:
        try:
            print('Failure document:', js("return {url:location.href, text:document.body.innerText.slice(0,2000),keys:window.__keys,focus:document.activeElement.tagName, resources:performance.getEntriesByType('resource').map(r=>({name:r.name,size:r.transferSize}))}"), flush=True)
            if args.screenshots: (root / 'failure.png').write_bytes(base64.b64decode(wd('GET','/screenshot')))
        except Exception as error: print('Diagnostics:', error, flush=True)
    raise
finally:
    if session:
        try: call('DELETE', f'/session/{session}')
        except Exception: pass
    # Closing a window intentionally hides it; explicitly stop only apps using
    # this test's temporary configuration, including after a failed assertion.
    for entry in Path('/proc').iterdir():
        if not entry.name.isdigit(): continue
        try:
            if Path(os.readlink(entry / 'exe')) != args.binary.resolve(): continue
            if f'XDG_CONFIG_HOME={root / "config"}'.encode() in (entry / 'environ').read_bytes().split(b'\0'):
                os.kill(int(entry.name), signal.SIGTERM)
        except (OSError, PermissionError): pass
    process.terminate()
    try: process.wait(timeout=5)
    except subprocess.TimeoutExpired: process.kill()
    log.close()
    print('Artifacts:', root, flush=True)
