'use strict';
const invoke = window.__TAURI__.core.invoke;
const $ = id => document.getElementById(id);
let suggestions = [], selected = -1, hits = [], active = 0, currentWord = '', revision = 0, timer, draft;
let generation = -1;
const report = error => { $('status').textContent = String(error); };
const focusSearch = () => { if (!$('settings').open) { $('search').focus(); $('search').select(); } };

async function lookup(word) {
  const request = ++revision;
  const previousDictionary = word === currentWord ? hits[active]?.id : null;
  $('status').textContent = 'Looking up…';
  try {
    const result = await invoke('lookup', {word});
    if (request !== revision) return;
    currentWord = word; hits = result;
    active = Math.max(0, hits.findIndex(hit => hit.id === previousDictionary));
    $('status').textContent = ''; renderResult();
  } catch (error) { if (request === revision) report(error); }
}
function renderResult() {
  $('word').textContent = currentWord;
  $('tabs').replaceChildren(...hits.map((hit, index) => {
    const button = document.createElement('button');
    button.textContent = hit.short_name; button.title = hit.name;
    button.setAttribute('aria-selected', index === active);
    button.onclick = () => { active = index; renderResult(); };
    return button;
  }));
  $('empty').hidden = hits.length > 0;
  $('empty').textContent = currentWord ? 'No definition found.' : 'Type a word to look it up.';
  $('definition').hidden = !hits.length;
  if (hits.length) {
    const origin = new URL(window.__TAURI__.core.convertFileSrc('', 'dict'));
    origin.hostname = origin.protocol === 'dict:' ? `${hits[active].id}.localhost` : `dict.${hits[active].id}.localhost`;
    const root = `${origin.href}__entry`;
    const url = `${root}?word=${encodeURIComponent(currentWord)}`;
    if ($('definition').src !== url) $('definition').src = url;
  } else $('definition').removeAttribute('src');
}
async function search(preserveWord = false) {
  const query = $('search').value.trim();
  const request = ++revision;
  if (!query) { $('status').textContent=''; suggestions=[]; selected=-1; hits=[]; currentWord=''; renderResult(); return; }
  try {
    const result = await invoke('search', {query});
    if (request !== revision) return;
    suggestions = result;
    selected = result.length ? (preserveWord ? Math.max(0, result.indexOf(currentWord)) : 0) : -1;
    await lookup(result[selected] || query);
  } catch (error) { if (request === revision) report(error); }
}
function navigate(delta) {
  if (!suggestions.length) return;
  selected = (selected + delta + suggestions.length) % suggestions.length;
  lookup(suggestions[selected]);
}
function keyboard(event) {
  if ($('settings').open) return;
  const key = event.code === 'Tab' || event.keyCode === 9 ? 'tab' : event.key.toLowerCase();
  if (key === 'tab' && hits.length) {
    event.preventDefault?.(); active = (active + (event.shiftKey ? -1 : 1) + hits.length) % hits.length; renderResult(); focusSearch();
  } else if (event.ctrlKey && ['n','p'].includes(key)) {
    event.preventDefault?.(); navigate(key === 'n' ? 1 : -1);
  } else if (event.ctrlKey && ['l','f'].includes(key)) {
    event.preventDefault?.(); focusSearch();
  } else if (key === 'escape') {
    $('search').value = ''; search(); focusSearch();
  } else if (event.target === $('search') && ['arrowdown','arrowup'].includes(key)) {
    event.preventDefault(); navigate(key === 'arrowdown' ? 1 : -1);
  }
}
$('search').addEventListener('input', () => { ++revision; clearTimeout(timer); timer = setTimeout(search, 150); });
$('search').addEventListener('keydown', event => { if (event.key === 'Enter') { clearTimeout(timer); lookup($('search').value.trim()); } });
document.addEventListener('keydown', keyboard);
window.addEventListener('focus', focusSearch);
window.addEventListener('message', event => {
  if (event.source !== $('definition').contentWindow || !event.data) return;
  const {type, value} = event.data;
  if (type === 'lookup' && typeof value === 'string' && value.length < 1024) { $('search').value = value; search(); }
  else if (type === 'key' && value && typeof value.key === 'string') keyboard(value);
  else if (type === 'error' && typeof value === 'string') report(value);
});
function button(text, click) { const b=document.createElement('button'); b.type='button'; b.textContent=text; b.onclick=click; return b; }
function renderSettings() {
  $('folders').replaceChildren(...draft.dictionary_directories.map((path, i) => {
    const row=document.createElement('div'); row.className='folder';
    const name=document.createElement('span'); name.textContent=path;
    row.append(name,button('Remove',()=>{draft.dictionary_directories.splice(i,1);renderSettings();})); return row;
  }));
  $('dictionary-list').replaceChildren(...draft.dictionaries.map((dict,i) => {
    const row=document.createElement('div'); row.className='dictionary';
    const enabled=document.createElement('input'); enabled.type='checkbox'; enabled.checked=dict.enabled; enabled.setAttribute('aria-label',`Enable ${dict.path}`); enabled.onchange=()=>dict.enabled=enabled.checked;
    const label=document.createElement('label'); label.textContent=draft.titles?.[dict.path] || dict.path.split(/[\\/]/).pop().replace(/\.mdx$/i,'');
    const path=document.createElement('small'); path.textContent=dict.path;
    const name=document.createElement('input'); name.type='text'; name.value=dict.short_name; name.placeholder='Automatic tab name'; name.setAttribute('aria-label','Tab name'); name.oninput=()=>dict.short_name=name.value;
    label.append(path,name);
    const move=delta=>{ const j=i+delta; if(j<0||j>=draft.dictionaries.length)return; [draft.dictionaries[i],draft.dictionaries[j]]=[draft.dictionaries[j],draft.dictionaries[i]]; renderSettings(); };
    const up=button('↑',()=>move(-1)), down=button('↓',()=>move(1)); up.disabled=i===0;down.disabled=i===draft.dictionaries.length-1;
    row.append(enabled,label,up,down);return row;
  }));
}
async function openSettings() {
  try { draft=await invoke('get_settings'); renderSettings(); $('settings-error').textContent=''; if (!$('settings').open) $('settings').showModal(); }
  catch(error){report(error);}
}
$('settings-open').onclick=openSettings;
$('add-folder').onclick=async()=>{
  try { const path=await invoke('choose_folder'); if(path&&!draft.dictionary_directories.includes(path)){draft.dictionary_directories.push(path);renderSettings();} }
  catch(error){$('settings-error').textContent=String(error);}
};
async function save(close) {
  $('save').disabled=true;
  try { await invoke('save_settings',{settings:draft}); if(close)$('settings').close(); else {draft=await invoke('get_settings');renderSettings();} }
  catch(error){$('settings-error').textContent=String(error);}
  finally{$('save').disabled=false;}
}
$('save').onclick=()=>save(true); $('rescan').onclick=()=>save(false);
$('settings').addEventListener('close',focusSearch);
async function poll() {
  try {
    const progress=await invoke('status');
    if(progress.busy) $('status').textContent=`Indexing ${progress.done}/${progress.total} · ${progress.current.split(/[\\/]/).pop() || 'Preparing…'}`;
    else if(progress.generation!==generation) {
      generation=progress.generation;
      $('status').textContent=progress.errors.join('\n');
      if($('search').value.trim()) await search(true);
      const settings=await invoke('get_settings');
      if(!settings.dictionaries.length&&!$('settings').open) await openSettings();
    }
  } catch(error){report(error);}
  setTimeout(poll,700);
}
poll();
