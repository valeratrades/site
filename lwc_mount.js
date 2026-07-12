import { createChart, LineSeries } from "https://cdn.jsdelivr.net/npm/lightweight-charts@5/dist/lightweight-charts.standalone.production.mjs";

let chart, seriesByPair = new Map();

export async function mount(el, src) {
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
      s = chart.addSeries(LineSeries, { color: m.color, lineWidth: m.width, priceLineVisible: false, lastValueVisible: false, crosshairMarkerVisible: false });
      seriesByPair.set(m.pair, s);
    } else {
      s.applyOptions({ color: m.color, lineWidth: m.width });
    }
    s.setData(d.time.map((t, j) => ({ time: t, value: d.values[i][j] })));
  });
  for (const [pair, s] of seriesByPair) {
    if (!seen.has(pair)) { chart.removeSeries(s); seriesByPair.delete(pair); }
  }
  chart.timeScale().fitContent();
  renderLegend(el, d.legend, d.title);
}

function renderLegend(el, legend, title) {
  let box = el.querySelector('.ms-legend');
  if (!box) {
    box = document.createElement('div');
    box.className = 'ms-legend';
    box.style.cssText = 'position:absolute;top:8px;right:8px;z-index:3;font:11px ui-monospace,monospace;line-height:1.4;pointer-events:none;text-align:right';
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
