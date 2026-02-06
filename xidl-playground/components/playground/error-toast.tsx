import { X } from 'lucide-react';
import { Button } from '@/components/ui/button';

type ErrorToastProps = {
  error: string;
  onClose: () => void;
};

export function ErrorToast({ error, onClose }: ErrorToastProps) {
  if (!error) {
    return null;
  }

  return (
    <div className="slide-in-from-bottom-2 fixed right-4 bottom-4 max-w-md animate-in">
      <div className="rounded-lg border border-destructive bg-destructive/10 p-4 shadow-lg">
        <div className="flex items-start gap-3">
          <div className="flex-1">
            <p className="font-medium text-sm">Error</p>
            <p className="mt-1 text-muted-foreground text-xs">{error}</p>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            className="h-6 w-6 p-0"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}
