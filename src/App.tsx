import { useMemo, useState } from 'react'
import { Braces, Check, ChevronDown, Clipboard, Folder, Github, Info, Plus, Search, Sparkles, Trash2 } from 'lucide-react'

type Pattern = { id: number; value: string }

const escapeShell = (value: string) => `'${value.replace(/'/g, `'"'"'`)}'`

function App() {
  const [patterns, setPatterns] = useState<Pattern[]>([
    { id: 1, value: 'AKIA[0-9A-Z]{16}' },
    { id: 2, value: 'password\\s*[=:]\\s*[^\\s]+' },
  ])
  const [sample, setSample] = useState('AWS_KEY=AKIAIOSFODNN7EXAMPLE\npassword = supersecret123\nstatus = ready')
  const [folder, setFolder] = useState('./src')
  const [generated, setGenerated] = useState(false)
  const [copied, setCopied] = useState(false)
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [includeHidden, setIncludeHidden] = useState(false)
  const [ignoreCase, setIgnoreCase] = useState(false)
  const [glob, setGlob] = useState('')

  const repositoryUrl = 'https://github.com/IanBedard/regex-sweep'

  const validPatterns = patterns.map(p => p.value.trim()).filter(Boolean)
  const command = useMemo(() => {
    const expressions = validPatterns.map(p => `-e ${escapeShell(p)}`).join(' ')
    const flags = [includeHidden ? '--hidden' : '', ignoreCase ? '--ignore-case' : '', glob.trim() ? `--glob ${escapeShell(glob.trim())}` : ''].filter(Boolean).join(' ')
    const searchParts = ['rg --json --no-heading --color never', flags, expressions || "-e 'YOUR_REGEX'", escapeShell(folder || '.')].filter(Boolean)
    return `${searchParts.join(' ')} | jq -c 'select(.type == "match") | .data as $d | $d.submatches[] | {path: $d.path.text, regex: .match.text}'`
  }, [patterns, folder, includeHidden, ignoreCase, glob])

  const matches = useMemo(() => validPatterns.map((pattern, index) => {
    try {
      const found = [...sample.matchAll(new RegExp(pattern, 'g'))].map(match => match[0])
      return { pattern, found, color: ['#13a05a', '#805ad5', '#d97706'][index % 3] }
    } catch { return { pattern, found: [], color: '#d14343', invalid: true } }
  }), [patterns, sample])

  const updatePattern = (id: number, value: string) => setPatterns(items => items.map(item => item.id === id ? { ...item, value } : item))
  const addPattern = () => setPatterns(items => [...items, { id: Date.now(), value: '' }])
  const removePattern = (id: number) => setPatterns(items => items.length === 1 ? items : items.filter(item => item.id !== id))
  const copy = async () => { await navigator.clipboard.writeText(command); setCopied(true); setTimeout(() => setCopied(false), 1600) }

  return <div className="shell noise">
    <header className="h-18 border-b border-[#dfe4df] bg-white/80 backdrop-blur">
      <div className="mx-auto flex h-full max-w-[1240px] items-center justify-between px-6">
        <div className="flex items-center gap-3"><div className="grid size-9 place-items-center rounded-xl bg-[#132a1d] text-[#95f5b8]"><Braces size={19}/></div><span className="text-lg font-bold tracking-[-.03em]">Regex Sweep</span><span className="rounded-full border border-[#dce3dd] bg-[#f6f8f6] px-2 py-0.5 text-[10px] font-bold uppercase tracking-widest text-[#617067]">Beta</span></div>
        <div className="flex items-center gap-5 text-sm text-[#657168]"><a href={`${repositoryUrl}#how-it-works`} target="_blank" rel="noreferrer" className="flex items-center gap-1.5 hover:text-[#16211a]"><Info size={15}/> How it works</a><a href={repositoryUrl} target="_blank" rel="noreferrer" className="flex items-center gap-1.5 hover:text-[#16211a]"><Github size={16}/> GitHub</a></div>
      </div>
    </header>

    <main className="mx-auto max-w-[1240px] px-6 py-12">
      <div className="mb-8 flex items-end justify-between gap-6">
        <div><div className="mb-3 inline-flex items-center gap-2 rounded-full bg-[#def8e7] px-3 py-1 text-xs font-bold text-[#147841]"><Sparkles size={13}/> Local-first file search</div><h1 className="max-w-2xl text-4xl font-bold tracking-[-.045em] text-[#142019]">Build a precise regex sweep.</h1><p className="mt-3 max-w-xl text-base leading-7 text-[#667269]">Define patterns, test them against sample text, and generate one safe Bash command that returns clean JSON.</p></div>
        <div className="hidden items-center gap-2 text-xs font-medium text-[#7a857d] md:flex"><span className="size-2 rounded-full bg-[#20b869] shadow-[0_0_0_4px_#dff7e8]"/> Runs entirely on your machine</div>
      </div>

      <section className="panel overflow-hidden rounded-2xl border border-[#dce2dd] bg-white">
        <div className="grid lg:grid-cols-2">
          <div className="border-b border-[#e1e6e2] p-6 lg:border-r lg:border-b-0">
            <div className="mb-5 flex items-center justify-between"><div><p className="text-xs font-bold uppercase tracking-[.14em] text-[#199254]">01 · Patterns</p><h2 className="mt-1 text-lg font-bold tracking-tight">What should we find?</h2></div><span className="rounded-md bg-[#f0f3f0] px-2 py-1 font-mono text-[11px] text-[#68736b]">{patterns.length} {patterns.length === 1 ? 'pattern' : 'patterns'}</span></div>
            <div className="space-y-3">{patterns.map((pattern, index) => <div key={pattern.id} className="group flex items-center gap-2"><span className="w-5 text-right font-mono text-xs text-[#a0aaa2]">{String(index + 1).padStart(2, '0')}</span><label className="input h-12 flex-1 rounded-xl border-[#d9dfda] bg-[#fafbfa] shadow-none focus-within:border-[#20a75d] focus-within:outline-2 focus-within:outline-[#d8f6e4]"><Search size={16} className="text-[#879189]"/><input aria-label={`Regex pattern ${index + 1}`} className="font-mono text-sm" value={pattern.value} onChange={e => updatePattern(pattern.id, e.target.value)} placeholder="Enter a regular expression…"/></label><button aria-label="Remove pattern" onClick={() => removePattern(pattern.id)} className="btn btn-ghost btn-square size-10 text-[#9aa29c] hover:bg-[#fff0f0] hover:text-[#c44848]" disabled={patterns.length === 1}><Trash2 size={16}/></button></div>)}</div>
            <button onClick={addPattern} className="mt-4 ml-7 flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-bold text-[#16864a] hover:bg-[#effaf3]"><Plus size={16}/> Add another pattern</button>
          </div>

          <div className="bg-[#fbfcfb] p-6">
            <div className="mb-5 flex items-center justify-between"><div><p className="text-xs font-bold uppercase tracking-[.14em] text-[#7c5ec7]">02 · Test string</p><h2 className="mt-1 text-lg font-bold tracking-tight">Preview your matches</h2></div><span className="text-xs text-[#879088]">Optional</span></div>
            <textarea aria-label="Sample text" value={sample} onChange={e => setSample(e.target.value)} className="textarea h-40 w-full resize-none rounded-xl border-[#d9dfda] bg-white p-4 font-mono text-sm leading-6 shadow-none focus:border-[#7c5ec7] focus:outline-2 focus:outline-[#e8e1f9]" placeholder="Paste text to test your expressions…" />
            <div className="mt-4 min-h-20 rounded-xl border border-[#e0e5e1] bg-white p-3"><div className="mb-2 flex items-center justify-between text-[11px] font-bold uppercase tracking-wider text-[#8a948c]"><span>Matches</span><span>{matches.reduce((sum, m) => sum + m.found.length, 0)} found</span></div><div className="flex flex-wrap gap-2">{matches.flatMap((m, i) => m.invalid ? [<span key={`i-${i}`} className="rounded-md bg-red-50 px-2 py-1 font-mono text-xs text-red-600">Invalid: {m.pattern}</span>] : m.found.map((found, j) => <span key={`${i}-${j}`} style={{ color: m.color, backgroundColor: `${m.color}12`, borderColor: `${m.color}32` }} className="rounded-md border px-2 py-1 font-mono text-xs">{found}</span>))}{matches.every(m => m.found.length === 0 && !m.invalid) && <span className="text-xs text-[#929b94]">No matches yet</span>}</div></div>
          </div>
        </div>

        <div className="border-t border-[#e1e6e2] p-6"><p className="text-xs font-bold uppercase tracking-[.14em] text-[#d17b1c]">03 · Search scope</p><div className="mt-3 flex flex-col gap-3 sm:flex-row"><label className="input h-12 flex-1 rounded-xl border-[#d9dfda] bg-[#fafbfa] shadow-none focus-within:border-[#20a75d] focus-within:outline-2 focus-within:outline-[#d8f6e4]"><Folder size={17} className="text-[#7f8a82]"/><input aria-label="Folder path" className="font-mono text-sm" value={folder} onChange={e => setFolder(e.target.value)} placeholder="/path/to/folder"/></label><button onClick={() => setGenerated(true)} className="btn h-12 rounded-xl border-0 bg-[#173c28] px-7 text-white shadow-[0_8px_20px_rgba(23,60,40,.18)] hover:bg-[#0f2e1d]"><Sparkles size={16}/> Generate command</button></div><p className="mt-2 text-xs text-[#879088]">Paths and patterns are safely shell-escaped. Requires <code className="rounded bg-[#f0f3f0] px-1">ripgrep</code> and <code className="rounded bg-[#f0f3f0] px-1">jq</code>.</p></div>
      </section>

      <section className={`panel mt-6 overflow-hidden rounded-2xl border bg-white transition-all ${generated ? 'border-[#b9d9c4] opacity-100' : 'border-[#dfe4df] opacity-65'}`}>
        <div className="flex items-center justify-between border-b border-[#e4e8e4] px-5 py-3"><div className="flex items-center gap-2"><span className="flex gap-1.5"><i className="size-2.5 rounded-full bg-[#ff6b65]"/><i className="size-2.5 rounded-full bg-[#f5bd4f]"/><i className="size-2.5 rounded-full bg-[#57c45c]"/></span><span className="ml-2 font-mono text-xs text-[#778179]">generated-command.sh</span></div><button onClick={copy} className="btn btn-ghost btn-sm rounded-lg text-xs text-[#536158]">{copied ? <Check size={14} className="text-[#199254]"/> : <Clipboard size={14}/>} {copied ? 'Copied' : 'Copy'}</button></div>
        <div className="code-scroll overflow-x-auto bg-[#15231a] p-5"><pre className="max-w-full whitespace-pre-wrap break-all font-mono text-sm leading-6 text-[#d9e7dc]"><span className="text-[#74d89b]">$ </span>{generated ? command : '# Your generated command will appear here'}</pre></div>
        <div className="flex flex-wrap items-center justify-between gap-3 px-5 py-3 text-xs text-[#778179]"><span>Output · newline-delimited JSON (NDJSON)</span><button aria-expanded={showAdvanced} onClick={() => setShowAdvanced(value => !value)} className="flex items-center gap-1 font-semibold text-[#4d5a51]">Advanced options <ChevronDown size={14} className={`transition-transform ${showAdvanced ? 'rotate-180' : ''}`}/></button></div>
        {showAdvanced && <div className="grid gap-4 border-t border-[#e4e8e4] bg-[#fafbfa] px-5 py-4 sm:grid-cols-3">
          <label className="flex cursor-pointer items-center gap-3 rounded-xl border border-[#dfe4df] bg-white p-3 text-sm"><input type="checkbox" className="checkbox checkbox-sm border-[#aab4ac] text-[#173c28]" checked={includeHidden} onChange={e => setIncludeHidden(e.target.checked)}/><span><b className="block text-[#263229]">Include hidden files</b><small className="text-[#7b867e]">Adds <code>--hidden</code></small></span></label>
          <label className="flex cursor-pointer items-center gap-3 rounded-xl border border-[#dfe4df] bg-white p-3 text-sm"><input type="checkbox" className="checkbox checkbox-sm border-[#aab4ac] text-[#173c28]" checked={ignoreCase} onChange={e => setIgnoreCase(e.target.checked)}/><span><b className="block text-[#263229]">Ignore letter case</b><small className="text-[#7b867e]">Adds <code>--ignore-case</code></small></span></label>
          <label className="rounded-xl border border-[#dfe4df] bg-white p-3 text-sm"><b className="block text-[#263229]">File glob</b><input value={glob} onChange={e => setGlob(e.target.value)} className="mt-1 w-full border-0 bg-transparent font-mono text-xs outline-none" placeholder="*.ts or !node_modules/**"/></label>
        </div>}
      </section>
    </main>
  </div>
}

export default App
