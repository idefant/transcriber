import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const currentFilePath = fileURLToPath(import.meta.url);

const formatNumber = (value, digits = 2) =>
  Number.isFinite(value) ? value.toFixed(digits) : 'n/a';

const formatMoney = (value) => `$${formatNumber(value, 6)}`;

const escapeHtml = (value) =>
  String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');

const getModelAnchor = (modelKey) => `model-${modelKey.replaceAll(/[^\w-]/g, '-')}`;

const average = (values) => {
  const numericValues = values.filter((value) => Number.isFinite(value));

  if (numericValues.length === 0) return Number.NaN;

  return numericValues.reduce((sum, value) => sum + value, 0) / numericValues.length;
};

const groupBy = (items, getKey) => {
  const groups = new Map();

  for (const item of items) {
    const key = getKey(item);
    const group = groups.get(key) ?? [];

    group.push(item);
    groups.set(key, group);
  }

  return groups;
};

const getModelSummaries = (results) => {
  const byModel = groupBy(results.runs, (run) => run.modelKey);

  return results.models
    .map((model) => {
      const modelRuns = byModel.get(model.key) ?? [];
      const completedRuns = modelRuns.filter((run) => run.status === 'completed');

      return {
        averageLatencyMs: average(modelRuns.map((run) => run.elapsedMs)),
        averageScore: average(completedRuns.map((run) => run.responseScore)),
        completedRuns: completedRuns.length,
        errorRuns: modelRuns.filter((run) => run.status === 'error').length,
        model,
        runs: modelRuns.length,
        totalCostUsd: modelRuns.reduce((sum, run) => sum + (run.estimatedCostUsd ?? 0), 0),
      };
    })
    .toSorted((left, right) => {
      if (left.model.enabled !== right.model.enabled) {
        return left.model.enabled ? -1 : 1;
      }

      return (right.averageScore || 0) - (left.averageScore || 0);
    });
};

const renderSummaryRows = (summaries) =>
  summaries
    .map(
      (summary, index) => `
        <tr class="${summary.model.enabled ? '' : 'muted'}">
          <td>${summary.model.enabled ? index + 1 : '-'}</td>
          <td>
            <a class="modelLink" href="#${escapeHtml(getModelAnchor(summary.model.key))}">
              <strong>${escapeHtml(summary.model.label)}</strong>
            </a>
            <div class="subtle">${escapeHtml(summary.model.apiId)}</div>
            ${summary.model.notes ? `<div class="warning">${escapeHtml(summary.model.notes)}</div>` : ''}
          </td>
          <td>${escapeHtml(summary.model.provider)}</td>
          <td>${summary.model.enabled ? 'enabled' : 'disabled'}</td>
          <td>${formatNumber(summary.averageScore)}</td>
          <td>${summary.completedRuns}/${summary.runs}</td>
          <td>${summary.errorRuns}</td>
          <td>${formatNumber(summary.averageLatencyMs, 0)} ms</td>
          <td>${formatMoney(summary.totalCostUsd)}</td>
          <td>${formatMoney(summary.model.inputPricePer1M)} / ${formatMoney(summary.model.outputPricePer1M)}</td>
        </tr>
      `,
    )
    .join('');

const renderDetailSections = (results) => {
  const runsByModel = groupBy(results.runs, (run) => run.modelKey);

  return results.models
    .map((model) => {
      const modelRuns = runsByModel.get(model.key) ?? [];
      const cells = groupBy(modelRuns, (run) => `${run.caseKey}::${run.language}`);

      const cellSections = [...cells.entries()]
        .toSorted(([left], [right]) => left.localeCompare(right))
        .map(([key, runs]) => {
          const [caseKey, language] = key.split('::');
          const completedRuns = runs.filter((run) => run.status === 'completed');
          const cellScore = average(completedRuns.map((run) => run.responseScore));
          const cellLatency = average(runs.map((run) => run.elapsedMs));

          const runCards = runs
            .map((run) => {
              const penalties = run.penalties?.length
                ? run.penalties
                    .map(
                      (penalty) =>
                        `<li>${escapeHtml(penalty.label)} <span class="subtle">-${penalty.points}</span></li>`,
                    )
                    .join('')
                : '<li class="subtle">No penalties</li>';

              return `
                <article class="run">
                  <header>
                    <strong>Run ${run.repeatIndex}</strong>
                    <span>${run.status}</span>
                    <span>score: ${formatNumber(run.responseScore)}</span>
                    <span>${formatNumber(run.elapsedMs, 0)} ms</span>
                    <span>${formatMoney(run.estimatedCostUsd ?? 0)}</span>
                  </header>
                  ${
                    run.status === 'error'
                      ? `<pre class="error">${escapeHtml(run.error?.message ?? 'Unknown error')}</pre>`
                      : `<pre>${escapeHtml(run.output)}</pre>`
                  }
                  <ul>${penalties}</ul>
                </article>
              `;
            })
            .join('');

          return `
            <details class="cell">
              <summary>
                <strong>${escapeHtml(caseKey)}</strong>
                <span>language: ${escapeHtml(language)}</span>
                <span>score: ${formatNumber(cellScore)}</span>
                <span>avg: ${formatNumber(cellLatency, 0)} ms</span>
              </summary>
              <div class="prompt">${escapeHtml(runs[0]?.input ?? '')}</div>
              ${runCards}
            </details>
          `;
        })
        .join('');

      return `
        <section class="model" id="${escapeHtml(getModelAnchor(model.key))}">
          <h2>${escapeHtml(model.label)}</h2>
          <p class="subtle">${escapeHtml(model.provider)} / ${escapeHtml(model.apiId)} / ${
            model.enabled ? 'enabled' : 'disabled'
          }</p>
          ${cellSections || '<p class="subtle">No runs for this model.</p>'}
        </section>
      `;
    })
    .join('');
};

const buildHtml = (results) => {
  const summaries = getModelSummaries(results);

  return `<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Post-processing model report</title>
  <style>
    :root {
      color: #172033;
      background: #f5f7fb;
      font-family: Inter, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    }

    body {
      margin: 0;
      padding: 32px;
    }

    main {
      max-width: 1440px;
      margin: 0 auto;
    }

    h1,
    h2 {
      margin: 0 0 12px;
    }

    .panel,
    .model,
    .cell,
    .run {
      border: 1px solid #d9e0ec;
      border-radius: 8px;
      background: #fff;
    }

    .panel,
    .model {
      padding: 20px;
      margin-bottom: 20px;
    }

    table {
      width: 100%;
      border-collapse: collapse;
      font-size: 14px;
    }

    th,
    td {
      padding: 10px;
      border-bottom: 1px solid #edf1f7;
      text-align: left;
      vertical-align: top;
    }

    th {
      color: #526071;
      font-size: 12px;
      text-transform: uppercase;
    }

    tbody tr {
      transition:
        background 120ms ease,
        box-shadow 120ms ease;
    }

    tbody tr:hover {
      background: #eef6ff;
      box-shadow: inset 3px 0 0 #175cd3;
    }

    details.cell {
      padding: 12px;
      margin-top: 12px;
    }

    summary {
      display: flex;
      gap: 16px;
      cursor: pointer;
      flex-wrap: wrap;
    }

    .run {
      padding: 12px;
      margin-top: 12px;
      background: #fbfcff;
    }

    .run header {
      display: flex;
      gap: 12px;
      margin-bottom: 8px;
      color: #526071;
      flex-wrap: wrap;
      font-size: 13px;
    }

    pre {
      overflow-x: auto;
      padding: 12px;
      border-radius: 6px;
      background: #101828;
      color: #f6f8fb;
      white-space: pre-wrap;
    }

    pre.error {
      background: #451a1a;
    }

    .prompt {
      margin-top: 12px;
      padding: 10px;
      border-radius: 6px;
      background: #f4f7fb;
      color: #344054;
    }

    .subtle {
      color: #667085;
      font-size: 12px;
    }

    .warning {
      margin-top: 4px;
      color: #b54708;
      font-size: 12px;
    }

    .modelLink {
      color: #175cd3;
      text-decoration: none;
    }

    .modelLink:hover {
      text-decoration: underline;
    }

    .backToTop {
      position: fixed;
      right: 24px;
      bottom: 24px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 48px;
      height: 48px;
      padding: 0;
      border: 1px solid #b2c3dc;
      border-radius: 999px;
      background: #fff;
      color: #175cd3;
      box-shadow: 0 8px 24px rgb(16 24 40 / 14%);
      text-decoration: none;
    }

    .backToTop:hover {
      background: #eef6ff;
    }

    .backToTop[hidden] {
      display: none;
    }

    .backToTop svg {
      width: 22px;
      height: 22px;
    }

    .muted {
      opacity: 0.58;
    }
  </style>
</head>
<body>
  <main id="top">
    <section class="panel">
      <h1>Post-processing model report</h1>
      <p class="subtle">Started: ${escapeHtml(results.startedAt)}. Finished: ${escapeHtml(
        results.finishedAt,
      )}. Repeats: ${results.config.repeats}. Languages: ${escapeHtml(
        results.config.languages.join(', '),
      )}.</p>
    </section>

    <section class="panel" id="ranking">
      <h2>Ranking</h2>
      <table>
        <thead>
          <tr>
            <th>#</th>
            <th>Model</th>
            <th>Provider</th>
            <th>Status</th>
            <th>Score</th>
            <th>Runs</th>
            <th>Errors</th>
            <th>Avg time</th>
            <th>Cost</th>
            <th>Input/output per 1M</th>
          </tr>
        </thead>
        <tbody>${renderSummaryRows(summaries)}</tbody>
      </table>
    </section>

    ${renderDetailSections(results)}
    <a class="backToTop" href="#top" aria-label="Пролистать наверх" hidden>
      <svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">
        <path d="m18 15-6-6-6 6" />
      </svg>
    </a>
  </main>
  <script>
    const backToTop = document.querySelector('.backToTop');
    const ranking = document.querySelector('#ranking');

    const updateBackToTop = () => {
      if (!backToTop || !ranking) return;

      const rankingRect = ranking.getBoundingClientRect();
      const rankingVisible = rankingRect.bottom > 0 && rankingRect.top < window.innerHeight;

      backToTop.hidden = rankingVisible;
    };

    updateBackToTop();
    window.addEventListener('scroll', updateBackToTop, { passive: true });
    window.addEventListener('resize', updateBackToTop);
  </script>
</body>
</html>`;
};

export const generateReport = async (results, outputFilePath) => {
  await mkdir(path.dirname(outputFilePath), { recursive: true });
  await writeFile(outputFilePath, buildHtml(results), 'utf8');
};

const runFromCli = async () => {
  const resultsFilePath = process.argv[2];

  if (!resultsFilePath) {
    console.error('Usage: node scripts/model-testing/report.mjs <results.json>');
    process.exitCode = 1;
    return;
  }

  const absoluteResultsPath = path.resolve(resultsFilePath);
  const results = JSON.parse(await readFile(absoluteResultsPath, 'utf8'));
  const outputFilePath = path.join(path.dirname(absoluteResultsPath), 'report.html');

  await generateReport(results, outputFilePath);
  console.log(outputFilePath);
};

if (process.argv[1] === currentFilePath) {
  await runFromCli();
}
