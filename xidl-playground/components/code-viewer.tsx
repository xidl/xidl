'use client';

import { useTheme } from 'next-themes';
import { useEffect, useMemo, useState } from 'react';
import { getHighlighter } from 'shiki';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';

type CodeViewerProps = {
  code: string;
  language: string;
  className?: string;
};

export function CodeViewer({ code, language, className }: CodeViewerProps) {
  const [html, setHtml] = useState('');
  const [loading, setLoading] = useState(true);
  const { theme } = useTheme();
  const isDark = theme === 'dark';

  console.log(language);
  if (language === 'hir' || language === 'typed_ast') {
    code = JSON.stringify(JSON.parse(code), null, 2);
    language = 'json';
  }

  const codeTheme = useMemo(
    () => (isDark ? 'github-dark' : 'github-light'),
    [theme],
  );

  useEffect(() => {
    let active = true;
    setLoading(true);
    setHtml('');

    const render = async () => {
      try {
        const highlighter = await getHighlighter({
          themes: ['github-dark', 'github-light'],
          langs: [language],
        });
        const rendered = highlighter.codeToHtml(code || '', {
          lang: language,
          theme: codeTheme,
        });
        if (active) {
          setHtml(rendered);
        }
      } catch (err) {
        if (active) {
          setHtml(
            `<pre class="shiki"><code>${(code || '').replace(/</g, '&lt;')}</code></pre>`,
          );
        }
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    render();

    return () => {
      active = false;
    };
  }, [code, language, codeTheme]);

  if (loading) {
    return (
      <div className={cn('space-y-3', className)}>
        <Skeleton className="h-4 w-1/3" />
        <Skeleton className="h-4 w-2/3" />
        <Skeleton className="h-4 w-5/6" />
        <Skeleton className="h-4 w-3/4" />
      </div>
    );
  }

  return (
    <div
      className={cn('font-mono [&_.shiki]:bg-transparent', className)}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
