// Dictionary code runs only inside a sandboxed frame, without native IPC.
(() => {
  const send = (type, value) => parent.postMessage({type, value}, '*');
  // Explicit entry styles take precedence over legacy dictionary-wide fallback
  // sheets. Let the browser identify links rather than parsing HTML in Rust.
  const removeFallbacks = () => {
    if (!document.querySelector('link[rel~="stylesheet"]:not([data-dicto-fallback])')) return;
    document.querySelectorAll('link[data-dicto-fallback]').forEach(link => link.remove());
    stylesObserver.disconnect();
  };
  const stylesObserver = new MutationObserver(removeFallbacks);
  stylesObserver.observe(document.documentElement, {childList:true, subtree:true});
  let audio, audioUrl, audioRequest = 0;
  async function playAudio(path) {
    const request = ++audioRequest;
    if (audio) audio.pause();
    const response = await fetch(new URL(path.replaceAll('\\', '/').replace(/^\/+/, ''), location.href));
    if (!response.ok) throw new Error('Audio resource not found');
    const blob = await response.blob();
    if (request !== audioRequest) return;
    if (audioUrl) URL.revokeObjectURL(audioUrl);
    audioUrl = URL.createObjectURL(blob);
    audio = new Audio(audioUrl);
    await audio.play();
  }
  document.addEventListener('click', event => {
    const anchor = event.target.closest('a, xhtml\\:a');
    if (!anchor) return;
    const href = anchor.getAttribute('href') || '';
    if (/^(entry:\/\/|d:)/i.test(href)) {
      event.preventDefault(); send('lookup', decodeURIComponent(href.replace(/^(entry:\/\/|d:)/i, '')));
    } else if (/^sound:\/\//i.test(href)) {
      event.preventDefault();
      // WebKit's media pipeline cannot stream custom URI schemes directly.
      playAudio(href.slice(8)).catch(() => send('error', 'This audio resource could not be played.'));
    } else if (!href.startsWith('#')) {
      event.preventDefault();
    }
  });
  document.addEventListener('keydown', event => {
    const key = event.code === 'Tab' || event.keyCode === 9 ? 'Tab' : event.key;
    if (key === 'Tab' || (event.ctrlKey && ['n','p','l','f'].includes(event.key.toLowerCase())) || event.key === 'Escape') {
      event.preventDefault(); send('key', {key, ctrlKey:event.ctrlKey, shiftKey:event.shiftKey});
    }
  });
  document.addEventListener('DOMContentLoaded', () => {
    removeFallbacks(); stylesObserver.disconnect();
    // Preserve dictionary-specific XHTML tags and their CSS. In particular,
    // OALECD generates sense labels with xhtml\:ol::before; making these native
    // lists would add a second number and change the layout.
    const generated = node => ['::before', '::after'].some(pseudo => {
      const content = getComputedStyle(node, pseudo).content;
      return content && !['none', 'normal', '""', "''"].includes(content);
    });
    const nativeLists = new Set([...document.querySelectorAll('xhtml\\:ol, xhtml\\:ul')]
      .filter(list => !generated(list) && ![...list.children].some(generated)));
    for (const node of [...document.querySelectorAll('*')]) {
      const tag = node.localName;
      const list = tag === 'xhtml:li' ? node.parentElement : node;
      if (tag !== 'xhtml:br' && !(['xhtml:ol', 'xhtml:ul', 'xhtml:li'].includes(tag) && nativeLists.has(list))) continue;
      const hidden = getComputedStyle(node).display === 'none';
      const element = document.createElement(tag.slice(6));
      for (const attr of node.attributes) element.setAttribute(attr.name, attr.value);
      if (hidden) element.style.display = 'none';
      element.append(...node.childNodes); node.replaceWith(element);
      // Children now reference the native parent rather than the replaced node.
      if (nativeLists.has(node)) nativeLists.add(element);
    }
  });
})();
