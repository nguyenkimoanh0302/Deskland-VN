// Kiểm tra sổ đăng ký chủ đề trước khi gửi PR / build bản phát hành.
//   node tools/check-themes.mjs
// Báo LỖI (thoát mã 1) khi thiếu thứ khiến chủ đề hỏng; CẢNH BÁO khi thiếu thứ chỉ làm chủ đề kém hoàn chỉnh.
import { readFileSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const proto = join(root, 'prototype');
const errors = [], warnings = [];
const err = m => errors.push(m), warn = m => warnings.push(m);

const readJson = p => { try { return JSON.parse(readFileSync(p, 'utf8')); } catch (e) { err(`${p}: JSON không hợp lệ (${e.message})`); return null; } };
const reg = readJson(join(proto, 'themes.json'));
const i18n = { vi: readJson(join(proto, 'i18n', 'vi.json')), en: readJson(join(proto, 'i18n', 'en.json')) };
const thumbsData = existsSync(join(proto, 'thumbs-data.js')) ? readFileSync(join(proto, 'thumbs-data.js'), 'utf8') : '';
const mainRs = existsSync(join(root, 'deskland-rs', 'src', 'main.rs')) ? readFileSync(join(root, 'deskland-rs', 'src', 'main.rs'), 'utf8') : '';

if (reg) {
  const ids = new Set();
  for (const t of reg.themes || []) {
    const where = `chủ đề "${t.id}"`;
    if (!t.id || !/^[a-z0-9-]+$/.test(t.id)) err(`${where}: id phải gồm chữ thường, số, dấu gạch ngang`);
    if (ids.has(t.id)) err(`${where}: trùng id`);
    ids.add(t.id);
    for (const k of ['name', 'desc', 'page', 'status']) if (!t[k]) err(`${where}: thiếu "${k}"`);
    if (t.page && !existsSync(join(proto, t.page))) err(`${where}: không thấy trang prototype/${t.page}`);
    const thumb = String(t.thumb || '').match(/url\(([^)?]+)/);
    if (thumb && !existsSync(join(proto, thumb[1]))) err(`${where}: không thấy ảnh thu nhỏ prototype/${thumb[1]}`);
    if (!thumb && t.status === 'ready' && !thumbsData.includes(`"${t.id}":"data:`)) warn(`${where}: chưa có ảnh thu nhỏ (thumbs/<id>.jpg, 1024x576)`);
    const keys = new Set();
    for (const g of t.groups || []) {
      if (!g.name) err(`${where}: nhóm thông số thiếu "name"`);
      for (const p of g.params || []) {
        if (!p.key) { err(`${where}: thông số thiếu "key"`); continue; }
        if (keys.has(p.key)) err(`${where}: trùng thông số "${p.key}"`);
        keys.add(p.key);
        if (p.default === undefined) err(`${where}.${p.key}: thiếu "default"`);
        if (p.options) {
          if (!Array.isArray(p.options) || !p.options.length) err(`${where}.${p.key}: "options" phải là mảng không rỗng`);
          else if (!(p.default >= 0 && p.default < p.options.length)) err(`${where}.${p.key}: default ngoài khoảng options`);
        } else {
          for (const k of ['min', 'max', 'step']) if (typeof p[k] !== 'number') err(`${where}.${p.key}: thiếu "${k}"`);
          if (p.min > p.max || p.default < p.min || p.default > p.max) err(`${where}.${p.key}: default/min/max không hợp lệ`);
        }
      }
    }
    for (const [code, j] of Object.entries(i18n)) {
      const tr = j && j.themes && j.themes[t.id];
      if (!tr) { warn(`${where}: chưa có bản dịch i18n/${code}.json (themes.${t.id})`); continue; }
      if (!tr.name) warn(`${where}: i18n/${code}.json thiếu name`);
      for (const k of keys) if (!(tr.params && tr.params[k] && tr.params[k].label)) warn(`${where}: i18n/${code}.json thiếu nhãn thông số "${k}"`);
    }
    if (t.status === 'ready' && mainRs && !mainRs.includes(`("${t.id}",`)) warn(`${where}: chưa có bản Rust (deskland-rs/src/main.rs, THEMES) - chỉ chạy được ở bản C#`);
  }
}

for (const w of warnings) console.log('CẢNH BÁO  ' + w);
for (const e of errors) console.log('LỖI       ' + e);
console.log(`\n${(reg && reg.themes || []).length} chủ đề · ${errors.length} lỗi · ${warnings.length} cảnh báo`);
process.exit(errors.length ? 1 : 0);
