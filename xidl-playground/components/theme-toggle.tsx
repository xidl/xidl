'use client';

import { Moon, Sun } from 'lucide-react';
import { useTheme } from 'next-themes';
import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => setMounted(true), []);

  if (!mounted) {
    return null;
  }

  const isDark = theme === 'dark';

  return (
    <Button variant="ghost" onClick={() => setTheme(isDark ? 'light' : 'dark')}>
      {isDark ? <Moon /> : <Sun />}
    </Button>
  );
}
