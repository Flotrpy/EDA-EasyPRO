//! Browser glue that is far shorter in JavaScript: IndexedDB, file
//! pickers and downloads. Everything else lives in Rust.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
const DB = 'easypro';
function db() {
  return new Promise((resolve, reject) => {
    const r = indexedDB.open(DB, 1);
    r.onupgradeneeded = () => {
      const d = r.result;
      for (const s of ['projects', 'meta', 'snapshots']) if (!d.objectStoreNames.contains(s)) d.createObjectStore(s);
    };
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error);
  });
}
function tx(store, mode, fn) {
  return db().then((d) => new Promise((resolve, reject) => {
    const t = d.transaction(store, mode);
    const s = t.objectStore(store);
    const out = fn(s);
    t.oncomplete = () => resolve(out && out.result !== undefined ? out.result : null);
    t.onerror = () => reject(t.error);
  }));
}
export function idb_put(store, key, value) { return tx(store, 'readwrite', (s) => s.put(value, key)); }
export function idb_get(store, key) { return tx(store, 'readonly', (s) => s.get(key)); }
export function idb_delete(store, key) { return tx(store, 'readwrite', (s) => s.delete(key)); }
export function idb_all(store) {
  return db().then((d) => new Promise((resolve, reject) => {
    const out = [];
    const t = d.transaction(store, 'readonly');
    const req = t.objectStore(store).openCursor();
    req.onsuccess = () => {
      const c = req.result;
      if (c) { out.push([String(c.key), c.value]); c.continue(); }
    };
    t.oncomplete = () => resolve(out);
    t.onerror = () => reject(t.error);
  }));
}
export function download(name, bytes, mime) {
  const blob = new Blob([bytes], { type: mime });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = name;
  document.body.appendChild(a);
  a.click();
  setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 1000);
}
let handles = {};
export async function save_text(key, name, text) {
  if (window.showSaveFilePicker) {
    try {
      let h = handles[key];
      if (!h) {
        h = await window.showSaveFilePicker({ suggestedName: name, types: [{ description: 'EasyPRO project', accept: { 'application/json': ['.json'] } }] });
        handles[key] = h;
      }
      const w = await h.createWritable();
      await w.write(text);
      await w.close();
      return h.name;
    } catch (e) {
      if (e && e.name === 'AbortError') return null;
    }
  }
  download(name, new TextEncoder().encode(text), 'application/json');
  return name;
}
export function forget_handle(key) { delete handles[key]; }
export function pick_file(accept) {
  return new Promise((resolve) => {
    const i = document.createElement('input');
    i.type = 'file';
    i.accept = accept;
    i.onchange = async () => {
      const f = i.files && i.files[0];
      if (!f) return resolve(null);
      resolve([f.name, await f.text()]);
    };
    i.addEventListener('cancel', () => resolve(null));
    i.click();
  });
}
export function register_sw() {
  if ('serviceWorker' in navigator && location.protocol === 'https:') {
    navigator.serviceWorker.register('sw.js').catch(() => {});
  }
}
export function now_iso() { return new Date().toISOString(); }
export function local_time(iso) { try { return new Date(iso).toLocaleString(); } catch (e) { return iso; } }
export function read_clipboard() {
  if (navigator.clipboard && navigator.clipboard.readText) return navigator.clipboard.readText().catch(() => '');
  return Promise.resolve('');
}
export function print_html(html) {
  const w = window.open('', '_blank');
  if (!w) return false;
  w.document.open();
  w.document.write(html);
  w.document.close();
  w.addEventListener('load', () => { w.focus(); w.print(); });
  setTimeout(() => { try { w.focus(); w.print(); } catch (e) {} }, 600);
  return true;
}
export function copy_text(t) { if (navigator.clipboard) navigator.clipboard.writeText(t).catch(() => {}); }
export function focus_canvas() { const c = document.querySelector('.editor canvas'); if (c) c.focus(); }
"#)]
extern "C" {
    pub fn idb_put(store: &str, key: &str, value: &str) -> js_sys::Promise;
    pub fn idb_get(store: &str, key: &str) -> js_sys::Promise;
    pub fn idb_delete(store: &str, key: &str) -> js_sys::Promise;
    pub fn idb_all(store: &str) -> js_sys::Promise;
    pub fn download(name: &str, bytes: &[u8], mime: &str);
    pub fn save_text(key: &str, name: &str, text: &str) -> js_sys::Promise;
    pub fn forget_handle(key: &str);
    pub fn pick_file(accept: &str) -> js_sys::Promise;
    pub fn register_sw();
    pub fn now_iso() -> String;
    pub fn local_time(iso: &str) -> String;
    pub fn copy_text(text: &str);
    pub fn print_html(html: &str) -> bool;
    pub fn read_clipboard() -> js_sys::Promise;
    pub fn focus_canvas();
}
