/* =========================================================================
   New Gods Unlimited — interface nova (V2.0.0)
   Os dados vêm do motor Rust pela ponte Tauri (src/engine.rs). Este arquivo
   só desenha e envia pedidos; nenhuma regra da simulação vive aqui.
   Estrutura: cada render* corresponde a um componente da tela
   (TopBar, Rail, abas da lateral, Arena, gavetas, modais, status).
   ========================================================================= */
'use strict';

const TAURI = window.__TAURI__;
const invoke = (cmd, args) => TAURI.core.invoke(cmd, args);
const send = cmd => invoke('command', { cmd }).catch(err => console.error(cmd, err));

const $ = s => document.querySelector(s);
const fmt = (n, d = 0) => Number(n ?? 0).toLocaleString('pt-BR', { minimumFractionDigits: d, maximumFractionDigits: d });
const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

const IC = {
  play: '<path d="M7 4.5v15l12-7.5z"/>', pause: '<path d="M8 5v14M16 5v14"/>',
  save: '<path d="M5 4h11l3 3v13H5z"/><path d="M8 4v5h7V4M8 20v-6h8v6"/>',
  restart: '<path d="M4 12a8 8 0 1 0 2.4-5.7"/><path d="M4 4v4h4"/>', chev: '<path d="M6 9l6 6 6-6"/>',
  session: '<circle cx="12" cy="12" r="8"/><path d="M12 8v4l3 2"/>', env: '<path d="M3 9c3-3 6 3 9 0s6 3 9 0M3 15c3-3 6 3 9 0s6 3 9 0"/>',
  map: '<path d="M9 4l-5 2v14l5-2 6 2 5-2V4l-5 2z"/><path d="M9 4v14M15 6v14"/>', cells: '<circle cx="9" cy="10" r="5"/><circle cx="16.5" cy="15.5" r="3.5"/>',
  demo: '<path d="M4 19V5M4 19h16"/><path d="M7 15l4-5 3 3 5-7"/>', layers: '<path d="M12 4l9 5-9 5-9-5z"/><path d="M3 14l9 5 9-5"/>',
  collapse: '<path d="M15 6l-6 6 6 6"/>', expand: '<path d="M9 6l6 6-6 6"/>', fit: '<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>',
  target: '<circle cx="12" cy="12" r="7"/><circle cx="12" cy="12" r="2"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3"/>',
  plus: '<path d="M12 5v14M5 12h14"/>', x: '<path d="M6 6l12 12M18 6L6 18"/>', left: '<path d="M15 6l-6 6 6 6"/>', right: '<path d="M9 6l6 6-6 6"/>',
  search: '<circle cx="11" cy="11" r="6"/><path d="M20 20l-4.5-4.5"/>', bolt: '<path d="M13 3L5 14h6l-1 7 8-11h-6z"/>',
  wall: '<rect x="4" y="5" width="16" height="14" rx="1"/><path d="M4 12h16M10 5v7M14 12v7"/>', eraser: '<path d="M8 20h12M5 14l8-8 6 6-8 8H8z"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
  cursor: '<path d="M5 3l14 8-6 2-2 6z"/>', undo: '<path d="M9 14L4 9l5-5"/><path d="M4 9h11a5 5 0 0 1 0 10h-3"/>',
  maxim: '<path d="M14 4h6v6M10 20H4v-6M20 4l-7 7M4 20l7-7"/>', flask: '<path d="M9 3h6M10 3v6l-5 9a2 2 0 0 0 1.7 3h10.6a2 2 0 0 0 1.7-3l-5-9V3"/><path d="M7.5 15h9"/>',
  shuffle: '<path d="M4 7h3l10 10h3M4 17h3l3-3M14 10l3-3h3"/><path d="M18 5l2 2-2 2M18 15l2 2-2 2"/>', folder: '<path d="M3 6h6l2 2h10v11H3z"/>',
  eye: '<path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
  bio: '<path d="M12 21c-4-2-7-6-7-11a7 7 0 0 1 14 0c0 5-3 9-7 11z"/><path d="M12 21V9M12 13l-3-3M12 16l3-3"/>', storm: '<path d="M7 16a4 4 0 1 1 .8-7.9A6 6 0 0 1 19 10a3.5 3.5 0 0 1-1 6.9"/><path d="M12 14l-2 4h3l-2 4"/>',
};
const ic = n => `<svg class="i" viewBox="0 0 24 24">${IC[n] || ''}</svg>`;

/* ------------------------------ Estado da interface ------------------------------ */
const SPEEDS = ['1×', '10×', '100×', '1.000×', '10.000×', '100.000×', '1.000.000×'];
const MENTAL_SCALE = 40000;
const TABS = [
  { id: 'sessao', label: 'Sessão', icon: 'session' }, { id: 'biologia', label: 'Biologia', icon: 'bio' }, { id: 'ambiente', label: 'Ambiente', icon: 'env' },
  { id: 'mapa', label: 'Mapa', icon: 'map' }, { id: 'celulas', label: 'Células', icon: 'cells' },
  { id: 'demografia', label: 'Demografia', icon: 'demo' }, { id: 'camadas', label: 'Camadas', icon: 'layers' },
];
// Configurações neurais: geradores fixos + indivíduos salvos no banco (vem do motor).
let CATALOG = { version: 0, bank_path: '', bank_error: null, options: [
  { key: 'classica', name: 'Clássica', description: 'Genomas iniciais totalmente aleatórios.', saved: false, compatible: true },
  { key: 'motor', name: 'Motor', description: '4 caminhos diretos visão → motor; resto aleatório.', saved: false, compatible: true },
  { key: 'pfguia', name: 'PF guia', description: 'Fundador experimental: segue PFx ou PFy dominante, sem diagonal.', saved: false, compatible: true },
] };
const neuralOption = key => CATALOG.options.find(o => o.key === key) || CATALOG.options[0];
const ENV_LABELS = { min_repro: 'Mínimo reprodução', limit: 'Limite obrigatório', mental: 'Custo mental', emission: 'Abiogênese', lifetime: 'Vida-base sem sol', phyto_rate: 'Multiplicação sob luz', food_energy: 'Energia por fito', flow_on: 'Corrente ligada', flow_intensity: 'Intensidade da corrente', flow_preset: 'Modelo de corrente', ac_on: 'Anti-cramming', ac_force: 'Força anti-cramming', ac_distance: 'Espaço anti-cramming' };
const SLOT_NAMES = ['V<', 'V>', 'V^', 'Vv', 'Cx', 'Cy', 'En', '+1', 'Ct', 'PG', 'PGx', 'PGy', 'P1', 'P1x', 'P1y', 'P2', 'P2x', 'P2y', 'P3', 'P3x', 'P3y', 'Fx', 'Fy', 'PF', 'PFx', 'PFy',
  'I0', 'I1', 'I2', 'I3', 'I4', 'I5', 'I6', 'I7', 'I8', 'I9', 'I10', 'I11', 'I12', 'I13', 'I14', 'I15', 'T1', 'T2', 'T3',
  '←', '→', '↑', '↓', 'Rep', 'Fag', 'EG', 'E1', 'E2', 'E3', 'Doar'];
const SLOT_DESC = ['Visão · entidade à esquerda', 'Visão · entidade à direita', 'Visão · entidade acima', 'Visão · entidade abaixo', 'Corrente no eixo X', 'Corrente no eixo Y', 'Energia relativa', 'Sinal constante', 'Contato com outra célula', 'Intensidade de PG (assinatura genética)', 'Gradiente X de PG', 'Gradiente Y de PG', 'Intensidade de P1', 'Gradiente X de P1', 'Gradiente Y de P1', 'Intensidade de P2', 'Gradiente X de P2', 'Gradiente Y de P2', 'Intensidade de P3', 'Gradiente X de P3', 'Gradiente Y de P3', 'Força externa X', 'Força externa Y', 'Intensidade de PF (fitoplâncton)', 'Gradiente X de PF', 'Gradiente Y de PF'];
const slotDesc = i => i < 26 ? SLOT_DESC[i] : i < 42 ? `Neurônio interno ${SLOT_NAMES[i]}` : i < 45 ? `Temporizador ${SLOT_NAMES[i]}` : ['Motor · nadar à esquerda', 'Motor · nadar à direita', 'Motor · nadar acima', 'Motor · nadar abaixo', 'Motor · reproduzir', 'Motor · fagocitar', 'Motor · reforçar PG', 'Motor · emitir P1', 'Motor · emitir P2', 'Motor · emitir P3', 'Motor · doar energia'][i - 45];

let INFO = null;
const S = {
  mode: 'sessao', tab: 'sessao', collapsed: false, speedMenu: false, keepAwake: true,
  insert: false, insertOk: false, selected: null, follow: false,
  cellsView: 'ind', genView: 'resumo', circView: 'con', demoWin: '30k', demoFull: false,
  cellGroups: { resumo: true, corpo: false, sensores: false, rede: true, genoma: false }, showRecords: false,
  layers: { flow: false, cones: false, conflicts: false, sun: true, chem: false, chemSel: 'PF' },
  sections: {}, mapTool: 'none', brush: 3,
  drawer: null, modal: null,
  init: null, initApplied: null, env: null, envApplied: null,
  events: [], msg: null, lastNoticeSeq: 0,
};
let F = null;                 // último Frame do motor
let MAP = null;               // último MapData
// epoch 0 = "ainda sem amostras"; o motor numera as runs a partir de 1 (u64, nunca negativo).
let TEL = { epoch: 0, samples: [] };
let GEN = null;
let FLOW = { key: '', data: [] };
let VIEW = null;               // indivíduo aberto na gaveta
let lastBankVersion = -1;

/* ------------------------------ Componentes ------------------------------ */
const getF = path => { if (!path.includes('.')) return S[path]; const [g, k] = path.split('.'); return S[g][k]; };
const setF = (path, v) => { const [g, k] = path.split('.'); S[g][k] = v; };
const isDirty = path => { const [g, k] = path.split('.'); if (g === 'env') return S.env[k] !== S.envApplied[k]; if (g === 'init') return S.init[k] !== S.initApplied[k]; return false; };
const envPending = () => Object.keys(S.env).filter(k => S.env[k] !== S.envApplied[k]);
const BIO_KEYS = ['min_repro', 'limit', 'mental', 'emission', 'lifetime', 'phyto_rate', 'food_energy'];
const bioPending = () => envPending().filter(k => BIO_KEYS.includes(k));
const physPending = () => envPending().filter(k => !BIO_KEYS.includes(k));
const initPending = () => Object.keys(S.init).filter(k => S.init[k] !== S.initApplied[k]);

function section(key, title, body, meta = '', defOpen = true) {
  const open = S.sections[key] ?? defOpen;
  return `<div class="sec ${open ? '' : 'closed'}"><button class="sec-h" data-act="sec" data-v="${key}"><span class="chev">${ic('chev')}</span>${title}<span class="meta">${meta}</span></button><div class="sec-b">${body}</div></div>`;
}
// Campo com valor exibido em unidade humana: `scale` converte do valor físico para a tela.
function slider(path, label, min, max, step, unit = '', foot = '', scale = 1, digits = null) {
  const raw = getF(path); const v = +(raw * scale).toFixed(digits ?? 6);
  return `<div class="field ${isDirty(path) ? 'dirty' : ''}" data-row="${path}"><div class="field-top"><label>${label}</label>${unit ? `<span class="unit">${unit}</span>` : ''}<input class="num" type="number" data-f="${path}" data-scale="${scale}" min="${min}" max="${max}" step="${step}" value="${v}"></div><input type="range" data-f="${path}" data-scale="${scale}" min="${min}" max="${max}" step="${step}" value="${v}">${foot ? `<div class="field-foot">${foot}</div>` : ''}</div>`;
}
function toggle(path, label, sub = '', layer = false) {
  const v = layer ? S.layers[path] : getF(path);
  const attr = layer ? `data-layer="${path}"` : `data-f="${path}"`;
  return `<label class="tog ${!layer && path.includes('.') && isDirty(path) ? 'dirty' : ''}" data-row="${path}"><input type="checkbox" ${attr} ${v ? 'checked' : ''}><span class="sw"></span><span class="tl"><span>${label}</span>${sub ? `<small>${sub}</small>` : ''}</span></label>`;
}
function segF(path, opts) {
  const v = getF(path);
  return `<div class="seg ${isDirty(path) ? 'dirty' : ''}" data-row="${path}">${opts.map(o => `<button class="${o === v ? 'on' : ''}" data-act="setf" data-f="${path}" data-v="${o}">${o}</button>`).join('')}</div>`;
}
const segUI = (key, opts, cur) => `<div class="seg">${opts.map(([id, l]) => `<button class="${id === cur ? 'on' : ''}" data-act="ui" data-k="${key}" data-v="${id}">${l}</button>`).join('')}</div>`;
const ro = (label, val, cls = '', live = '') => `<div class="ro ${cls}"><small>${label}</small><b class="mono" ${live ? `data-live="${live}"` : ''}>${val}</b></div>`;
const head = (title, sub, right = '') => `<div class="side-head"><div style="flex:1;min-width:0"><h2>${title}</h2>${sub ? `<div class="sub">${sub}</div>` : ''}</div>${right}</div>`;
const hex = c => '#' + c.toString(16).padStart(6, '0');

/* ------------------------------ Barra superior ------------------------------ */
function renderTop() {
  const top = $('#top');
  const brand = `<div class="brand"><div class="logo"></div>New Gods Unlimited <span class="pill">V${INFO.version}</span></div>`;
  const modes = `<div class="modesw"><button class="${S.mode === 'sessao' ? 'on' : ''}" data-act="mode" data-v="sessao">Sessão</button><button class="${S.mode === 'lab' ? 'on' : ''}" data-act="mode" data-v="lab">Laboratório</button></div>`;
  if (S.mode === 'lab') { top.innerHTML = `${brand}${modes}<div class="vsep"></div><span class="muted">Campanhas offline, execução paralela e replay gravado</span>`; return; }
  const ep = envPending().length, ip = initPending().length, paused = F?.paused;
  top.innerHTML = `${brand}${modes}<div class="vsep"></div>
    <div class="runinfo">
      <div class="kv"><small>Run</small><b class="mono" data-live="run">#${String(F?.run_id ?? '').slice(-7)}</b></div>
      <div class="kv"><small>Tick</small><b class="mono" data-live="tick">${fmt(F?.tick)}</b></div>
      <div class="kv"><small>Células</small><b class="mono" data-live="cells">${fmt(F?.population)}</b></div>
      <div class="kv"><small>Fitoplâncton</small><b class="mono" data-live="food">${fmt(F?.food_count)}</b></div>
      <div class="kv"><small>Ticks/s</small><b class="mono" data-live="tps">${fmt(F?.tps)}</b></div>
    </div>
    <div class="grow"></div>
    <button class="btn ${paused ? 'pri' : ''}" data-act="pause" title="Espaço">${ic(paused ? 'play' : 'pause')}${paused ? 'Iniciar' : 'Pausar'}</button>
    <div class="speed"><button class="btn" data-act="speedMenu" title="Teclas 1–7"><span class="mono">${SPEEDS[F?.speed ?? 0]}</span>${ic('chev')}</button>
      ${S.speedMenu ? `<div class="menu">${SPEEDS.map((s, i) => `<button class="${i === F?.speed ? 'on' : ''}" data-act="speed" data-v="${i}"><span class="mono">${s}</span><span class="kbd">${i + 1}</span></button>`).join('')}</div>` : ''}</div>
    <div class="vsep"></div>
    <div class="pend">
      ${ep ? `<span class="pill wa">${ep} ${ep === 1 ? 'alteração pendente' : 'alterações pendentes'}</span>` : ''}
      ${ip ? `<span class="pill">${ip} ao reiniciar</span>` : ''}
      <button class="btn ${ep ? 'warn' : ''}" data-act="apply" ${ep ? '' : 'disabled'}>Aplicar</button>
      <button class="btn ghost" data-act="revert" ${ep || ip ? '' : 'disabled'}>Reverter</button>
    </div>
    <button class="btn icon ghost" data-act="askRestart" title="Reiniciar sessão">${ic('restart')}</button>
    <button class="btn" data-act="drawer" data-v="runs" title="Ctrl+S">${ic('save')}Salvar</button>`;
}

/* ------------------------------ Trilho de abas ------------------------------ */
function renderRail() {
  const lab = S.mode === 'lab';
  const dots = { biologia: bioPending().length > 0, ambiente: physPending().length > 0, sessao: initPending().length > 0, mapa: !!F?.map_dirty };
  $('#rail').innerHTML = lab ? '' : TABS.map(t => `<button class="${t.id === S.tab && !S.collapsed ? 'on' : ''}" data-act="tab" data-v="${t.id}" title="${t.label}">${ic(t.icon)}${t.label}${dots[t.id] ? '<span class="dot"></span>' : ''}</button>`).join('') +
    `<div class="sp"></div><button data-act="collapse" title="${S.collapsed ? 'Abrir painel' : 'Recolher painel'}">${ic(S.collapsed ? 'collapse' : 'expand')}${S.collapsed ? 'Abrir' : 'Recolher'}</button>`;
  $('#main').classList.toggle('collapsed', S.collapsed || lab);
  $('#rail').style.display = lab ? 'none' : '';
  $('#main').style.gridTemplateColumns = lab ? '1fr 0 0' : '';
}

/* ------------------------------ Painel lateral ------------------------------ */
function renderPanel() {
  if (S.mode === 'lab') { $('#sideInner').innerHTML = ''; return; }
  const keep = $('.side-body')?.scrollTop || 0;
  const fn = { sessao: tabSessao, biologia: tabBiologia, ambiente: tabAmbiente, mapa: tabMapa, celulas: tabCelulas, demografia: tabDemografia, camadas: tabCamadas }[S.tab];
  $('#sideInner').innerHTML = fn();
  const sb = $('.side-body'); if (sb && renderPanel.lastTab === S.tab) sb.scrollTop = keep;
  renderPanel.lastTab = S.tab;
}

function tabSessao() {
  const ip = initPending().length;
  const neural = neuralOption(S.init.neural);
  const saved = CATALOG.options.filter(o => o.saved);
  const option = o => `<div class="choice ${S.init.neural === o.key ? 'on' : ''}" style="padding:0;align-items:stretch">
      <button class="choice" style="border:0;background:none;flex:1" data-act="neural" data-v="${esc(o.key)}" ${o.compatible ? '' : 'disabled'}><span class="rd"></span>${o.color ? `<span class="sw-c" style="background:${esc(o.color)};margin-top:1px"></span>` : ''}<span><b>${esc(o.name)}</b><small>${esc(o.description)}</small></span></button>
      <button class="btn icon ghost" style="height:auto;width:38px;border-radius:0 8px 8px 0" data-act="viewInd" data-v="${esc(o.key)}" title="Ver o indivíduo">${ic('eye')}</button></div>`;
  const body = `
    ${section('s-exec', 'Execução', `
      <div class="readouts">${ro('Tick', fmt(F?.tick), '', 'tick')}${ro('Células vivas', fmt(F?.population), '', 'cells')}${ro('Ticks/s', fmt(F?.tps), '', 'tps')}${ro('Publicação', fmt(F?.publish_hz) + ' Hz', '', 'pub')}${ro('Teto de cálculo (sem limitador)', fmt(F?.capacity_tps) + ' ticks/s', 'wide', 'cap')}</div>
      <div class="hint">Ticks/s é a velocidade real da sessão (1× ≈ 10 ticks/s). O teto é quanto o motor conseguiria sem o limitador de velocidade.</div>
      ${threadsControl()}
      <button class="btn block" data-act="modal" data-v="headless">${ic('bolt')}Modo headless · medir teto de cálculo</button>
      ${toggle('keepAwake', 'Manter Windows ativo', 'Impede a suspensão enquanto a simulação calcula')}
    `)}
    ${section('s-init', 'Condições iniciais', `
      <div class="hint">Estes valores só valem quando a sessão é reiniciada.</div>
      <div class="field ${isDirty('init.seed') ? 'dirty' : ''}" data-row="init.seed"><div class="field-top"><label>Seed</label></div>
        <div class="row"><input class="txt mono" type="text" data-f="init.seed" value="${esc(S.init.seed)}"><button class="btn sm" data-act="newSeed">${ic('shuffle')}Nova</button></div></div>
      ${slider('init.cells', 'Células iniciais', 1, 400, 1)}
      <div class="field"><div class="field-top"><label>Configuração neural</label><span class="unit" style="margin-left:auto">${ic('eye')} abre o indivíduo</span></div>
        <div class="choices">${CATALOG.options.filter(o => !o.saved).map(option).join('')}</div>
        <div class="field-top" style="margin-top:6px"><label>Indivíduos salvos</label><span class="unit" style="margin-left:auto">${saved.length}</span></div>
        ${saved.length ? `<div class="choices">${saved.map(option).join('')}</div>` : `<div class="hint">Abra uma célula em <b>Células › Indivíduo</b> e clique em <b>${ic('save')} Salvar</b> para ela aparecer aqui.</div>`}
        ${CATALOG.bank_error ? `<div class="hint" style="color:var(--dg)">${esc(CATALOG.bank_error)}</div>` : ''}</div>
      ${toggle('init.amadeus', 'Modo Amadeus', 'Uma única célula central; descendentes ainda mutam')}
      <button class="btn ${ip ? 'warn' : ''} block" data-act="askRestart">${ic('restart')}Reiniciar com estas condições</button>
    `, `<span class="tag ${ip ? 'wa' : ''}">ao reiniciar</span>`)}
    ${section('s-insert', 'Inserir célula', `
      <div class="hint">Cria uma célula no ponto clicado sem reiniciar a run, com a configuração neural escolhida acima: <b>${esc(neural.name)}</b>.</div>
      <button class="btn ${S.insert ? 'pri' : ''} block" data-act="insert">${ic(S.insert ? 'x' : 'plus')}${S.insert ? 'Cancelar inserção' : 'Inserir célula no mapa'}</button>
    `)}
    ${section('s-persist', 'Salvar e presets', `
      <div class="soon"><b>Chega na próxima versão.</b> Salvar e carregar runs (.ecosrun) e presets de sessão ainda não passaram para a interface nova. Enquanto isso, use a V1.1.0 para salvar runs.</div>
    `)}`;
  return head('Sessão', `run #${String(F?.run_id ?? '').slice(-7)}`) + `<div class="side-body">${body}</div>`;
}

function pendingPill(list) { const n = list.length; return n ? `<span class="pill wa">${n} pendente${n > 1 ? 's' : ''}</span>` : ''; }
function tabBiologia() {
  const body = `
    ${section('b-cel', 'Células', `
      ${slider('env.min_repro', 'Mínimo reprodução', 1, 5000, 1, 'energia', '<span>acima disso o motor Reproduzir é aceito</span>')}
      ${slider('env.limit', 'Limite obrigatório', 100, 20000, 50, 'energia', '<span>divisão forçada neste teto</span>')}
      ${slider('env.mental', 'Custo mental', 0, 400, 1, 'por conexão/tick', '<span>100 = 0,0025</span>', MENTAL_SCALE, 0)}
    `)}
    ${section('b-fito', 'Fitoplâncton', `
      <div class="readouts">${ro('No mapa', fmt(F?.food_count), '', 'food')}${ro('Vida sob sol', fmt(Math.round(S.env.lifetime * 1.5)) + ' ticks', '', 'lifeSun')}</div>
      ${slider('env.emission', 'Nascimento por abiogênese', 0, 1, 0.01, 'fito/tick')}
      ${slider('env.lifetime', 'Vida-base sem sol', 10, 6000, 10, 'ticks', '<span>sob sol recebe 1,5×</span>')}
      ${slider('env.phyto_rate', 'Multiplicação sob luz', 0, 2, 0.01, '%/tick', '', 100, 3)}
      ${slider('env.food_energy', 'Energia por fito', 1, 500, 1, 'energia')}
    `)}`;
  return head('Biologia', 'Células e fitoplâncton · valem ao aplicar', `<span data-pending="bio">${pendingPill(bioPending())}</span>`) + `<div class="side-body">${body}</div>`;
}

function tabAmbiente() {
  const body = `
    ${section('a-corr', 'Corrente', `
      ${toggle('env.flow_on', 'Corrente ligada')}
      ${slider('env.flow_intensity', 'Intensidade', 0, 100, 1, '%', '', 100, 0)}
      <div class="field"><div class="field-top"><label>Modelo</label></div>${segF('env.flow_preset', ['Clássico', 'Oásis', 'Esquerda'])}</div>
      <div class="hint">Vetores da corrente: aba Camadas.</div>
    `)}
    ${section('a-dens', 'Densidade', `
      ${toggle('env.ac_on', 'Anti-cramming', 'Separa células sobrepostas')}
      ${slider('env.ac_force', 'Força', 0, 1, 0.05)}
      ${slider('env.ac_distance', 'Espaço mínimo', 1, +(INFO.max_exclusion / INFO.min_exclusion).toFixed(2), 0.05, '× contato', '', 1 / INFO.min_exclusion, 2)}
      <div class="li"><div class="meta"><b class="mono" data-live="conflicts">${fmt(F?.crowding_pairs)} pares em conflito</b><small>abaixo do espaço mínimo</small></div><button class="btn sm" data-act="layerOn" data-v="conflicts">${ic('eye')}Ver na arena</button></div>
    `)}`;
  return head('Ambiente', 'Física do mundo · valem ao aplicar', `<span data-pending="phys">${pendingPill(physPending())}</span>`) + `<div class="side-body">${body}</div>`;
}

function tabMapa() {
  const tools = [['none', 'Selecionar', 'cursor'], ['wall', 'Parede', 'wall'], ['eraser', 'Borracha', 'eraser'], ['sun', 'Sol', 'sun']];
  const dirty = !!F?.map_dirty;
  const body = `
    ${section('m-tools', 'Ferramentas', `
      <div class="seg">${tools.map(([id, l, i]) => `<button class="${S.mapTool === id ? 'on' : ''}" data-act="tool" data-v="${id}" style="display:flex;align-items:center;justify-content:center;gap:5px">${ic(i)}${l}</button>`).join('')}</div>
      <div class="field"><div class="field-top"><label>Raio do pincel</label><span class="unit">pixels do mapa</span><input class="num" type="number" data-g="brush" min="0" max="12" value="${S.brush}"></div><input type="range" data-g="brush" min="0" max="12" step="1" value="${S.brush}"></div>
      <div class="hint">${S.mapTool === 'sun' ? 'Sol: desenhe o contorno da região arrastando na arena; ao soltar, ela substitui a área ensolarada.' : 'Parede e Borracha: arraste na arena para pintar.'} Nada entra no mundo antes de aplicar o mapa.</div>
      <div class="row"><button class="btn sm" data-act="mapUndo">${ic('undo')}Desfazer</button><button class="btn sm" data-act="mapClearSun">${ic('sun')}Limpar sol</button><div class="grow"></div>${toggle('sun', 'Ver iluminação', '', true)}</div>
      <div class="row"><button class="btn ${dirty ? 'warn' : ''}" style="flex:1" data-act="mapApply" ${dirty ? '' : 'disabled'}>Aplicar mapa${dirty ? ' *' : ''}</button><button class="btn ghost" data-act="mapRevert" ${dirty ? '' : 'disabled'}>Descartar</button></div>
    `)}
    ${section('m-flow', 'Correnteza do mapa', `
      ${segF('env.flow_preset', ['Clássico', 'Oásis', 'Esquerda'])}
      <div class="hint">Clássico: recirculante com giros locais. Esquerda: vento contínuo. Oásis: giro em torno do oásis. Vale ao aplicar.</div>
    `)}
    ${section('m-presets', 'Presets de mapa', `<div class="soon"><b>Chega na próxima versão.</b> Salvar e carregar mapas ainda não passou para a interface nova.</div>`)}`;
  return head('Mapa', S.mapTool === 'none' ? 'Editor de paredes, sol e correnteza' : 'Editando · arraste na arena', dirty ? '<span class="pill wa">não aplicado</span>' : '') + `<div class="side-body">${body}</div>`;
}

function tabCelulas() {
  const top = `<div class="subnav">${segUI('cellsView', [['ind', 'Indivíduo'], ['gen', 'Genética']], S.cellsView)}</div>`;
  if (S.cellsView === 'gen') return head('Células', `${fmt(F?.population)} vivas · relatório a cada ~1,5 s`) + top + `<div class="side-body">${genetics()}</div>`;
  const d = F?.detail;
  if (!d) return head('Células', `${fmt(F?.population)} vivas`) + top + `<div class="empty">Clique numa célula na arena<br>ou use <span class="kbd">[</span> <span class="kbd">]</span> para navegar.</div>`;
  const g = S.cellGroups;
  const sec = (k, t, b) => `<div class="sec ${g[k] ? '' : 'closed'}"><button class="sec-h" data-act="cgroup" data-v="${k}"><span class="chev">${ic('chev')}</span>${t}</button><div class="sec-b">${b}</div></div>`;
  const r = d.radius / INFO.cell_radius;
  const body = `
    <div class="cellhead">
      <button class="btn icon" data-act="cellNav" data-v="-1" title="[">${ic('left')}</button>
      <div class="who"><span class="cdot" style="background:${d.color}"></span><div class="meta"><b>Célula #${d.id}</b><small>geração ${fmt(d.generation)} · cor ${d.color}</small></div></div>
      <button class="btn icon ${S.follow ? 'pri' : ''}" data-act="follow" title="Seguir na arena">${ic('target')}</button>
      <button class="btn icon" data-act="cellNav" data-v="1" title="]">${ic('right')}</button>
    </div>
    <div style="padding:10px 16px 0"><button class="btn block" data-act="askSave">${ic('save')}Salvar este indivíduo no banco</button></div>
    <div style="padding:10px 16px;border-bottom:1px solid var(--bd)"><div class="row"><span class="muted">${ic('search')}</span><input class="txt mono" placeholder="Buscar por id e Enter…" data-g="cellSearch" style="height:28px"></div></div>
    ${sec('resumo', 'Resumo', `
      <div class="field"><div class="field-top"><label>Energia</label><span class="mono" style="margin-left:auto" data-live="cE">${fmt(d.energy, 1)} / ${fmt(d.capacity)}</span></div><div class="bar"><i data-live="cEbar" style="width:${clamp(d.energy / d.capacity * 100, 0, 100)}%"></i></div></div>
      <div class="readouts">
        ${ro('Tempo viva / limite', `${fmt(d.age)} / ${fmt(d.max_age)}`, 'wide', 'cAge')}
        ${ro('Filhos produzidos', fmt(d.children), '', 'cKids')}${ro('Conexões / nós ativos', `${d.connections} / ${d.active_nodes}`, '', 'cConn')}
        ${ro('Mais parecida', d.similar_id != null ? `#${d.similar_id} · ${fmt(d.similar * 100, 1)}%` : 'sem outra célula', 'wide', 'cSim')}
      </div>`)}
    ${sec('corpo', 'Corpo', `<div class="readouts">${ro('Raio físico', fmt(d.radius, 1) + ' u', '', 'cRad')}${ro('Tamanho', fmt(r, 2) + ' R', '', 'cSize')}${ro('Velocidade máx.', fmt(8 / Math.sqrt(Math.max(r, 0.01)), 2) + ' u/tick', 'wide', 'cSpeed')}</div>`)}
    ${sec('sensores', 'Sensores', `<div class="readouts">${ro('Alcance visual', fmt(d.sensor_range) + ' u', '', 'cRange')}${ro('Custo do sensor', fmt(d.sensor_cost, 4) + '/tick', '', 'cCost')}${ro('Alcance químico', '500 u')}${ro('Idade máxima (alelo)', fmt(d.max_age) + ' ticks')}</div><button class="btn sm" data-act="layerOn" data-v="cones">${ic('eye')}Mostrar cones na arena</button>`)}
    ${sec('rede', `Rede neural ao vivo`, `<div class="hint">P1, P2 e P3 deste indivíduo: <b class="mono">${d.genome_map ? d.genome_map.channels.map((c, k) => `P${k + 1}=C${c}`).join(' · ') : '—'}</b></div><div id="nnBox">${neuralNet({ ...d, channels: d.genome_map?.channels })}</div><div class="legend"><span><i style="background:var(--ok)"></i>sinal positivo</span><span><i style="background:var(--dg)"></i>negativo</span><span><i style="background:var(--s4)"></i>nó inativo geneticamente</span></div>`)}
    ${sec('genoma', 'Código genético', genomeDisclosure(d.genome, d.genome_map))}
  `;
  return head('Células', `${fmt(F?.population)} vivas`) + top + `<div class="side-body" style="padding-top:0">${body}</div>`;
}

// Rede neural: 26 sensores, 19 internos, 11 motores; fios e nós coloridos pelo sinal real.
function nnLayout() {
  const rowH = 13, w = 300, hgt = 26 * rowH + 26, pos = [];
  const col = (from, to, x) => { const n = to - from, off = 18 + (hgt - 18 - n * rowH) / 2; for (let i = 0; i < n; i++) pos[from + i] = [x, off + i * rowH + rowH / 2]; };
  col(0, 26, 34); col(26, 45, 150); col(45, 56, 266);
  return { w, hgt, pos };
}
// P1/P2/P3 (sensores 12–20) e E1/E2/E3 (motores 52–54) trabalham com a substância escolhida pelo genoma.
function chemOf(i, channels) {
  if (!channels) return null;
  if (i >= 12 && i <= 20) return channels[Math.floor((i - 12) / 3)];
  if (i >= 52 && i <= 54) return channels[i - 52];
  return null;
}
const slotTitle = (i, channels) => { const c = chemOf(i, channels); return c == null ? SLOT_NAMES[i] : `${SLOT_NAMES[i]} (C${c})`; };
const sigColor = v => v > 0.05 ? 'var(--ok)' : v < -0.05 ? 'var(--dg)' : 'var(--mu)';
function neuralNet(d) {
  const { w, hgt, pos } = nnLayout();
  const wires = d.wires.map(([a, b, wt, sig], i) => {
    const pa = pos[a | 0], pb = pos[b | 0]; if (!pa || !pb) return '';
    const [x1, y1] = pa, [x2, y2] = pb;
    return `<path data-w="${i}" d="M${x1} ${y1} C${(x1 + x2) / 2} ${y1} ${(x1 + x2) / 2} ${y2} ${x2} ${y2}" stroke="${wt >= 0 ? 'var(--ok)' : 'var(--dg)'}" stroke-opacity="${0.15 + Math.min(1, Math.abs(sig)) * 0.7}" stroke-width="${0.6 + Math.min(4, Math.abs(wt)) * 0.45}" fill="none"/>`;
  }).join('');
  const nodes = pos.map(([x, y], i) => {
    const inactive = !d.active[i]; const v = d.slots[i] || 0;
    const kind = i < 26 ? 's' : i < 45 ? 'i' : 'm';
    const anchor = kind === 's' ? 'end' : kind === 'm' ? 'start' : 'middle'; const tx = kind === 's' ? x - 8 : kind === 'm' ? x + 8 : x;
    const ch = chemOf(i, d.channels), label = [12, 15, 18, 52, 53, 54].includes(i) && ch != null ? `${SLOT_NAMES[i]}·C${ch}` : SLOT_NAMES[i];
    const chemNote = ch != null ? ` · trabalha com a substância C${ch}` : '';
    return `<g><title>${slotTitle(i, d.channels)} · ${slotDesc(i)}${chemNote}${inactive ? ' · inativo geneticamente' : ''} · ${v.toFixed(2)}</title><circle data-nn="${i}" cx="${x}" cy="${y}" r="${inactive ? 3 : 3 + Math.min(1, Math.abs(v)) * 2}" fill="${inactive ? 'var(--s4)' : sigColor(v)}" stroke="var(--s1)" stroke-width="1"/>${kind !== 'i' ? `<text x="${tx}" y="${y + 3}" text-anchor="${anchor}">${label}</text>` : ''}</g>`;
  }).join('');
  return `<svg class="nn" viewBox="0 0 ${w} ${hgt}" style="max-height:420px;width:100%">
    <text x="34" y="9" text-anchor="middle" style="fill:var(--mu2)">sensores</text><text x="150" y="9" text-anchor="middle" style="fill:var(--mu2)">internos</text><text x="266" y="9" text-anchor="middle" style="fill:var(--mu2)">motores</text>
    ${wires}${nodes}</svg>`;
}
const SECTION_COLORS = { fiacao: 'var(--ac)', canais: '#b48cf0', alcance: 'var(--ok)', atividade: 'var(--wa)', reserva: 'var(--mu2)' };
function genomeGrid(bits, map) {
  const colorAt = i => {
    if (!map) return 'var(--ac)';
    if (i >= map.sections.find(x => x.key === 'canais').start && i < map.sections.find(x => x.key === 'canais').start + 9) return SECTION_COLORS.canais;
    for (const sec of map.sections) if (sec.length && i >= sec.start && i < sec.start + sec.length && sec.key !== 'canais') return SECTION_COLORS[sec.key] || 'var(--ac)';
    return 'var(--ac)';
  };
  let s = ''; for (let i = 0; i < bits.length; i++) s += `<i style="${bits[i] === '1' ? `background:${colorAt(i)};opacity:.85` : ''}" title="bit ${i}"></i>`;
  return `<div class="genome">${s}</div>`;
}
const groupBits = (bits, sizes) => { let out = [], p = 0; for (const n of sizes) { out.push(bits.slice(p, p + n)); p += n; } return out.join(' '); };
function genomeDisclosure(bits, map) {
  if (!map) return genomeGrid(bits);
  const legend = `<div class="legend">${map.sections.filter(x => x.length).map(x => `<span><i style="background:${SECTION_COLORS[x.key]}"></i>${esc(x.name)}</span>`).join('')}</div>`;
  const sections = map.sections.map(x => `<div class="li" style="flex-direction:column;align-items:stretch;gap:4px">
      <div class="row"><span class="sw-c" style="background:${SECTION_COLORS[x.key] || 'var(--s4)'}"></span><b style="font-weight:500">${esc(x.name)}</b><span class="mono muted" style="margin-left:auto;font-size:11px">${x.length ? `bits ${x.start}–${x.start + x.length - 1} · ${x.length} bits` : 'fora do genoma'}</span></div>
      <div class="mono" style="color:var(--tx)">${esc(x.summary)}</div>
      ${x.bits ? `<div class="mono muted" style="font-size:11px;word-break:break-all">${x.key === 'canais' ? groupBits(x.bits, [3, 3, 3]) : esc(x.bits)}</div>` : ''}
      <small class="muted" style="line-height:1.4">${esc(x.note)}</small></div>`).join('');
  const recs = map.records.map(r => {
    const status = r.effective ? '<span class="pill ok">no cérebro</span>' : !r.active ? '<span class="pill">desligado</span>' : !r.legal ? '<span class="pill dg">ilegal</span>' : '<span class="pill wa">nó desligado</span>';
    return `<div class="li" style="flex-direction:column;align-items:stretch;gap:2px;padding:6px 10px;${r.effective ? '' : 'opacity:.6'}">
      <div class="row"><span class="mono muted" style="width:26px">#${r.index}</span><b class="mono" style="font-weight:500">${esc(slotTitle(r.source, map.channels))} → ${esc(slotTitle(r.destination, map.channels))}</b><span class="mono" style="margin-left:auto">${r.weight >= 0 ? '+' : '−'}${fmt(Math.abs(r.weight), 3)}</span>${status}</div>
      <span class="mono muted" style="font-size:10.5px;padding-left:34px">${groupBits(r.bits, [6, 6, 6, 1])} · bits ${r.start}–${r.start + 18}</span></div>`;
  }).join('');
  const shown = S.showRecords;
  return `<div class="hint mono">${map.total_bits} bits · cor dos bits = trecho</div>${genomeGrid(bits, map)}${legend}
    <div class="list">${sections}</div>
    <button class="btn sm" data-act="toggleRecords">${ic(shown ? 'chev' : 'right')}${shown ? 'Esconder' : 'Ver'} os 64 registros de fiação</button>
    ${shown ? `<div class="hint">Cada registro: <span class="mono">origem · destino · peso · ativo</span>. Pesos de −4 a +3,875 em passos de 0,125.</div><div class="list">${recs}</div>` : ''}`;
}

function genetics() {
  const G = GEN;
  const sub = `<div class="subnav" style="border-bottom:0;padding-bottom:0">${segUI('genView', [['resumo', 'Resumo'], ['grupos', 'Grupos'], ['circ', 'Circuitos']], S.genView)}</div>`;
  if (!G) return sub + `<div class="empty">Calculando o relatório genético…</div>`;
  if (!G.population) return sub + `<div class="empty">Não há células vivas.</div>`;
  if (S.genView === 'grupos') return sub + `<div class="sec-b" style="padding-top:12px"><div class="hint">Grupos por similaridade mínima de 98,5% · ${G.group_count} grupos</div><div class="list">${G.groups.map((g, i) => `<div class="li click" data-act="openRep" data-v="${g.representative}"><span class="sw-c" style="background:${g.color}"></span><div class="meta"><b>Grupo ${i + 1} · ${fmt(g.count)} células (${fmt(g.share * 100, 1)}%)</b><small>gen. ${fmt(g.average_generation, 1)} · coesão ${fmt(g.cohesion * 100, 2)}% · abrir representante</small></div>${ic('right')}</div>`).join('')}</div></div>`;
  if (S.genView === 'circ') {
    const sub2 = `<div style="padding:10px 16px 0">${segUI('circView', [['con', 'Conexões'], ['neu', 'Neurônios']], S.circView)}</div>`;
    if (S.circView === 'neu') {
      const top = (key) => { const best = [...G.slots].sort((a, b) => b[key] - a[key])[0]; return best ? `${SLOT_NAMES[best.slot]} · ${fmt(best[key] * 100, 1)}%` : '—'; };
      const motor = [...G.slots].filter(s => s.slot >= 45).sort((a, b) => b.as_destination - a.as_destination)[0];
      return sub + sub2 + `<div class="sec-b" style="padding-top:12px"><div class="readouts">${ro('Origem mais usada', top('as_source'), 'wide')}${ro('Destino mais recrutado', top('as_destination'), 'wide')}${ro('Motor mais alimentado', motor ? `${SLOT_NAMES[motor.slot]} · ${fmt(motor.as_destination * 100, 1)}%` : '—', 'wide')}</div><div class="hint">Participação por slot</div><div class="list">${G.slots.slice(0, 16).map(s => `<div class="li"><span class="mono" style="width:38px">${SLOT_NAMES[s.slot]}</span><div class="meta"><div class="bar"><i style="width:${s.used * 100}%"></i></div><small>uso ${fmt(s.used * 100)}% · sai ${fmt(s.as_source * 100)}% · entra ${fmt(s.as_destination * 100)}%</small></div></div>`).join('')}</div></div>`;
    }
    return sub + sub2 + `<div class="sec-b" style="padding-top:12px"><div class="hint">Pares origem → destino mais presentes</div><div class="list">${G.connections.map(c => `<div class="li"><div class="meta"><b class="mono">${SLOT_NAMES[c.source]} → ${SLOT_NAMES[c.destination]}</b><small>${fmt(c.frequency * 100, 1)}% dos cérebros · ${c.groups} grupos</small></div><span class="pill ${c.mean_weight < 0 ? 'dg' : 'ok'} mono">peso ${c.mean_weight >= 0 ? '+' : '−'}${fmt(Math.abs(c.mean_weight), 2)}</span></div>`).join('')}</div></div>`;
  }
  const c0 = G.connections[0], s0 = G.slots[0];
  return sub + `<div class="sec-b" style="padding-top:12px"><div class="readouts">
    ${ro('População analisada', fmt(G.population))}${ro('Grupos genéticos', fmt(G.group_count))}
    ${ro('Grupo dominante', fmt(G.dominant_share * 100, 1) + '%')}${ro('Concordância consenso', fmt(G.consensus * 100, 1) + '%')}
    ${ro('Coesão do dominante', fmt(G.dominant_cohesion * 100, 2) + '%', 'wide')}
    ${ro('Circuito mais recorrente', c0 ? `${SLOT_NAMES[c0.source]} → ${SLOT_NAMES[c0.destination]} · ${fmt(c0.frequency * 100)}%` : '—', 'wide')}
    ${ro('Slot mais usado', s0 ? `${SLOT_NAMES[s0.slot]} · ${fmt(s0.used * 100)}% dos cérebros` : '—', 'wide')}
  </div><div class="hint">Relatório do tick ${fmt(G.tick)}.</div></div>`;
}

/* ------------------------------ Demografia ------------------------------ */
const WIN_TICKS = { '5k': 5000, '30k': 30000, '100k': 100000, '25%': null, 'Tudo': Infinity };
function windowSamples() {
  const all = TEL.samples; if (!all.length) return [];
  const last = all[all.length - 1].tick;
  let from;
  if (S.demoWin === '25%') from = last * 0.75; else if (S.demoWin === 'Tudo') from = -1; else from = last - WIN_TICKS[S.demoWin];
  let i = all.length - 1; while (i > 0 && all[i - 1].tick >= from) i--;
  return all.slice(Math.max(0, i - 1));
}
function rate(series, key) {
  const out = []; for (let i = 1; i < series.length; i++) { const dt = series[i].tick - series[i - 1].tick; out.push(dt > 0 ? (series[i][key] - series[i - 1][key]) / dt * 1000 : 0); } return out;
}
function demoHeader() { if (TEL.error) return 'erro ao ler a telemetria: ' + esc(TEL.error); const w = windowSamples(); if (!w.length) return 'coletando telemetria…'; const a = w[0], b = w[w.length - 1]; return `${w.length} amostras · ticks ${fmt(a.tick)} a ${fmt(b.tick)} · nasc. ${fmt(b.births_total - a.births_total)} · mortes ${fmt(b.deaths_total - a.deaths_total)}`; }
function demoKpis() {
  const w = windowSamples(); const b = w[w.length - 1] || {}, a = w[0] || {};
  const live = F?.population ?? b.population, d = (live ?? 0) - (a.population ?? live ?? 0);
  return `<div class="kpis">${ro('Células vivas', fmt(live), '', 'cells')}${ro('Fitoplâncton vivo', fmt(F?.food_count ?? b.phytoplankton), '', 'food')}${ro('Ticks/s', fmt(F?.tps), '', 'tps')}${ro('Variação na janela', (d >= 0 ? '+' : '') + fmt(d))}${ro('Gerações', `máx ${fmt(b.max_generation)} · méd ${fmt(b.average_generation, 1)}`)}${ro('Diversidade genética', fmt((b.genetic_diversity ?? 0) * 100, 1) + '%')}</div>`;
}
function demoCharts(big) {
  const w = windowSamples(); const h = big ? 170 : 96;
  if (w.length < 2) return `<div class="empty">Os gráficos aparecem a partir da segunda amostra (a cada ${fmt(F?.telemetry_interval ?? 500)} ticks).</div>`;
  const ticks = w.map(s => s.tick);
  const card = (t, right, svg, legend = '') => `<div class="chartcard"><div class="ct"><b>${t}</b><span class="mono">${right}</span></div>${svg}${legend}</div>`;
  const lg = items => `<div class="legend">${items.map(([c, l]) => `<span><i style="background:${c}"></i>${l}</span>`).join('')}</div>`;
  const last = w[w.length - 1];
  const rt = ticks.slice(1);
  return [
    card('População viva', fmt(last.population) + ' células', lineChart(ticks, [{ d: w.map(s => s.population), c: 'var(--ac)' }], { h })),
    card('População de fitoplâncton', fmt(last.phytoplankton ?? 0) + ' partículas', lineChart(ticks, [{ d: w.map(s => s.phytoplankton ?? 0), c: 'var(--ok)' }], { h })),
    card('Velocidade da simulação', fmt(last.tps ?? 0) + ' ticks/s', lineChart(ticks, [{ d: w.map(s => s.tps ?? 0), c: 'var(--wa)' }], { h }), lg([['var(--wa)', 'ticks por segundo no momento da amostra']])),
    card('Energia média', fmt(last.average_energy, 1), lineChart(ticks, [{ d: w.map(s => s.average_energy), c: 'var(--ac)' }], { h, step: w.map(s => s.reproduction_minimum) }), lg([['var(--ac)', 'energia média'], ['var(--wa)', 'mín. reprodução (em vigor)']])),
    card('Idade média', fmt(last.average_age) + ' ticks', lineChart(ticks, [{ d: w.map(s => s.average_age), c: 'var(--ac)' }], { h })),
    card('Diversidade genética', fmt(last.genetic_diversity * 100, 1) + '%', lineChart(ticks, [{ d: w.map(s => s.genetic_diversity * 100), c: '#b48cf0' }], { h })),
    card('Gerações', `máx ${fmt(last.max_generation)} · méd ${fmt(last.average_generation, 1)}`, lineChart(ticks, [{ d: w.map(s => s.max_generation), c: 'var(--ac)' }, { d: w.map(s => s.average_generation), c: 'var(--mu)' }], { h }), lg([['var(--ac)', 'máxima'], ['var(--mu)', 'média']])),
    card('Natalidade', 'eventos / 1k ticks', lineChart(rt, [{ d: rate(w, 'births_neural'), c: 'var(--ok)' }, { d: rate(w, 'births_limit'), c: 'var(--wa)' }], { h }), lg([['var(--ok)', 'decisão neural'], ['var(--wa)', 'limite energético']])),
    card('Mortalidade', 'eventos / 1k ticks', lineChart(rt, [{ d: rate(w, 'deaths_starvation'), c: 'var(--dg)' }, { d: rate(w, 'deaths_age'), c: 'var(--mu)' }, { d: rate(w, 'deaths_phagocytosis'), c: '#b48cf0' }], { h }), lg([['var(--dg)', 'fome'], ['var(--mu)', 'idade'], ['#b48cf0', 'fagocitose']])),
    card('Energia atual', 'distribuição', histogram(h)),
  ].join('');
}
function lineChart(ticks, series, { h = 96, step }) {
  const w = 320, pad = 4, all = series.flatMap(s => s.d).concat(step || []).filter(Number.isFinite);
  if (!all.length || ticks.length < 2) return `<svg viewBox="0 0 ${w} ${h}" style="height:${h}px"></svg>`;
  let mn = Math.min(...all), mx = Math.max(...all); if (mx - mn < 1e-6) mx = mn + 1; const span = mx - mn; mn -= span * 0.08; mx += span * 0.08;
  const t0 = ticks[0], t1 = ticks[ticks.length - 1] || t0 + 1;
  const X = t => pad + (t - t0) / Math.max(1, t1 - t0) * (w - pad * 2), Y = v => h - pad - (v - mn) / (mx - mn) * (h - pad * 2);
  const path = d => d.map((v, i) => `${i ? 'L' : 'M'}${X(ticks[i]).toFixed(1)} ${Y(v).toFixed(1)}`).join('');
  const grid = [0.25, 0.5, 0.75].map(f => `<line x1="0" x2="${w}" y1="${h * f}" y2="${h * f}" stroke="var(--bd)"/>`).join('');
  const stepPath = step ? `<path d="${step.map((v, i) => `${i ? 'H' + X(ticks[i]).toFixed(1) + 'V' : 'M' + X(ticks[i]).toFixed(1) + ' '}${Y(v).toFixed(1)}`).join('')}" stroke="var(--wa)" stroke-dasharray="4 3" fill="none" stroke-width="1.2" vector-effect="non-scaling-stroke"/>` : '';
  const markers = S.events.filter(e => e.tick >= t0 && e.tick <= t1).map(e => { const x = X(e.tick); return `<line x1="${x}" x2="${x}" y1="0" y2="${h}" stroke="var(--wa)" stroke-dasharray="3 3" stroke-opacity=".7"><title>tick ${fmt(e.tick)} · ${esc(e.label)}</title></line><circle cx="${x}" cy="5" r="3" fill="var(--wa)"><title>tick ${fmt(e.tick)} · ${esc(e.label)}</title></circle>`; }).join('');
  return `<svg viewBox="0 0 ${w} ${h}" preserveAspectRatio="none" style="height:${h}px">${grid}${stepPath}${series.map(s => `<path d="${path(s.d)}" stroke="${s.c}" fill="none" stroke-width="1.5" vector-effect="non-scaling-stroke"/>`).join('')}${markers}</svg>`;
}
function histogram(h) {
  const w = 320, b = 14, cap = F?.energy_capacity || 1, counts = new Array(b).fill(0);
  (F?.energies || []).forEach(e => counts[clamp(Math.floor(e / cap * b), 0, b - 1)]++);
  const m = Math.max(...counts, 1), bw = w / b;
  return `<svg viewBox="0 0 ${w} ${h}" preserveAspectRatio="none" style="height:${h}px">${counts.map((k, i) => `<rect x="${i * bw + 1}" y="${h - k / m * (h - 6)}" width="${bw - 2}" height="${k / m * (h - 6)}" rx="2" fill="var(--ac)" fill-opacity=".75"><title>${fmt(i * cap / b)}–${fmt((i + 1) * cap / b)}: ${k}</title></rect>`).join('')}</svg>`;
}
function threadsControl() {
  const max = F?.max_threads || 1, req = F?.threads_requested ?? 0;
  const opts = [[0, 'Auto']].concat([1, 2, 4, 8, 16].filter(n => n < max).map(n => [n, String(n)])).concat(max > 1 ? [[max, String(max)]] : []);
  return `<div class="field"><div class="field-top"><label>Threads de cálculo</label><span class="unit mono" style="margin-left:auto" data-live="threads">em uso: ${fmt(F?.threads ?? 1)} de ${fmt(max)}</span></div>
    <div class="seg">${opts.map(([n, l]) => `<button class="${n === req ? 'on' : ''}" data-act="threads" data-v="${n}">${l}</button>`).join('')}</div>
    <div class="hint">O resultado da simulação é o mesmo com qualquer número de threads; só muda a velocidade.</div></div>`;
}
const SAMPLING = [250, 500, 1000, 2000, 5000];
function samplingControl() {
  const cur = F?.telemetry_interval ?? 500;
  return `<div class="field"><div class="field-top"><label>Coleta da demografia</label><span class="unit" style="margin-left:auto">a cada N ticks</span></div>
    <div class="seg">${SAMPLING.map(n => `<button class="${n === cur ? 'on' : ''}" data-act="sampling" data-v="${n}">${fmt(n)}</button>`).join('')}</div></div>`;
}
function tabDemografia() {
  const body = `<div class="sec-b" style="padding-top:12px">${samplingControl()}<div class="field"><div class="field-top"><label>Janela do gráfico</label></div>${segUI('demoWin', [['5k', '5k'], ['30k', '30k'], ['100k', '100k'], ['25%', '25%'], ['Tudo', 'Tudo']], S.demoWin)}</div>
    <div class="hint mono">${demoHeader()}</div>${demoKpis()}${demoCharts(false)}</div>`;
  return head('Demografia', `Telemetria da sessão · amostra a cada ${fmt(F?.telemetry_interval ?? 500)} ticks`, `<button class="btn sm" data-act="demoFull">${ic('maxim')}Expandir</button>`) + `<div class="side-body">${body}</div>`;
}

function tabCamadas() {
  const chemOpts = ['PG', 'PF', 'C0', 'C1', 'C2', 'C3', 'C4', 'C5', 'C6', 'C7', 'Todos'];
  const body = `
    ${section('l-world', 'Mundo', `
      ${toggle('flow', 'Vetores da corrente', 'Direção do fluxo; tamanho visual fixo', true)}
      ${toggle('sun', 'Iluminação', 'Regiões ensolaradas do mapa', true)}
      ${toggle('cones', 'Cones dos sensores', 'Alcance visual de cada célula', true)}
      ${toggle('conflicts', 'Conflitos de densidade', 'Células abaixo do espaço mínimo', true)}
    `)}
    ${section('l-chem', 'Mapa químico instantâneo', `
      ${toggle('chem', 'Mostrar mapa químico', 'Reconstruído a cada tick', true)}
      <div class="field"><div class="field-top"><label>Substância</label><span class="unit mono" style="margin-left:auto" data-live="peak">${F?.chem ? 'pico ' + fmt(F.chem.max, 2) + ' ppm' : ''}</span></div>
        <div class="seg" style="flex-wrap:wrap">${chemOpts.map(o => `<button style="flex:0 0 auto" class="${S.layers.chemSel === o ? 'on' : ''}" data-act="chemSel" data-v="${o}">${o}</button>`).join('')}</div></div>
    `)}
    ${section('l-future', 'Clima', `<div class="hint">Temperatura e umidade entram aqui quando existirem no motor.</div>`)}`;
  return head('Camadas', 'Só muda a visualização · vale na hora') + `<div class="side-body">${body}</div>`;
}

/* ------------------------------ Gavetas e modais ------------------------------ */
function renderLayer() {
  const L = $('#layer');
  if (S.drawer) { L.innerHTML = `<div class="scrim" data-act="closeLayer">${drawerContent(S.drawer)}</div>`; return; }
  if (S.modal) { L.innerHTML = `<div class="scrim" data-act="closeLayer">${modalContent(S.modal)}</div>`; return; }
  L.innerHTML = '';
}
function individualDrawer() {
  const v = VIEW;
  if (!v) return `<div class="drawer" data-stop="1"><div class="drawer-h"><div style="flex:1"><h3>Indivíduo</h3><p>Carregando…</p></div><button class="btn icon ghost" data-act="closeLayer">${ic('x')}</button></div></div>`;
  const d = { slots: new Array(56).fill(0), active: v.active, wires: v.wires.map(([a, b, w]) => [a, b, w, w / 4]), channels: v.genome_map?.channels };
  const when = v.created_at ? new Date(v.created_at * 1000).toLocaleString('pt-BR') : null;
  return `<div class="drawer" data-stop="1"><div class="drawer-h"><span class="cdot" style="background:${esc(v.color)};margin-top:2px"></span><div style="flex:1"><h3>${esc(v.name)}</h3><p>${esc(v.description)}</p></div><button class="btn icon ghost" data-act="closeLayer">${ic('x')}</button></div>
    <div class="drawer-b">
      <div class="readouts">${ro('Origem', esc(v.origin), 'wide')}${v.generation != null ? ro('Geração', fmt(v.generation)) : ''}${ro('Conexões ativas', fmt(v.connections))}${when ? ro('Salvo em', when, 'wide') : ''}${ro('Cor genética', esc(v.color))}</div>
      <div class="hint">Fiação do genoma (os sinais aparecem quando a célula está viva na arena).</div>
      ${neuralNet(d)}
      <div class="legend"><span><i style="background:var(--ok)"></i>peso positivo</span><span><i style="background:var(--dg)"></i>negativo</span><span><i style="background:var(--s4)"></i>nó inativo geneticamente</span></div>
      ${genomeDisclosure(v.genome, v.genome_map)}
    </div>
    <div class="drawer-f">${v.saved ? `<button class="btn danger ghost" data-act="askDelete" style="margin-right:auto">Apagar do banco</button>` : ''}<button class="btn" data-act="closeLayer">Fechar</button><button class="btn pri" data-act="useInd" data-v="${esc(v.key)}">Usar como configuração neural</button></div></div>`;
}
function drawerContent(d) {
  if (d === 'individual') return individualDrawer();
  if (d === 'runs') return `<div class="drawer" data-stop="1"><div class="drawer-h"><div style="flex:1"><h3>Estado da sessão</h3><p>Salvar e carregar runs completas (.ecosrun).</p></div><button class="btn icon ghost" data-act="closeLayer">${ic('x')}</button></div>
    <div class="drawer-b"><div class="soon"><b>Chega na próxima versão.</b><br>Esta é a primeira versão da interface nova. Salvar e carregar runs, presets de sessão e presets de mapa estão sendo portados do executável antigo. Até lá, use a <b>V1.1.0</b> (em <i>2 - Versões de teste</i>) para salvar uma run.</div></div>
    <div class="drawer-f"><button class="btn" data-act="closeLayer">Fechar</button></div></div>`;
  return '';
}
function modalContent(m) {
  const shell = (t, p, f) => `<div class="modal" data-stop="1"><div class="modal-b"><h3>${t}</h3>${p}</div><div class="modal-f">${f}</div></div>`;
  if (m === 'restart') return shell('Reiniciar a sessão?', `<p>A run atual (tick ${fmt(F?.tick)}) será descartada e uma nova começa com as condições iniciais${initPending().length ? ' editadas' : ''} e o mapa do editor.</p><p>Nesta versão ainda não dá para salvar a run antes.</p>`, `<button class="btn ghost" data-act="closeLayer">Cancelar</button><button class="btn pri" data-act="restart">Reiniciar</button>`);
  if (m === 'saveInd') {
    const d = F?.detail;
    if (!d) return shell('Salvar indivíduo', '<p>A célula selecionada não está mais viva.</p>', `<button class="btn" data-act="closeLayer">Fechar</button>`);
    return shell('Salvar indivíduo no banco', `<p>O genoma da célula #${d.id} (geração ${fmt(d.generation)}) fica guardado no banco e aparece em <b>Sessão › Configuração neural</b>, inclusive nas próximas versões.</p>
      <div class="field"><div class="field-top"><label>Nome</label></div><input class="txt" id="saveName" maxlength="60" value="Célula #${d.id} · gen ${d.generation}"></div>`,
      `<button class="btn ghost" data-act="closeLayer">Cancelar</button><button class="btn pri" data-act="saveInd" data-v="${d.id}">${ic('save')}Salvar</button>`);
  }
  if (m === 'delInd') return shell('Apagar indivíduo?', `<p>“${esc(VIEW?.name || '')}” será removido do banco. Runs em andamento não mudam.</p><p>Esta ação não pode ser desfeita.</p>`, `<button class="btn ghost" data-act="closeLayer">Cancelar</button><button class="btn danger" data-act="deleteInd" data-v="${esc(VIEW?.key || '')}">Apagar</button>`);
  if (m === 'headless') return shell('Ativar execução headless?', `<p>A simulação continua, mas a arena deixa de ser desenhada e o motor para de enviar o mundo para a tela. Você verá tick, população e ticks/s.</p><p>Use para medir o teto de cálculo; o estado biológico continua o mesmo.</p>`, `<button class="btn ghost" data-act="closeLayer">Cancelar</button><button class="btn pri" data-act="headlessOn">${ic('bolt')}Ativar</button>`);
  return '';
}

/* ------------------------------ Palco: overlays ------------------------------ */
function renderStage() {
  const ov = $('#stageOverlay');
  if (S.mode === 'lab') {
    ov.innerHTML = `<div class="lab"><div class="lab-in"><h1>Laboratório de Experimentos</h1><p class="lead">Campanhas offline em paralelo, sem renderizar durante o cálculo. Cada mundo grava configuração, resultado e replay.</p>
      <div class="cards3">
        <button class="bigcard" disabled>${ic('shuffle')}<b>Varredura de seeds aleatórias</b><span>Mesmo ambiente; vários começos aleatórios.</span></button>
        <button class="bigcard" disabled>${ic('storm')}<b>Estressar uma seed por ambiente</b><span>Uma condição inicial; o meio varia dentro de faixas.</span></button>
        <button class="bigcard" disabled>${ic('folder')}<b>Simulações salvas</b><span>Abrir, reproduzir ou duplicar campanhas.</span></button>
      </div>
      <div class="soon" style="margin-top:16px"><b>O Laboratório chega numa próxima versão da interface nova.</b> O layout já está reservado aqui; as campanhas ainda rodam no executável <span class="mono">laboratorio_experimentos</span> do código-fonte.</div></div></div>`;
    $('#layerChips').innerHTML = ''; $('#camCtl').innerHTML = ''; $('#stageBanner').innerHTML = ''; return;
  }
  if (F?.headless) {
    ov.innerHTML = `<div class="headless"><div class="hl-card"><h3>Execução headless</h3><div class="muted">Cálculo ativo sem desenhar a arena nem copiar o mundo para a tela.</div>
      <div class="hl-big">${ro('Tick', fmt(F.tick), '', 'tick')}${ro('População', fmt(F.population), '', 'cells')}${ro('Ticks/s', fmt(F.tps), '', 'tps')}</div>
      <div class="hint" style="margin-bottom:14px">O limite de velocidade da interface está suspenso para mostrar o teto de cálculo.</div>
      <div class="row"><button class="btn" data-act="pause">${ic(F.paused ? 'play' : 'pause')}${F.paused ? 'Retomar' : 'Pausar'}</button><div class="grow"></div><button class="btn pri" data-act="headlessOff">Voltar à visualização <span class="kbd">Esc</span></button></div></div></div>`;
    return;
  }
  if (S.demoFull) {
    ov.innerHTML = `<div class="demofull"><div class="demofull-h"><h2>Demografia da sessão</h2><span class="muted mono">${demoHeader()}</span><div class="grow"></div><div style="width:320px">${samplingControl()}</div><div style="width:300px">${segUI('demoWin', [['5k', '5k'], ['30k', '30k'], ['100k', '100k'], ['25%', '25%'], ['Tudo', 'Tudo']], S.demoWin)}</div><button class="btn" data-act="demoFull">${ic('x')}Fechar</button></div>
      <div class="demogrid">${demoKpis()}${demoCharts(true)}</div></div>`;
  } else ov.innerHTML = '';
  let b = '';
  const neural = neuralOption(S.init.neural);
  if (S.insert) b = `<div class="banner">${ic('plus')}<span>Inserir célula <b>${esc(neural.name)}</b> · clique no mapa</span><span class="kbd">Esc</span><button class="btn sm ghost" data-act="insert">Cancelar</button></div>`;
  else if (S.tab === 'mapa' && S.mapTool !== 'none') b = `<div class="banner wa">${ic(S.mapTool)}<span>Editando mapa · ${{ wall: 'Parede', eraser: 'Borracha', sun: 'Região ensolarada (contorno)' }[S.mapTool]}${S.mapTool !== 'sun' ? ' · raio ' + S.brush : ''}</span>${F?.map_dirty ? '<button class="btn sm warn" data-act="mapApply">Aplicar mapa</button>' : ''}<button class="btn sm ghost" data-act="tool" data-v="none">Sair</button></div>`;
  $('#stageBanner').innerHTML = b;
  renderChips();
  $('#camCtl').innerHTML = `<button class="btn sm ${S.follow ? 'pri' : ''}" data-act="follow" title="Seguir célula selecionada">${ic('target')}Seguir</button><button class="btn sm" data-act="fit">${ic('fit')}Encaixar</button><span class="pill mono" style="height:26px;background:rgba(20,23,26,.88)">${Math.round(cam.zoom * 100)}%</span>`;
}
function renderChips() {
  const L = S.layers, chips = [];
  if (L.chem) chips.push(`<span class="pill">mapa químico · ${L.chemSel}${F?.chem ? ` · pico ${fmt(F.chem.max, 2)} ppm` : ''}</span>`);
  if (L.flow) chips.push(`<span class="pill">corrente · ${S.envApplied.flow_preset}</span>`);
  if (L.sun) chips.push(`<span class="pill">iluminação</span>`);
  if (L.cones) chips.push(`<span class="pill">cones</span>`);
  if (L.conflicts) chips.push(`<span class="pill dg">conflitos</span>`);
  $('#layerChips').innerHTML = chips.join('');
}

/* ------------------------------ Barra de status ------------------------------ */
function renderStatus() {
  const m = S.msg ? `<span class="msg ${S.msg.ok ? 'ok' : ''}" style="${S.msg.ok === false ? 'color:var(--dg)' : ''}">${esc(S.msg.text)}</span>` : '';
  const ep = envPending().length;
  $('#status').innerHTML = `<span class="mono" data-live="pubhz">pub. ${fmt(F?.publish_hz)} Hz</span><span class="mono" data-live="fps">${fps} FPS</span>${ep ? `<span style="color:var(--wa)">${ep} ${ep === 1 ? 'alteração não aplicada' : 'alterações não aplicadas'}</span>` : ''}${S.insert ? '<span style="color:var(--ac)">inserção armada</span>' : ''}${m}
     <span class="proto"><span class="kbd">Espaço</span> pausa · <span class="kbd">1–7</span> velocidade · <span class="kbd">[ ]</span> células · <span class="kbd">Esc</span> cancela</span>`;
}
function say(text, ok = true) { S.msg = { text, ok }; renderStatus(); clearTimeout(say.t); say.t = setTimeout(() => { S.msg = null; renderStatus(); }, 5000); }
function renderAll() { renderTop(); renderRail(); renderPanel(); renderStage(); renderLayer(); renderStatus(); }

/* ------------------------------ Ações ------------------------------ */
const ACT = {
  mode(v) { S.mode = v; S.speedMenu = false; S.demoFull = false; renderAll(); },
  tab(v) { if (S.tab === v && !S.collapsed) S.collapsed = true; else { S.tab = v; S.collapsed = false; } if (v !== 'mapa') S.mapTool = 'none'; renderRail(); renderPanel(); renderStage(); },
  collapse() { S.collapsed = !S.collapsed; renderRail(); if (!S.collapsed) renderPanel(); },
  pause() { send({ type: 'pause', value: !F?.paused }); },
  speedMenu() { S.speedMenu = !S.speedMenu; renderTop(); },
  speed(v) { S.speedMenu = false; send({ type: 'speed', value: +v }); say('Velocidade ' + SPEEDS[+v]); renderTop(); },
  apply() {
    const ch = envPending(); if (!ch.length) return;
    ch.forEach(k => S.events.push({ tick: F?.tick ?? 0, label: `${ENV_LABELS[k]}: ${S.envApplied[k]} → ${S.env[k]}` }));
    S.envApplied = { ...S.env }; send({ type: 'apply', env: S.env }); renderAll();
  },
  revert() { S.env = { ...S.envApplied }; S.init = { ...S.initApplied }; renderAll(); say('Campos revertidos ao último valor aplicado'); },
  askRestart() { S.modal = 'restart'; S.drawer = null; renderLayer(); },
  restart() {
    S.modal = null; S.initApplied = { ...S.init }; S.envApplied = { ...S.env }; S.events = []; S.selected = null; S.follow = false;
    TEL = { epoch: 0, samples: [] }; GEN = null;
    send({ type: 'restart', init: S.init, env: S.env }); renderAll();
  },
  newSeed() { const a = Math.floor(Math.random() * 9e8 + 1e8), b = Math.floor(Math.random() * 1e9); S.init.seed = `${a}${String(b).padStart(9, '0')}`; renderPanel(); renderTop(); renderRail(); },
  neural(v) { S.init.neural = v; renderPanel(); renderTop(); renderRail(); },
  insert() { S.insert = !S.insert; if (S.insert) S.mapTool = 'none'; renderPanel(); renderStage(); renderStatus(); },
  setf(v, t) { setF(t.dataset.f, v); renderPanel(); renderTop(); renderRail(); renderStatus(); },
  sec(v) { const cur = S.sections[v] ?? true; S.sections[v] = !cur; const el = document.querySelector(`[data-act="sec"][data-v="${v}"]`)?.parentElement; if (el) el.classList.toggle('closed', cur); },
  cgroup(v) { S.cellGroups[v] = !S.cellGroups[v]; renderPanel(); },
  ui(v, t) { S[t.dataset.k] = v; renderPanel(); if (S.demoFull) renderStage(); if (t.dataset.k === 'cellsView' && v === 'gen') pollGenetics(true); },
  drawer(v) { S.drawer = v; S.modal = null; renderLayer(); },
  modal(v) { S.modal = v; S.drawer = null; renderLayer(); },
  closeLayer(v, t, e) { if (t.classList.contains('scrim') && e.target.closest('[data-stop]')) return; S.drawer = null; S.modal = null; renderLayer(); },
  headlessOn() { S.modal = null; renderLayer(); send({ type: 'headless', value: true }); },
  headlessOff() { send({ type: 'headless', value: false }); },
  layerOn(v) { S.layers[v] = true; renderStage(); say('Camada ligada: ' + ({ conflicts: 'conflitos de densidade', cones: 'cones dos sensores' }[v] || v)); },
  chemSel(v) { S.layers.chemSel = v; S.layers.chem = true; send({ type: 'chem', display: v }); renderPanel(); renderStage(); },
  tool(v) { S.mapTool = v; S.insert = false; renderPanel(); renderStage(); },
  mapApply() { send({ type: 'map_apply' }); },
  mapRevert() { send({ type: 'map_revert' }); },
  mapUndo() { send({ type: 'map_undo' }); },
  mapClearSun() { send({ type: 'map_clear_sun' }); },
  cellNav(v) {
    if (!F || !F.ids.length) return;
    const i = Math.max(0, F.ids.indexOf(S.selected));
    selectCell(F.ids[(i + (+v) + F.ids.length) % F.ids.length]);
  },
  follow() { S.follow = !S.follow; if (S.follow && S.selected == null && F?.ids.length) selectCell(F.ids[0]); renderStage(); if (S.tab === 'celulas') renderPanel(); },
  fit() { cam.zoom = 1; cam.cx = INFO.world_width / 2; cam.cy = INFO.world_height / 2; S.follow = false; renderStage(); },
  openRep(v) { selectCell(+v); S.cellsView = 'ind'; renderPanel(); },
  demoFull() { S.demoFull = !S.demoFull; renderStage(); },
  threads(v) { send({ type: 'threads', count: +v }); setTimeout(() => { if (S.tab === 'sessao') renderPanel(); }, 200); },
  sampling(v) { send({ type: 'telemetry_interval', ticks: +v }); setTimeout(() => { if (S.tab === 'demografia') renderPanel(); if (S.demoFull) renderStage(); }, 150); },
  toggleRecords() { S.showRecords = !S.showRecords; if (S.drawer === 'individual') renderLayer(); else renderPanel(); },
  askSave() { S.modal = 'saveInd'; S.drawer = null; renderLayer(); setTimeout(() => { const i = $('#saveName'); if (i) { i.focus(); i.select(); } }, 30); },
  saveInd(v) { const name = $('#saveName')?.value || ''; S.modal = null; renderLayer(); send({ type: 'save_individual', id: +v, name }); },
  async viewInd(v) { VIEW = null; S.drawer = 'individual'; S.modal = null; renderLayer(); try { VIEW = await invoke('individual_view', { key: v }); } catch (err) { console.error(err); } if (!VIEW) { S.drawer = null; say('Não foi possível abrir este indivíduo', false); } renderLayer(); },
  useInd(v) { S.init.neural = v; S.drawer = null; renderLayer(); renderAll(); say('Configuração escolhida; vale ao reiniciar (ou ao inserir célula)'); },
  askDelete() { S.modal = 'delInd'; S.drawer = null; renderLayer(); },
  deleteInd(v) { S.modal = null; renderLayer(); send({ type: 'delete_individual', key: v }); say('Indivíduo apagado do banco'); },
};
async function refreshCatalog() {
  try {
    CATALOG = await invoke('neural_catalog');
    if (!CATALOG.options.some(o => o.key === S.init.neural)) S.init.neural = 'classica';
    if (!CATALOG.options.some(o => o.key === S.initApplied.neural)) S.initApplied.neural = 'classica';
    if (S.tab === 'sessao') renderPanel();
  } catch (err) { console.error(err); }
}
function selectCell(id) {
  S.selected = id; send({ type: 'select', id });
  if (S.mode !== 'lab') { S.tab = 'celulas'; S.cellsView = 'ind'; S.collapsed = false; renderRail(); }
  pendingDetailRender = true;
}
let pendingDetailRender = false;

/* ------------------------------ Eventos globais ------------------------------ */
document.addEventListener('click', e => {
  if (S.speedMenu && !e.target.closest('.speed')) { S.speedMenu = false; renderTop(); }
  const t = e.target.closest('[data-act]'); if (!t || t.disabled) return;
  const fn = ACT[t.dataset.act]; if (fn) fn(t.dataset.v, t, e);
});
document.addEventListener('input', e => {
  const t = e.target;
  if (t.dataset.f) {
    const path = t.dataset.f; let v;
    if (t.type === 'checkbox') v = t.checked;
    else if (t.type === 'text') v = t.value;
    else { v = parseFloat(t.value); if (isNaN(v)) return; v = v / (parseFloat(t.dataset.scale) || 1); }
    if (path === 'keepAwake') { S.keepAwake = v; send({ type: 'keep_awake', value: v }); return; }
    if (['init.cells', 'env.lifetime'].includes(path)) v = Math.round(v);
    setF(path, v);
    document.querySelectorAll(`[data-f="${path}"]`).forEach(o => { if (o !== t) { if (o.type === 'checkbox') o.checked = t.checked; else o.value = t.value; } });
    document.querySelectorAll(`[data-row="${path}"]`).forEach(r => r.classList.toggle('dirty', isDirty(path)));
    if (path === 'env.lifetime') { const el = document.querySelector('[data-live="lifeSun"]'); if (el) el.textContent = fmt(Math.round(v * 1.5)) + ' ticks'; }
    renderTop(); renderRail(); renderStatus();
    const pb = document.querySelector('[data-pending="bio"]'); if (pb) pb.innerHTML = pendingPill(bioPending());
    const pp = document.querySelector('[data-pending="phys"]'); if (pp) pp.innerHTML = pendingPill(physPending());
    return;
  }
  if (t.dataset.layer) {
    const k = t.dataset.layer; S.layers[k] = t.checked; document.querySelectorAll(`[data-layer="${k}"]`).forEach(o => o.checked = t.checked);
    if (k === 'chem') send({ type: 'chem', display: t.checked ? S.layers.chemSel : null });
    if (k === 'flow' && t.checked) FLOW.key = '';
    renderStage(); return;
  }
  if (t.dataset.g === 'brush') { S.brush = clamp(+t.value || 0, 0, 12); document.querySelectorAll('[data-g="brush"]').forEach(o => { if (o !== t) o.value = t.value; }); renderStage(); return; }
});
document.addEventListener('keydown', e => {
  const typing = e.target.matches('input,textarea,select');
  if (e.target.id === 'saveName' && e.key === 'Enter') { const b = document.querySelector('[data-act="saveInd"]'); if (b) ACT.saveInd(b.dataset.v); return; }
  if (e.target.dataset?.g === 'cellSearch' && e.key === 'Enter') { const id = +e.target.value; if (F?.ids.includes(id)) selectCell(id); else say('Nenhuma célula viva com esse id', false); return; }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') { e.preventDefault(); if (S.mode !== 'lab') ACT.drawer('runs'); return; }
  if (e.key === 'Escape') {
    if (S.drawer || S.modal) { S.drawer = null; S.modal = null; renderLayer(); return; }
    if (F?.headless) { ACT.headlessOff(); return; }
    if (S.insert) { ACT.insert(); return; }
    if (S.mapTool !== 'none') { ACT.tool('none'); return; }
    if (S.demoFull) { ACT.demoFull(); return; }
    return;
  }
  if (typing || S.mode === 'lab') return;
  if (e.key === ' ') { e.preventDefault(); ACT.pause(); }
  else if (/^[1-7]$/.test(e.key)) ACT.speed(+e.key - 1);
  else if (e.key === '[') ACT.cellNav(-1);
  else if (e.key === ']') ACT.cellNav(1);
});
(function resizer() {
  const r = $('#resizer'); let drag = false;
  r.addEventListener('mousedown', e => { drag = true; r.classList.add('drag'); e.preventDefault(); });
  window.addEventListener('mousemove', e => { if (!drag) return; const w = clamp(window.innerWidth - e.clientX - 56, 320, 560); document.documentElement.style.setProperty('--sw', w + 'px'); });
  window.addEventListener('mouseup', () => { drag = false; r.classList.remove('drag'); });
})();

/* ------------------------------ Arena ------------------------------ */
const cv = $('#arena'), ctx = cv.getContext('2d');
const cam = { zoom: 1, cx: 0, cy: 0 };
const view = { s: 1, ox: 0, oy: 0 };
const mouse = { x: -1, y: -1, inside: false, down: false, moved: false, sx: 0, sy: 0, btn: 0, lasso: [] };
const wallCv = document.createElement('canvas'), sunCv = document.createElement('canvas'), chemCv = document.createElement('canvas');
let lastMapVersion = -1;
function buildMap(m) {
  wallCv.width = sunCv.width = m.width; wallCv.height = sunCv.height = m.height;
  const wc = wallCv.getContext('2d'), sc = sunCv.getContext('2d');
  const wi = wc.createImageData(m.width, m.height), si = sc.createImageData(m.width, m.height);
  for (let i = 0; i < m.walls.length; i++) {
    if (m.walls[i]) { wi.data[i * 4] = 78; wi.data[i * 4 + 1] = 88; wi.data[i * 4 + 2] = 100; wi.data[i * 4 + 3] = 255; }
    if (m.sun[i]) { si.data[i * 4] = 240; si.data[i * 4 + 1] = 200; si.data[i * 4 + 2] = 90; si.data[i * 4 + 3] = 34; }
  }
  wc.putImageData(wi, 0, 0); sc.putImageData(si, 0, 0);
}
// O canvas precisa ter o mesmo tamanho que ocupa na tela; senão o desenho é
// esticado e o clique erra a célula. A área da arena muda sem a janela mudar
// (painel lateral arrastado, recolhido ou alargado, tela cheia da Demografia),
// por isso observamos o próprio canvas e ainda conferimos a cada quadro.
function resize() {
  const r = cv.getBoundingClientRect(), d = window.devicePixelRatio || 1;
  const w = Math.max(1, Math.round(r.width * d)), h = Math.max(1, Math.round(r.height * d));
  if (cv.width !== w) cv.width = w;
  if (cv.height !== h) cv.height = h;
}
window.addEventListener('resize', resize);
if (window.ResizeObserver) new ResizeObserver(resize).observe(cv);
function cellIndex(id) { return F ? F.ids.indexOf(id) : -1; }
function computeView() {
  const d = window.devicePixelRatio || 1, cw = cv.width / d, ch = cv.height / d, pad = 24;
  const W = INFO.world_width, H = INFO.world_height;
  const fit = Math.min((cw - pad * 2) / W, (ch - pad * 2) / H);
  if (S.follow && S.selected != null) { const i = cellIndex(S.selected); if (i >= 0) { cam.cx += (F.xs[i] - cam.cx) * 0.15; cam.cy += (F.ys[i] - cam.cy) * 0.15; } }
  view.s = fit * cam.zoom; view.ox = cw / 2 - cam.cx * view.s; view.oy = ch / 2 - cam.cy * view.s;
}
const toScreen = (x, y) => [view.ox + x * view.s, view.oy + y * view.s];
const toWorld = (x, y) => [(x - view.ox) / view.s, (y - view.oy) / view.s];
function cellAt(wx, wy) {
  if (!F) return -1; let best = -1, bd = 1e18;
  for (let i = 0; i < F.ids.length; i++) { const dx = F.xs[i] - wx, dy = F.ys[i] - wy, d2 = dx * dx + dy * dy, r = F.radii[i] + 8 / view.s; if (d2 < r * r && d2 < bd) { bd = d2; best = i; } }
  return best;
}
function isWallAt(wx, wy) {
  if (!MAP) return false; const gx = Math.floor(wx / MAP.world_width * MAP.width), gy = Math.floor(wy / MAP.world_height * MAP.height);
  if (gx < 0 || gy < 0 || gx >= MAP.width || gy >= MAP.height) return true; return MAP.walls[gy * MAP.width + gx] === 1;
}
function draw() {
  resize();
  const d = window.devicePixelRatio || 1;
  ctx.setTransform(d, 0, 0, d, 0, 0);
  const cw = cv.width / d, ch = cv.height / d;
  ctx.clearRect(0, 0, cw, ch);
  if (!F || !INFO) return;
  computeView();
  const W = INFO.world_width, H = INFO.world_height;
  const [x0, y0] = toScreen(0, 0), ww = W * view.s, hh = H * view.s;
  ctx.fillStyle = '#121518'; ctx.fillRect(x0, y0, ww, hh);
  ctx.save(); ctx.beginPath(); ctx.rect(x0, y0, ww, hh); ctx.clip();
  ctx.strokeStyle = 'rgba(255,255,255,.025)'; ctx.lineWidth = 1;
  const g = 800;
  for (let gx = 0; gx <= W; gx += g) { const [sx] = toScreen(gx, 0); ctx.beginPath(); ctx.moveTo(sx, y0); ctx.lineTo(sx, y0 + hh); ctx.stroke(); }
  for (let gy = 0; gy <= H; gy += g) { const [, sy] = toScreen(0, gy); ctx.beginPath(); ctx.moveTo(x0, sy); ctx.lineTo(x0 + ww, sy); ctx.stroke(); }
  if (S.layers.chem && F.chem) drawChem(x0, y0, ww, hh);
  if (MAP && (S.layers.sun || (S.tab === 'mapa' && S.mapTool === 'sun'))) { ctx.imageSmoothingEnabled = true; ctx.drawImage(sunCv, x0, y0, ww, hh); }
  if (MAP) { ctx.imageSmoothingEnabled = false; ctx.drawImage(wallCv, x0, y0, ww, hh); }
  if (S.layers.flow) drawFlow();
  // fitoplâncton
  ctx.fillStyle = 'rgba(140,210,140,.85)';
  const fr = Math.max(1.1, 5 * view.s);
  for (let i = 0; i < F.food.length; i += 2) { const [sx, sy] = toScreen(F.food[i], F.food[i + 1]); if (sx < -4 || sy < -4 || sx > cw + 4 || sy > ch + 4) continue; ctx.fillRect(sx - fr, sy - fr, fr * 2, fr * 2); }
  // células
  const selIdx = S.selected != null ? cellIndex(S.selected) : -1;
  for (let i = 0; i < F.ids.length; i++) {
    const [sx, sy] = toScreen(F.xs[i], F.ys[i]); const r = Math.max(2.4, F.radii[i] * view.s);
    if (sx < -r - 50 || sy < -r - 50 || sx > cw + r + 50 || sy > ch + r + 50) continue;
    const c = F.colors[i], R = c >> 16 & 255, G = c >> 8 & 255, B = c & 255;
    if (S.layers.cones) { ctx.strokeStyle = `rgba(${R},${G},${B},${i === selIdx ? .6 : .14})`; ctx.lineWidth = 1; ctx.beginPath(); ctx.arc(sx, sy, F.ranges[i] * view.s, 0, 6.283); ctx.stroke(); }
    ctx.fillStyle = `rgba(${R},${G},${B},.92)`; ctx.beginPath(); ctx.arc(sx, sy, r, 0, 6.283); ctx.fill();
    if (r > 4) { ctx.fillStyle = 'rgba(255,255,255,.28)'; ctx.beginPath(); ctx.arc(sx - r * 0.28, sy - r * 0.28, r * 0.3, 0, 6.283); ctx.fill(); }
    if (S.layers.conflicts && F.crowded[i]) { ctx.strokeStyle = 'rgba(229,104,107,.95)'; ctx.lineWidth = 1.5; ctx.beginPath(); ctx.arc(sx, sy, r + 2.5, 0, 6.283); ctx.stroke(); }
  }
  if (selIdx >= 0) {
    const [sx, sy] = toScreen(F.xs[selIdx], F.ys[selIdx]), r = Math.max(2, F.radii[selIdx] * view.s);
    ctx.strokeStyle = '#fff'; ctx.lineWidth = 2; ctx.beginPath(); ctx.arc(sx, sy, r + 4, 0, 6.283); ctx.stroke();
    ctx.strokeStyle = 'rgba(90,169,230,.6)'; ctx.lineWidth = 1; ctx.beginPath(); ctx.arc(sx, sy, r + 9, 0, 6.283); ctx.stroke();
    if (S.layers.cones) { ctx.setLineDash([3, 4]); ctx.strokeStyle = 'rgba(255,255,255,.4)'; const R = F.ranges[selIdx] * view.s; for (let k = 0; k < 4; k++) { const a = Math.PI / 4 + k * Math.PI / 2; ctx.beginPath(); ctx.moveTo(sx, sy); ctx.lineTo(sx + Math.cos(a) * R, sy + Math.sin(a) * R); ctx.stroke(); } ctx.setLineDash([]); }
  }
  // hover
  const tip = $('#tip');
  if (mouse.inside && !S.insert && !(S.tab === 'mapa' && S.mapTool !== 'none')) {
    const [wx, wy] = toWorld(mouse.x, mouse.y); const h = cellAt(wx, wy);
    if (h >= 0) {
      const [sx, sy] = toScreen(F.xs[h], F.ys[h]), r = Math.max(2, F.radii[h] * view.s);
      ctx.strokeStyle = 'rgba(255,255,255,.55)'; ctx.lineWidth = 1; ctx.beginPath(); ctx.arc(sx, sy, r + 3, 0, 6.283); ctx.stroke();
      tip.style.display = 'block'; tip.style.left = (mouse.x + 14) + 'px'; tip.style.top = (mouse.y + 12) + 'px';
      tip.innerHTML = `<b>Célula #${F.ids[h]}</b><br><span class="muted">energia</span> <span class="mono">${fmt(F.energies[h])}</span> · <span class="muted">raio</span> <span class="mono">${fmt(F.radii[h], 1)}</span><br><span class="muted">clique para abrir</span>`;
      cv.style.cursor = 'pointer';
    } else { tip.style.display = 'none'; cv.style.cursor = mouse.down && mouse.moved ? 'grabbing' : 'default'; }
  } else tip.style.display = 'none';
  // fantasma de inserção
  if (S.insert && mouse.inside) {
    const [wx, wy] = toWorld(mouse.x, mouse.y), rr = INFO.cell_radius * 1.1;
    let ok = !isWallAt(wx, wy) && wx > rr && wy > rr && wx < W - rr && wy < H - rr;
    if (ok) for (let i = 0; i < F.ids.length; i++) { const dx = F.xs[i] - wx, dy = F.ys[i] - wy; if (dx * dx + dy * dy < (F.radii[i] + rr) ** 2) { ok = false; break; } }
    S.insertOk = ok;
    ctx.fillStyle = ok ? 'rgba(90,169,230,.35)' : 'rgba(229,104,107,.3)'; ctx.strokeStyle = ok ? '#5aa9e6' : '#e5686b'; ctx.lineWidth = 1.5; ctx.setLineDash([4, 3]);
    ctx.beginPath(); ctx.arc(mouse.x, mouse.y, Math.max(5, rr * view.s), 0, 6.283); ctx.fill(); ctx.stroke(); ctx.setLineDash([]);
    cv.style.cursor = ok ? 'crosshair' : 'not-allowed';
  }
  // pincel e laço do mapa
  if (S.tab === 'mapa' && S.mapTool !== 'none' && mouse.inside && MAP) {
    if (S.mapTool === 'sun') {
      cv.style.cursor = 'crosshair';
      if (mouse.lasso.length > 1) { ctx.strokeStyle = '#e3b341'; ctx.fillStyle = 'rgba(227,179,65,.12)'; ctx.lineWidth = 1.5; ctx.beginPath(); mouse.lasso.forEach(([x, y], i) => { const [sx, sy] = toScreen(x, y); i ? ctx.lineTo(sx, sy) : ctx.moveTo(sx, sy); }); ctx.closePath(); ctx.fill(); ctx.stroke(); }
    } else {
      const cell = MAP.world_width / MAP.width;
      ctx.strokeStyle = S.mapTool === 'eraser' ? '#e5686b' : '#cfd6dd'; ctx.lineWidth = 1.5; ctx.beginPath(); ctx.arc(mouse.x, mouse.y, Math.max(3, (S.brush + 0.5) * cell * view.s), 0, 6.283); ctx.stroke();
      cv.style.cursor = 'none';
    }
  }
  ctx.restore();
  ctx.strokeStyle = '#2a3036'; ctx.lineWidth = 1; ctx.strokeRect(x0 + .5, y0 + .5, ww - 1, hh - 1);
}
function drawChem(x0, y0, ww, hh) {
  const c = F.chem; chemCv.width = c.width; chemCv.height = c.height;
  const g = chemCv.getContext('2d'), img = g.createImageData(c.width, c.height);
  const col = { PG: [90, 169, 230], PF: [110, 205, 120] }[S.layers.chemSel] || [190, 140, 240];
  for (let i = 0; i < c.values.length; i++) { img.data[i * 4] = col[0]; img.data[i * 4 + 1] = col[1]; img.data[i * 4 + 2] = col[2]; img.data[i * 4 + 3] = Math.floor(c.values[i] * 0.55); }
  g.putImageData(img, 0, 0); ctx.imageSmoothingEnabled = true; ctx.drawImage(chemCv, x0, y0, ww, hh);
}
function drawFlow() {
  const key = `${S.envApplied.flow_preset}|${S.envApplied.flow_intensity}|${S.envApplied.flow_on}|${F.map_version}`;
  if (FLOW.key !== key && !FLOW.loading) {
    FLOW.loading = true;
    invoke('flow_vectors', { columns: 48, rows: 27 }).then(v => { FLOW = { key, data: v }; }).catch(() => { FLOW.loading = false; });
  }
  const v = FLOW.data; ctx.fillStyle = 'rgba(160,175,190,.38)';
  for (let i = 0; i < v.length; i += 4) {
    const m = Math.hypot(v[i + 2], v[i + 3]); if (m < 0.05) continue;
    const [sx, sy] = toScreen(v[i], v[i + 1]), a = Math.atan2(v[i + 3], v[i + 2]), L = 8;
    ctx.save(); ctx.translate(sx, sy); ctx.rotate(a); ctx.beginPath(); ctx.moveTo(L, 0); ctx.lineTo(-L * 0.6, -L * 0.45); ctx.lineTo(-L * 0.6, L * 0.45); ctx.closePath(); ctx.fill(); ctx.restore();
  }
}
let lastStroke = 0;
function stroke(wx, wy) {
  const now = performance.now(); if (now - lastStroke < 25) return; lastStroke = now;
  send({ type: 'map_stroke', erase: S.mapTool === 'eraser', x: wx, y: wy, radius: S.brush });
}
cv.addEventListener('mousemove', e => {
  const r = cv.getBoundingClientRect(); mouse.x = e.clientX - r.left; mouse.y = e.clientY - r.top; mouse.inside = true;
  if (!mouse.down) return;
  if (Math.abs(mouse.x - mouse.sx) + Math.abs(mouse.y - mouse.sy) > 4) mouse.moved = true;
  const editing = S.tab === 'mapa' && S.mapTool !== 'none' && mouse.btn === 0;
  if (editing) { const [wx, wy] = toWorld(mouse.x, mouse.y); if (S.mapTool === 'sun') { const lp = mouse.lasso[mouse.lasso.length - 1]; if (!lp || Math.hypot(lp[0] - wx, lp[1] - wy) > 40) mouse.lasso.push([wx, wy]); } else stroke(wx, wy); }
  else if (mouse.moved) { cam.cx -= e.movementX / view.s; cam.cy -= e.movementY / view.s; S.follow = false; }
});
cv.addEventListener('mouseleave', () => { mouse.inside = false; if (mouse.down && mouse.lasso.length) finishLasso(); mouse.down = false; });
cv.addEventListener('mousedown', e => {
  mouse.down = true; mouse.moved = false; mouse.sx = mouse.x; mouse.sy = mouse.y; mouse.btn = e.button;
  if (e.button === 2 && S.insert) { ACT.insert(); return; }
  if (S.tab === 'mapa' && S.mapTool !== 'none' && e.button === 0) { const [wx, wy] = toWorld(mouse.x, mouse.y); if (S.mapTool === 'sun') mouse.lasso = [[wx, wy]]; else { lastStroke = 0; stroke(wx, wy); } }
});
function finishLasso() { if (mouse.lasso.length >= 3) send({ type: 'map_sun', points: mouse.lasso }); mouse.lasso = []; }
cv.addEventListener('contextmenu', e => e.preventDefault());
cv.addEventListener('mouseup', e => {
  const click = mouse.down && !mouse.moved; mouse.down = false;
  if (S.tab === 'mapa' && S.mapTool === 'sun' && e.button === 0) { finishLasso(); return; }
  if (!click || e.button !== 0) return;
  const [wx, wy] = toWorld(mouse.x, mouse.y);
  if (S.insert) {
    if (!S.insertOk) { say('Posição inválida: parede, borda ou célula no lugar', false); return; }
    send({ type: 'insert', x: wx, y: wy, neural: S.init.neural }); S.insert = false; renderPanel(); renderStage(); renderStatus(); return;
  }
  if (S.tab === 'mapa' && S.mapTool !== 'none') return;
  const h = cellAt(wx, wy); if (h >= 0) selectCell(F.ids[h]);
});
cv.addEventListener('wheel', e => {
  e.preventDefault(); const [wx, wy] = toWorld(mouse.x, mouse.y);
  cam.zoom = clamp(cam.zoom * (e.deltaY < 0 ? 1.15 : 1 / 1.15), 1, 40);
  const d = window.devicePixelRatio || 1, cw = cv.width / d, ch = cv.height / d, fit = Math.min((cw - 48) / INFO.world_width, (ch - 48) / INFO.world_height), s = fit * cam.zoom;
  cam.cx = wx - (mouse.x - cw / 2) / s; cam.cy = wy - (mouse.y - ch / 2) / s;
  if (cam.zoom === 1) { cam.cx = INFO.world_width / 2; cam.cy = INFO.world_height / 2; }
  renderStage();
}, { passive: false });

/* ------------------------------ Laços de dados ------------------------------ */
let fps = 60, frames = 0, fpsT = performance.now(), lastPaused = null, lastSpeed = null, lastHeadless = null, lastMapDirty = null;
let frameBusy = false;
async function pollFrame() {
  if (frameBusy) return; frameBusy = true;
  try {
    const f = await invoke('frame');
    const prev = F; F = f;
    if (F.notice && F.notice.seq !== S.lastNoticeSeq) { S.lastNoticeSeq = F.notice.seq; if (F.notice.text) say(F.notice.text, F.notice.ok); }
    if (F.bank_version !== lastBankVersion) { lastBankVersion = F.bank_version; refreshCatalog(); }
    if (F.map_version !== lastMapVersion) { lastMapVersion = F.map_version; MAP = await invoke('map_data'); buildMap(MAP); }
    if (F.paused !== lastPaused || F.speed !== lastSpeed) { lastPaused = F.paused; lastSpeed = F.speed; renderTop(); }
    if (F.headless !== lastHeadless) { lastHeadless = F.headless; renderStage(); }
    if (F.map_dirty !== lastMapDirty) { lastMapDirty = F.map_dirty; renderRail(); if (S.tab === 'mapa') { renderPanel(); renderStage(); } }
    if (S.selected != null && !F.detail && prev && prev.detail) { pendingDetailRender = true; }
    if (pendingDetailRender && S.tab === 'celulas') { pendingDetailRender = false; renderPanel(); }
  } catch (err) { console.error(err); }
  frameBusy = false;
}
async function pollTelemetry() {
  try {
    TEL.error = null;
    const b = await invoke('telemetry', { from: TEL.samples.length, epoch: TEL.epoch });
    if (b.epoch !== TEL.epoch) TEL = { epoch: b.epoch, samples: b.samples };
    else if (b.start === TEL.samples.length) TEL.samples.push(...b.samples);
    else TEL.samples = TEL.samples.slice(0, b.start).concat(b.samples);
    if (b.total < TEL.samples.length) { TEL = { epoch: 0, samples: [] }; }
  } catch (err) { console.error(err); TEL.error = String(err); }
}
async function pollGenetics(force) {
  if (!force && !(S.tab === 'celulas' && S.cellsView === 'gen' && !S.collapsed && S.mode === 'sessao')) return;
  try { const g = await invoke('genetics'); if (g) { GEN = g; if (S.tab === 'celulas' && S.cellsView === 'gen') renderPanel(); } } catch (err) { console.error(err); }
}
function updateLive() {
  if (!F) return;
  const set = (k, v) => document.querySelectorAll(`[data-live="${k}"]`).forEach(el => { if (el.textContent !== v) el.textContent = v; });
  set('tick', fmt(F.tick)); set('cells', fmt(F.population)); set('tps', fmt(F.tps)); set('food', fmt(F.food_count)); set('cap', fmt(F.capacity_tps) + ' ticks/s'); set('threads', `em uso: ${fmt(F.threads ?? 1)} de ${fmt(F.max_threads ?? 1)}`);
  set('fps', fps + ' FPS'); set('pubhz', `pub. ${fmt(F.publish_hz)} Hz`); set('pub', `${fmt(F.publish_hz)} Hz`);
  set('run', '#' + String(F.run_id).slice(-7)); set('conflicts', `${fmt(F.crowding_pairs)} pares em conflito`);
  if (F.chem) set('peak', 'pico ' + fmt(F.chem.max, 2) + ' ppm');
  const d = F.detail;
  if (d && S.tab === 'celulas' && S.cellsView === 'ind') {
    if (!document.querySelector(`.cellhead`) || !document.querySelector('.cellhead b')?.textContent.includes('#' + d.id)) { renderPanel(); return; }
    set('cE', `${fmt(d.energy, 1)} / ${fmt(d.capacity)}`); set('cAge', `${fmt(d.age)} / ${fmt(d.max_age)}`); set('cKids', fmt(d.children));
    set('cConn', `${d.connections} / ${d.active_nodes}`); set('cSim', d.similar_id != null ? `#${d.similar_id} · ${fmt(d.similar * 100, 1)}%` : 'sem outra célula');
    set('cRad', fmt(d.radius, 1) + ' u'); set('cSize', fmt(d.radius / INFO.cell_radius, 2) + ' R'); set('cRange', fmt(d.sensor_range) + ' u'); set('cCost', fmt(d.sensor_cost, 4) + '/tick');
    const b = document.querySelector('[data-live="cEbar"]'); if (b) b.style.width = clamp(d.energy / d.capacity * 100, 0, 100) + '%';
    document.querySelectorAll('[data-nn]').forEach(n => { const i = +n.dataset.nn; if (!d.active[i]) return; const v = d.slots[i] || 0; n.setAttribute('fill', sigColor(v)); n.setAttribute('r', 3 + Math.min(1, Math.abs(v)) * 2); });
    document.querySelectorAll('[data-w]').forEach(p => { const w = d.wires[+p.dataset.w]; if (w) p.setAttribute('stroke-opacity', 0.15 + Math.min(1, Math.abs(w[3])) * 0.7); });
  } else if (S.selected != null && !d && S.tab === 'celulas' && S.cellsView === 'ind' && document.querySelector('.cellhead')) {
    say(`A célula #${S.selected} morreu`, false); S.selected = null; S.follow = false; send({ type: 'select', id: null }); renderPanel();
  }
}
function loop(now) {
  frames++; if (now - fpsT > 1000) { fps = Math.round(frames * 1000 / (now - fpsT)); frames = 0; fpsT = now; }
  if (!F?.headless && S.mode !== 'lab') draw();
  requestAnimationFrame(loop);
}

/* ------------------------------ Início ------------------------------ */
async function boot() {
  if (!TAURI) { $('#boot').innerHTML = '<b>New Gods Unlimited</b><span>Esta página precisa rodar dentro do aplicativo.</span>'; return; }
  INFO = await invoke('info');
  S.env = { ...INFO.env }; S.envApplied = { ...INFO.env };
  S.init = { ...INFO.init }; S.initApplied = { ...INFO.init };
  cam.cx = INFO.world_width / 2; cam.cy = INFO.world_height / 2;
  resize();
  await pollFrame(); await pollTelemetry();
  $('#boot').remove();
  renderAll();
  setInterval(pollFrame, 33);
  setInterval(updateLive, 250);
  setInterval(async () => { await pollTelemetry(); if (S.tab === 'demografia' && !S.collapsed) renderPanel(); if (S.demoFull) renderStage(); }, 1500);
  setInterval(() => pollGenetics(false), 2000);
  setInterval(renderChips, 1000);
  requestAnimationFrame(loop);
}
boot().catch(err => { console.error(err); const b = $('#boot'); if (b) b.innerHTML = `<b>Não foi possível iniciar</b><span class="mono">${esc(err)}</span>`; });
