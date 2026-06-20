import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const currentFilePath = fileURLToPath(import.meta.url);

const formatNumber = (value, digits = 2) =>
  Number.isFinite(value) ? value.toFixed(digits) : 'n/a';

const formatMoney = (value) => `$${formatNumber(value, 6)}`;

const formatMicroDollars = (value) => `${formatNumber(value * 1_000_000, 2)} µ$`;

const escapeHtml = (value) =>
  String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');

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

const isInvalidRun = (run) => run.status === 'error' || (run.penalties?.length ?? 0) > 0;

const renderScoreBadge = (score) => {
  const variant = score === 100 ? 'perfect' : 'warning';

  return `<span class="scoreBadge scoreBadge-${variant}">score: ${formatNumber(score)}</span>`;
};

const getReportModels = (results) => {
  const selectedModelKeys = new Set(results.selectedModelKeys);

  if (selectedModelKeys.size > 0) {
    return results.models.filter((model) => selectedModelKeys.has(model.key));
  }

  return results.models.filter((model) => model.enabled);
};

const getModelSummaries = (results, models) => {
  const byModel = groupBy(results.runs, (run) => run.modelKey);

  return models
    .map((model) => {
      const modelRuns = byModel.get(model.key) ?? [];
      const completedRuns = modelRuns.filter((run) => run.status === 'completed');

      return {
        averageCostUsd: average(modelRuns.map((run) => run.estimatedCostUsd ?? 0)),
        averageLatencyMs: average(modelRuns.map((run) => run.elapsedMs)),
        averageScore: average(completedRuns.map((run) => run.responseScore)),
        completedRuns: completedRuns.length,
        errorRuns: modelRuns.filter((run) => run.status === 'error').length,
        model,
        runs: modelRuns.length,
      };
    })
    .toSorted((left, right) => (right.averageScore || 0) - (left.averageScore || 0));
};

const renderSummaryRows = (summaries) =>
  summaries
    .map(
      (summary, index) => `
        <tr>
          <td>${index + 1}</td>
          <td>
            <button class="modelLink" type="button" data-model-key="${escapeHtml(summary.model.key)}">
              <strong>${escapeHtml(summary.model.label)}</strong>
            </button>
            <div class="subtle">${escapeHtml(summary.model.apiId)}</div>
            ${summary.model.notes ? `<div class="warning">${escapeHtml(summary.model.notes)}</div>` : ''}
          </td>
          <td>${escapeHtml(summary.model.provider)}</td>
          <td>${formatNumber(summary.averageScore)}</td>
          <td>${summary.completedRuns}/${summary.runs}</td>
          <td>${summary.errorRuns}</td>
          <td>${formatNumber(summary.averageLatencyMs, 0)} ms</td>
          <td>${formatMicroDollars(summary.averageCostUsd)}</td>
          <td>${formatMoney(summary.model.inputPricePer1M)} / ${formatMoney(summary.model.outputPricePer1M)}</td>
        </tr>
      `,
    )
    .join('');

const renderPenalties = (run) => {
  if (!run.penalties?.length) {
    return '<li class="subtle">No penalties</li>';
  }

  return run.penalties
    .map((penalty) => {
      const detail = penalty.detail
        ? ` <span class="subtle">${escapeHtml(penalty.detail)}</span>`
        : '';

      return `<li>${escapeHtml(penalty.label)} <span class="subtle">-${penalty.points}</span>${detail}</li>`;
    })
    .join('');
};

const renderRunCards = (runs) =>
  runs
    .map(
      (run) => `
        <article class="run" data-invalid="${isInvalidRun(run) ? 'true' : 'false'}">
          <header>
            <strong>Run ${run.repeatIndex}</strong>
            <span>${run.status}</span>
            ${renderScoreBadge(run.responseScore)}
            <span>${formatNumber(run.elapsedMs, 0)} ms</span>
            <span>${formatMoney(run.estimatedCostUsd ?? 0)}</span>
          </header>
          ${
            run.status === 'error'
              ? `<pre class="error">${escapeHtml(run.error?.message ?? 'Unknown error')}</pre>`
              : `<pre>${escapeHtml(run.output)}</pre>`
          }
          <ul>${renderPenalties(run)}</ul>
        </article>
      `,
    )
    .join('');

const renderModelPanels = (results, models) => {
  const runsByModel = groupBy(results.runs, (run) => run.modelKey);
  const caseOrder = new Map(results.cases.map((testCase, index) => [testCase.key, index]));
  const languageOrder = new Map(
    results.config.languages.map((language, index) => [language, index]),
  );
  const getCellOrder = (key) => {
    const [caseKey, language] = key.split('::');

    return {
      caseIndex: caseOrder.get(caseKey) ?? Number.MAX_SAFE_INTEGER,
      caseKey,
      language,
      languageIndex: languageOrder.get(language) ?? Number.MAX_SAFE_INTEGER,
    };
  };

  return models
    .map((model) => {
      const modelRuns = runsByModel.get(model.key) ?? [];
      const cells = groupBy(modelRuns, (run) => `${run.caseKey}::${run.language}`);

      const cellSections = [...cells.entries()]
        .toSorted(([left], [right]) => {
          const leftOrder = getCellOrder(left);
          const rightOrder = getCellOrder(right);

          return (
            leftOrder.caseIndex - rightOrder.caseIndex ||
            leftOrder.languageIndex - rightOrder.languageIndex ||
            leftOrder.caseKey.localeCompare(rightOrder.caseKey) ||
            leftOrder.language.localeCompare(rightOrder.language)
          );
        })
        .map(([key, runs]) => {
          const [caseKey, language] = key.split('::');
          const completedRuns = runs.filter((run) => run.status === 'completed');
          const cellScore = average(completedRuns.map((run) => run.responseScore));
          const cellLatency = average(runs.map((run) => run.elapsedMs));
          const hasErrors = runs.some((run) => isInvalidRun(run));

          return `
            <details class="cell" data-invalid="${hasErrors ? 'true' : 'false'}">
              <summary>
                <strong>${escapeHtml(caseKey)}</strong>
                <span>language: ${escapeHtml(language)}</span>
                ${renderScoreBadge(cellScore)}
                <span>avg: ${formatNumber(cellLatency, 0)} ms</span>
              </summary>
              <div class="fullResult">
                <div class="prompt">${escapeHtml(runs[0]?.input ?? '')}</div>
                ${renderRunCards(runs)}
              </div>
            </details>
          `;
        })
        .join('');

      return `
        <section class="modelPanel" data-model-panel="${escapeHtml(model.key)}" hidden>
          <header class="modelHeader">
            <div>
              <h2>${escapeHtml(model.label)}</h2>
              <p class="subtle">${escapeHtml(model.provider)} / ${escapeHtml(model.apiId)}</p>
              <label class="invalidToggle">
                <input type="checkbox" data-invalid-toggle />
                <span>Показать только невалидные</span>
              </label>
            </div>
          </header>
          ${cellSections || '<p class="subtle">No runs for this model.</p>'}
        </section>
      `;
    })
    .join('');
};

const buildHtml = (results) => {
  const models = getReportModels(results);
  const summaries = getModelSummaries(results, models);

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

    body.modalOpen {
      overflow: hidden;
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
    .cell,
    .run {
      border: 1px solid #d9e0ec;
      border-radius: 8px;
      background: #fff;
    }

    .panel {
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
      margin: -12px;
      padding: 12px;
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

    .scoreBadge {
      display: inline-flex;
      align-items: center;
      min-height: 22px;
      padding: 2px 8px;
      border-radius: 6px;
      color: #172033;
      font-weight: 600;
    }

    .scoreBadge-perfect {
      background: #dcfce7;
    }

    .scoreBadge-warning {
      background: #fef3c7;
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
      padding: 0;
      border: 0;
      background: transparent;
      color: #175cd3;
      cursor: pointer;
      font: inherit;
      text-align: left;
    }

    .modelLink:hover {
      text-decoration: underline;
    }

    .modalBackdrop {
      position: fixed;
      inset: 0;
      z-index: 10;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 32px;
      background: rgb(16 24 40 / 58%);
    }

    .modalBackdrop[hidden] {
      display: none;
    }

    .modal {
      position: relative;
      width: min(1120px, 100%);
      max-height: min(860px, calc(100vh - 64px));
      overflow: auto;
      padding: 24px;
      border-radius: 8px;
      background: #fff;
      box-shadow: 0 24px 80px rgb(16 24 40 / 24%);
    }

    .modalClose {
      position: sticky;
      top: 0;
      float: right;
      width: 36px;
      height: 36px;
      border: 1px solid #d0d7e2;
      border-radius: 999px;
      background: #fff;
      color: #344054;
      cursor: pointer;
      font-size: 24px;
      line-height: 1;
    }

    .modelHeader {
      margin-bottom: 16px;
      padding-right: 48px;
    }

    .invalidToggle {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      margin-top: 8px;
      color: #344054;
      font-size: 14px;
    }

    .modelPanel.showInvalidOnly .cell[data-invalid="false"] {
      display: none;
    }

    .modelPanel.showInvalidOnly .run[data-invalid="false"] {
      display: none;
    }
  </style>
</head>
<body>
  <main>
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
            <th>Score</th>
            <th>Runs</th>
            <th>Errors</th>
            <th>Avg time</th>
            <th>Avg cost</th>
            <th>Input/output per 1M</th>
          </tr>
        </thead>
        <tbody>${renderSummaryRows(summaries)}</tbody>
      </table>
    </section>
  </main>

  <div class="modalBackdrop" data-modal hidden>
    <section class="modal" role="dialog" aria-modal="true" aria-label="Model run details">
      <button class="modalClose" type="button" data-modal-close aria-label="Закрыть">×</button>
      ${renderModelPanels(results, models)}
    </section>
  </div>

  <script>
    const modal = document.querySelector('[data-modal]');
    const closeButton = document.querySelector('[data-modal-close]');
    const modelButtons = document.querySelectorAll('[data-model-key]');
    const modelPanels = document.querySelectorAll('[data-model-panel]');
    let activePanel = null;

    const closeModal = () => {
      if (!modal) return;

      modal.hidden = true;
      document.body.classList.remove('modalOpen');
      activePanel = null;

      for (const panel of modelPanels) {
        panel.hidden = true;
      }
    };

    const openModal = (modelKey) => {
      if (!modal) return;

      closeModal();
      activePanel = document.querySelector(\`[data-model-panel="\${CSS.escape(modelKey)}"]\`);

      if (!activePanel) return;

      activePanel.hidden = false;
      modal.hidden = false;
      document.body.classList.add('modalOpen');
      closeButton?.focus();
    };

    for (const button of modelButtons) {
      button.addEventListener('click', () => openModal(button.dataset.modelKey));
    }

    for (const toggle of document.querySelectorAll('[data-invalid-toggle]')) {
      toggle.addEventListener('change', () => {
        const panel = toggle.closest('[data-model-panel]');

        panel?.classList.toggle('showInvalidOnly', toggle.checked);
      });
    }

    closeButton?.addEventListener('click', closeModal);

    modal?.addEventListener('click', (event) => {
      if (event.target === modal) {
        closeModal();
      }
    });

    window.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && !modal?.hidden) {
        closeModal();
      }
    });
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
