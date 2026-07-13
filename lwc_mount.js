// Loaded lazily inside mount() — a *static* top-level CDN import here becomes a hard dependency of
// the wasm hydration bundle (site.js imports this snippet), so a slow/failed CDN would silently
// stall hydration and blank the whole app.
const CDN = "https://cdn.jsdelivr.net/npm/lightweight-charts@5/dist/lightweight-charts.standalone.production.mjs";
let _lib;
const lib = async () => (_lib ??= await import(CDN));

const GREY = '#88888855';
// axis is log-return space; show tags/ticks as the % move they represent, matching the legend
const PCT = { type: 'custom', minMove: 0.0001, formatter: v => (v >= 0 ? '+' : '') + ((Math.exp(v) - 1) * 100).toFixed(1) + '%' };

let chart, seriesByPair = new Map(), bulk, bulkHost;

// The grey background pairs, drawn as one canvas pass pinned to the chart's scales.
// Non-interactive (no crosshair / legend), but zooms & pans with the real series.
class BulkLines {
  // lines: [{ time:[sec…], value:[…] }] — each with its own domain (late listings, market-hours gaps)
  setData(lines) {
    this._lines = lines;
    let lo = Infinity, hi = -Infinity;
    for (const ln of lines) for (const v of ln.value) { if (v < lo) lo = v; if (v > hi) hi = v; }
    this._range = lines.length ? { minValue: lo, maxValue: hi } : null;
    this._req && this._req();
  }
  attached({ chart, series, requestUpdate }) { this._chart = chart; this._series = series; this._req = requestUpdate; }
  updateAllViews() {}
  // contribute grey extent to autoscale so the price scale still fits the bulk
  autoscaleInfo() { return this._range ? { priceRange: this._range } : null; }
  paneViews() { return [{ zOrder: () => 'bottom', renderer: () => ({ draw: t => this._draw(t) }) }]; }
  _draw(target) {
    if (!this._lines || !this._lines.length) return;
    const ts = this._chart.timeScale();
    // most lines share the BTC grid — memoize time→x so per-line domains stay cheap during pan/zoom
    const xcache = new Map();
    const xat = t => { let x = xcache.get(t); if (x === undefined) { x = ts.timeToCoordinate(t); xcache.set(t, x); } return x; };
    // price scale is linear here, so derive the affine value→pixel map from two probes
    const c0 = this._series.priceToCoordinate(0), c1 = this._series.priceToCoordinate(1);
    if (c0 == null || c1 == null) return;
    const slope = c1 - c0;
    target.useBitmapCoordinateSpace(scope => {
      const ctx = scope.context, hr = scope.horizontalPixelRatio, vr = scope.verticalPixelRatio;
      ctx.lineWidth = vr; ctx.strokeStyle = GREY;
      for (const ln of this._lines) {
        ctx.beginPath();
        let started = false;
        for (let j = 0; j < ln.time.length; j++) {
          const x = xat(ln.time[j]); if (x == null) continue;
          const px = x * hr, py = (c0 + ln.value[j] * slope) * vr;
          if (started) ctx.lineTo(px, py); else { ctx.moveTo(px, py); started = true; }
        }
        ctx.stroke();
      }
    });
  }
}

export async function mount(el, src) {
  const { createChart, LineSeries } = await lib();
  const d = await (await fetch(src)).json();
  if (!chart) {
    chart = createChart(el, {
      autoSize: true,
      layout: { background: { color: 'transparent' }, textColor: '#cbd5e1' },
      grid: { vertLines: { color: '#ffffff10' }, horzLines: { color: '#ffffff10' } },
      rightPriceScale: { borderVisible: false },
      timeScale: { timeVisible: true, borderVisible: false },
    });
  }

  const seen = new Set();
  d.series.forEach((m, i) => {
    seen.add(m.pair);
    let s = seriesByPair.get(m.pair);
    if (!s) {
      s = chart.addSeries(LineSeries, { color: m.color, lineWidth: m.width, priceFormat: PCT, priceLineVisible: false, lastValueVisible: true, crosshairMarkerVisible: true });
      seriesByPair.set(m.pair, s);
    } else {
      s.applyOptions({ color: m.color, lineWidth: m.width });
    }
    const ln = d.values[i];
    s.setData(ln.time.map((t, j) => ({ time: t, value: ln.value[j] })));
  });
  for (const [pair, s] of seriesByPair) {
    if (!seen.has(pair)) { chart.removeSeries(s); seriesByPair.delete(pair); }
  }

  // greys: one primitive, hosted on any live series (they share the right price scale)
  if (!bulk) bulk = new BulkLines();
  bulk.setData(d.bulk);
  const host = d.series[0] && seriesByPair.get(d.series[0].pair);
  if (host && host !== bulkHost) { host.attachPrimitive(bulk); bulkHost = host; }

  chart.timeScale().fitContent();
  renderLegend(el, d.legend, d.title);
}

function renderLegend(el, legend, title) {
  let box = el.querySelector('.ms-legend');
  if (!box) {
    box = document.createElement('div');
    box.className = 'ms-legend';
    box.style.cssText = 'position:absolute;top:8px;left:8px;z-index:3;font:11px ui-monospace,monospace;line-height:1.4;pointer-events:none;text-align:left';
    el.appendChild(box);
  }
  box.replaceChildren();
  if (title) {
    const t = document.createElement('div');
    t.textContent = title;
    t.style.cssText = 'color:#cbd5e1;margin-bottom:4px;font-weight:600';
    box.appendChild(t);
  }
  for (const e of legend) {
    const row = document.createElement('div');
    row.textContent = e.label;
    row.style.color = e.color;
    row.style.whiteSpace = 'pre';
    box.appendChild(row);
  }
}
