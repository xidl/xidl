import { Code2, FileCode, Github, Share2 } from 'lucide-react';
import { ThemeToggle } from '@/components/theme-toggle';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import { LANG_OPTIONS } from './constants';

type HeaderBarProps = {
  lang: string;
  enableMetadata: boolean;
  loading: boolean;
  copySuccess: boolean;
  onLangChange: (lang: string) => void;
  onMetadataChange: (checked: boolean) => void;
  onGenerate: () => void;
  onOpenGithub: () => void;
  onShare: () => void;
};

export function HeaderBar({
  lang,
  enableMetadata,
  loading,
  copySuccess,
  onLangChange,
  onMetadataChange,
  onGenerate,
  onOpenGithub,
  onShare,
}: HeaderBarProps) {
  return (
    <header className="flex h-14 items-center justify-between border-border border-b bg-card px-6 shadow-sm">
      <div className="flex items-center gap-3">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-linear-to-br from-primary to-primary/80 text-primary-foreground shadow-sm">
          <Code2 className="h-4 w-4" />
        </div>
        <div>
          <div className="font-semibold text-sm leading-none">
            XIDL Playground
          </div>
          <div className="mt-0.5 text-muted-foreground text-xs">
            Interactive IDL compiler
          </div>
        </div>
      </div>

      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2">
          <Label className="text-muted-foreground text-xs">Language</Label>
          <Select value={lang} onValueChange={onLangChange}>
            <SelectTrigger className="h-8 w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {LANG_OPTIONS.map(opt => {
                return (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                );
              })}
            </SelectContent>
          </Select>
        </div>

        <Separator orientation="vertical" className="h-6" />

        <div className="flex items-center gap-2">
          <Label className="text-muted-foreground text-xs">Metadata</Label>
          <Switch
            checked={enableMetadata}
            onCheckedChange={onMetadataChange}
            className="scale-90"
          />
        </div>

        <Button
          onClick={onGenerate}
          disabled={loading}
          className="h-8 gap-1.5 shadow-sm"
        >
          <FileCode className="h-3.5 w-3.5" />
          {loading ? 'Generating...' : 'Generate'}
        </Button>

        <Button
          variant="ghost"
          size="sm"
          onClick={onOpenGithub}
          className="gap-2"
        >
          <Github className="h-4 w-4" />
          GitHub
        </Button>

        <Button variant="ghost" size="sm" onClick={onShare} className="gap-2">
          <Share2 className="h-4 w-4" />
          {copySuccess ? 'Copied!' : 'Share'}
        </Button>

        <ThemeToggle />
      </div>
    </header>
  );
}
