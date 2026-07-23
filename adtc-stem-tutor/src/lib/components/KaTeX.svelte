<script lang="ts">
  import katex from 'katex';
  import 'katex/dist/katex.min.css';

  export let text = '';

  // Helper to split plain text and math blocks ($...$, $$...$$, \(...\), \[...\])
  function parseMathAndText(input: string) {
    const parts: Array<{ type: 'text' | 'math'; content: string; display?: boolean }> = [];
    // Robust regex to match display math ($$...$$, \[...\]) and inline math ($...$, \(...\))
    const regex = /(\$\$[\s\S]*?\$\$|\$[^\s$](?:[^\n$]*?[^\s$])?\$|\\\[[\s\S]*?\\\]|\\\(.*?\\\))/g;
    let lastIndex = 0;
    let match;

    while ((match = regex.exec(input)) !== null) {
      if (match.index > lastIndex) {
        parts.push({ type: 'text', content: input.slice(lastIndex, match.index) });
      }
      const rawMath = match[0];
      let isDisplay = false;
      let mathContent = "";
      
      if (rawMath.startsWith('$$')) {
        isDisplay = true;
        mathContent = rawMath.slice(2, -2);
      } else if (rawMath.startsWith('$')) {
        isDisplay = false;
        mathContent = rawMath.slice(1, -1);
      } else if (rawMath.startsWith('\\[')) {
        isDisplay = true;
        mathContent = rawMath.slice(2, -2);
      } else if (rawMath.startsWith('\\(')) {
        isDisplay = false;
        mathContent = rawMath.slice(2, -2);
      }
      
      parts.push({ type: 'math', content: mathContent, display: isDisplay });
      lastIndex = regex.lastIndex;
    }

    if (lastIndex < input.length) {
      parts.push({ type: 'text', content: input.slice(lastIndex) });
    }

    return parts;
  }

  function renderMath(mathStr: string, displayMode?: boolean) {
    try {
      return katex.renderToString(mathStr, {
        displayMode,
        throwOnError: false
      });
    } catch (e) {
      return mathStr;
    }
  }

  // Basic HTML escape
  function escapeHtml(str: string): string {
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  }

  // Simple parser to convert basic Markdown syntax into HTML
  function renderMarkdown(input: string): string {
    let html = escapeHtml(input);

    // Multiline Code blocks: ```language ... ```
    html = html.replace(/```(?:[a-zA-Z0-9]+)?\n([\s\S]*?)```/g, '<pre class="bg-slate-800 text-slate-100 p-3 rounded-lg my-3 font-mono text-xs overflow-x-auto border border-slate-700">$1</pre>');
    html = html.replace(/```([\s\S]*?)```/g, '<pre class="bg-slate-800 text-slate-100 p-3 rounded-lg my-3 font-mono text-xs overflow-x-auto border border-slate-700">$1</pre>');

    // Inline code: `code`
    html = html.replace(/`([^`\n]+)`/g, '<code class="bg-slate-200 text-slate-800 px-1.5 py-0.5 rounded font-mono text-[10px] font-bold border border-slate-300">$1</code>');

    // Bold: **text** or __text__
    html = html.replace(/\*\*([\s\S]*?)\*\*/g, '<strong class="font-bold text-slate-900">$1</strong>');
    html = html.replace(/__([\s\S]*?)__/g, '<strong class="font-bold text-slate-900">$1</strong>');

    // Italic: *text* or _text_
    html = html.replace(/\*([\s\S]*?)\*/g, '<em class="italic text-slate-800">$1</em>');
    html = html.replace(/_([\s\S]*?)_/g, '<em class="italic text-slate-800">$1</em>');

    // Bullet lists
    const lines = html.split('\n');
    let inList = false;
    const processedLines = lines.map(line => {
      const trimmed = line.trim();
      const bulletMatch = line.match(/^(\s*)[-*]\s+(.*)$/);
      if (bulletMatch) {
        let prefix = '';
        if (!inList) {
          inList = true;
          prefix = '<ul class="list-disc pl-5 my-2 space-y-1 text-slate-800">';
        }
        return prefix + `<li class="leading-relaxed">${bulletMatch[2]}</li>`;
      } else {
        let prefix = '';
        if (inList) {
          inList = false;
          prefix = '</ul>';
        }
        return prefix + line;
      }
    });
    if (inList) {
      processedLines.push('</ul>');
    }
    
    html = processedLines.join('\n');

    return html;
  }
</script>

<div class="prose prose-xs max-w-none">
  {#each parseMathAndText(text) as part}
    {#if part.type === 'text'}
      <span class="whitespace-pre-wrap">{@html renderMarkdown(part.content)}</span>
    {:else if part.type === 'math'}
      <span class={part.display ? "block my-2 text-center" : "inline-block px-1"}>
        {@html renderMath(part.content, part.display)}
      </span>
    {/if}
  {/each}
</div>