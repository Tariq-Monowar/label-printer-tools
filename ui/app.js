const { invoke } = window.__TAURI__.core
const { listen } = window.__TAURI__.event

const $ = (id) => document.getElementById(id)
const DEFAULT = '' // empty = Windows default printer

function renderJobs(jobs) {
  const list = $('jobs')
  if (!jobs.length) {
    list.innerHTML = '<li class="empty">Noch nichts gedruckt</li>'
    return
  }
  list.replaceChildren(
    ...jobs.map((job) => {
      const li = document.createElement('li')
      if (job.error) li.className = 'failed'
      const time = new Date(job.at).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' })
      li.innerHTML = `<span class="badge">${job.error ? '!' : '✓'}</span><div class="job"><b></b><small></small></div>`
      li.querySelector('b').textContent = job.title
      li.querySelector('small').textContent = job.error ? `${time} · ${job.error}` : `${time} · gedruckt`
      return li
    }),
  )
}

async function loadPrinters(selected) {
  const select = $('printer')
  select.disabled = true
  const names = await invoke('printers')
  const options = [['Standarddrucker von Windows', DEFAULT], ...names.map((n) => [n, n])]
  // keep a saved printer visible even when it is switched off right now
  if (selected && !names.includes(selected)) options.push([`${selected} (nicht gefunden)`, selected])
  select.replaceChildren(...options.map(([text, value]) => new Option(text, value, false, value === selected)))
  select.disabled = false
}

async function init() {
  const state = await invoke('overview')
  renderJobs(state.jobs)
  $('footer').textContent = `v${state.version} · Port ${state.port} · Fenster schließen beendet den Druck nicht`
  if (state.server_error) {
    $('status').classList.add('error')
    $('status-text').textContent = state.server_error
  }
  await loadPrinters(state.printer)
}

$('printer').addEventListener('change', (e) => invoke('set_printer', { name: e.target.value }))
$('refresh').addEventListener('click', () => loadPrinters($('printer').value))
$('test').addEventListener('click', async (e) => {
  const button = e.currentTarget
  button.disabled = true
  button.textContent = 'Druckt …'
  await invoke('test_print').catch(() => {}) // the result shows in the job list
  button.disabled = false
  button.textContent = 'Testdruck'
})

listen('jobs', (event) => renderJobs(event.payload))
init()
