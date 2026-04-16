import Editor from '@monaco-editor/react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import xmlFormatter from 'xml-formatter'
import { Copy, Check, FileIcon } from 'lucide-react'
import { useState, useMemo } from 'react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

export type DataFormat = 'auto' | 'json' | 'xml' | 'xaml' | 'yaml' | 'bpmn' | 'csv' | 'markdown'

interface DataViewerProps {
  content: string
  filename?: string
  format?: DataFormat
  height?: string
  className?: string
  /** Transport-Enkodierung: "utf8" (Standard) oder "base64" für Binärdateien. */
  encoding?: string
}

// ── MIME-Type-Erkennung anhand Datei-Extension ──────────────────────────────

const EXT_MIME: Record<string, string> = {
  png:      'image/png',
  jpg:      'image/jpeg',
  jpeg:     'image/jpeg',
  gif:      'image/gif',
  webp:     'image/webp',
  svg:      'image/svg+xml',
  pdf:      'application/pdf',
  json:     'application/json',
  xml:      'application/xml',
  bpmn:     'application/xml',
  csv:      'text/csv',
  txt:      'text/plain',
  log:      'text/plain',
  md:       'text/markdown',
  markdown: 'text/markdown',
  yaml:     'text/yaml',
  yml:      'text/yaml',
  html:     'text/html',
  htm:      'text/html',
}

function detectMimeType(filename: string): string {
  const ext = filename.split('.').pop()?.toLowerCase() ?? ''
  return EXT_MIME[ext] ?? 'application/octet-stream'
}

type RenderMode = 'text' | 'image' | 'pdf' | 'binary'

function getRenderMode(mimeType: string): RenderMode {
  if (mimeType.startsWith('image/')) return 'image'
  if (mimeType === 'application/pdf') return 'pdf'
  if (
    mimeType.startsWith('text/') ||
    mimeType === 'application/json' ||
    mimeType === 'application/xml'
  ) return 'text'
  return 'binary'
}

/** Base64 → UTF-8-String (korrekte Mehrbyte-Behandlung). */
function decodeBase64Text(b64: string): string {
  const bytes = Uint8Array.from(atob(b64), c => c.charCodeAt(0))
  return new TextDecoder('utf-8').decode(bytes)
}

// ── Textformat-Erkennung (unverändert) ──────────────────────────────────────

function detectFormat(content: string, filename?: string): Exclude<DataFormat, 'auto'> {
  if (filename) {
    const ext = filename.split('.').pop()?.toLowerCase()
    if (ext === 'json') return 'json'
    if (ext === 'xml') return 'xml'
    if (ext === 'xaml') return 'xaml'
    if (ext === 'bpmn') return 'bpmn'
    if (ext === 'yaml' || ext === 'yml') return 'yaml'
    if (ext === 'csv') return 'csv'
    if (ext === 'md' || ext === 'markdown') return 'markdown'
  }

  const trimmed = content.trimStart()

  try {
    JSON.parse(content)
    return 'json'
  } catch { /* weiter */ }

  if (trimmed.startsWith('<')) {
    if (trimmed.includes('xmlns:bpmn') || trimmed.includes('bpmn:definitions') || trimmed.includes('bpmn2:definitions')) return 'bpmn'
    return 'xml'
  }

  const firstLine = trimmed.split('\n')[0]
  if (/^---/.test(trimmed) || /^[a-zA-Z_][a-zA-Z0-9_]*\s*:/.test(firstLine)) return 'yaml'

  if ((firstLine.match(/,/g) ?? []).length > 1 && !firstLine.includes('<')) return 'csv'

  if (/^#{1,6} /m.test(content)) return 'markdown'

  return 'json'
}

function formatContent(content: string, format: Exclude<DataFormat, 'auto'>): string {
  try {
    if (format === 'json') {
      return JSON.stringify(JSON.parse(content), null, 2)
    }
    if (format === 'xml' || format === 'xaml' || format === 'bpmn') {
      return xmlFormatter(content, {
        indentation: '  ',
        collapseContent: true,
        lineSeparator: '\n',
      })
    }
  } catch { /* Fallback: Rohinhalt */ }
  return content
}

const FORMAT_LABELS: Record<Exclude<DataFormat, 'auto'>, string> = {
  json:     'JSON',
  xml:      'XML',
  xaml:     'XAML',
  yaml:     'YAML',
  bpmn:     'BPMN',
  csv:      'CSV',
  markdown: 'Markdown',
}

const MONACO_LANG: Record<Exclude<DataFormat, 'auto'>, string> = {
  json:     'json',
  xml:      'xml',
  xaml:     'xml',
  yaml:     'yaml',
  bpmn:     'xml',
  csv:      'plaintext',
  markdown: 'markdown',
}

// ── Haupt-Komponente ─────────────────────────────────────────────────────────

export function DataViewer({ content, filename, format = 'auto', height = '400px', className, encoding }: DataViewerProps) {
  const [copied, setCopied] = useState(false)

  // Ermittle Render-Modus und ggf. dekodierten Text für base64-Inhalte
  const { renderMode, mimeType, decodedContent } = useMemo(() => {
    if (encoding !== 'base64') {
      return { renderMode: 'text' as RenderMode, mimeType: 'text/plain', decodedContent: content }
    }
    const fname = filename ?? ''
    const mime = detectMimeType(fname)
    const mode = getRenderMode(mime)
    const decoded = mode === 'text' ? decodeBase64Text(content) : content
    return { renderMode: mode, mimeType: mime, decodedContent: decoded }
  }, [content, filename, encoding])

  const resolved = format === 'auto' ? detectFormat(decodedContent, filename) : format
  const formatted = useMemo(() => formatContent(decodedContent, resolved), [decodedContent, resolved])

  const handleCopy = async () => {
    await navigator.clipboard.writeText(formatted)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  // ── Bild ──
  if (renderMode === 'image') {
    return (
      <div className={cn('flex flex-col rounded-md border overflow-hidden', className)}>
        <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-900 border-b border-zinc-700">
          <Badge variant="secondary" className="text-xs font-mono">
            {mimeType.split('/')[1]?.toUpperCase() ?? 'Bild'}
          </Badge>
        </div>
        <div className="flex items-center justify-center bg-zinc-950 p-4" style={{ minHeight: height }}>
          <img
            src={`data:${mimeType};base64,${content}`}
            alt={filename ?? 'Bild'}
            className="max-w-full max-h-full object-contain rounded"
            style={{ maxHeight: height }}
          />
        </div>
      </div>
    )
  }

  // ── PDF ──
  if (renderMode === 'pdf') {
    return (
      <div className={cn('flex flex-col rounded-md border overflow-hidden', className)}>
        <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-900 border-b border-zinc-700">
          <Badge variant="secondary" className="text-xs font-mono">PDF</Badge>
        </div>
        <embed
          src={`data:application/pdf;base64,${content}`}
          type="application/pdf"
          className="w-full"
          style={{ height }}
        />
      </div>
    )
  }

  // ── Unbekannte Binärdatei ──
  if (renderMode === 'binary') {
    const sizeKb = Math.round((content.length * 3) / 4 / 1024)
    return (
      <div className={cn('flex flex-col rounded-md border overflow-hidden', className)}>
        <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-900 border-b border-zinc-700">
          <Badge variant="secondary" className="text-xs font-mono">Binärdatei</Badge>
        </div>
        <div className="flex flex-col items-center justify-center gap-3 py-12 text-muted-foreground" style={{ minHeight: '120px' }}>
          <FileIcon className="h-10 w-10 opacity-40" />
          <p className="text-sm">{filename ?? 'Datei'}</p>
          <p className="text-xs opacity-60">~{sizeKb} KB · {mimeType}</p>
        </div>
      </div>
    )
  }

  // ── Text / Code (Standard-Pfad) ──
  return (
    <div className={cn('flex flex-col rounded-md border overflow-hidden', className)}>
      {/* Toolbar */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-900 border-b border-zinc-700">
        <Badge variant="secondary" className="text-xs font-mono">
          {FORMAT_LABELS[resolved]}
        </Badge>
        <Button
          size="sm"
          variant="ghost"
          className="h-6 px-2 text-zinc-400 hover:text-zinc-100"
          onClick={handleCopy}
        >
          {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
          <span className="ml-1 text-xs">{copied ? 'Kopiert' : 'Kopieren'}</span>
        </Button>
      </div>

      {/* Inhalt */}
      {resolved === 'markdown' ? (
        <div
          className="flex-1 overflow-auto p-4 bg-background prose prose-sm dark:prose-invert max-w-none"
          style={{ minHeight: height }}
        >
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{decodedContent}</ReactMarkdown>
        </div>
      ) : (
        <Editor
          height={height}
          language={MONACO_LANG[resolved]}
          value={formatted}
          theme="vs-dark"
          options={{
            readOnly: true,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            fontSize: 13,
            lineNumbers: 'on',
            wordWrap: 'on',
            folding: true,
            renderLineHighlight: 'none',
            overviewRulerLanes: 0,
            hideCursorInOverviewRuler: true,
            scrollbar: { verticalScrollbarSize: 6, horizontalScrollbarSize: 6 },
          }}
        />
      )}
    </div>
  )
}
