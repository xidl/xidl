import { FileCode, Plus, Settings, Trash2 } from 'lucide-react';
import type { PropItem } from '@/components/playground/types';
import { Button } from '@/components/ui/button';
import { Collapsible } from '@/components/ui/collapsible';
import { Input } from '@/components/ui/input';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';

type EditorPanelProps = {
  idl: string;
  formatting: boolean;
  skipClient: boolean;
  skipServer: boolean;
  propsOpen: boolean;
  propItems: PropItem[];
  onIdlChange: (value: string) => void;
  onFormat: () => void;
  onSkipClientChange: (value: boolean) => void;
  onSkipServerChange: (value: boolean) => void;
  onPropsOpenChange: (value: boolean) => void;
  onAddProp: () => void;
  onUpdateProp: (id: string, field: 'key' | 'value', value: string) => void;
  onRemoveProp: (id: string) => void;
};

export function EditorPanel({
  idl,
  formatting,
  skipClient,
  skipServer,
  propsOpen,
  propItems,
  onIdlChange,
  onFormat,
  onSkipClientChange,
  onSkipServerChange,
  onPropsOpenChange,
  onAddProp,
  onUpdateProp,
  onRemoveProp,
}: EditorPanelProps) {
  return (
    <ResizablePanel defaultSize={40} minSize={25}>
      <ResizablePanelGroup orientation="vertical">
        <ResizablePanel defaultSize={65} minSize={30}>
          <div className="flex h-full flex-col">
            <div className="flex items-center justify-between border-border border-b bg-muted/30 px-4 py-2.5">
              <div className="flex items-center gap-2">
                <FileCode className="h-3.5 w-3.5 text-muted-foreground" />
                <span className="font-medium text-sm">IDL Editor</span>
              </div>
              <div className="flex items-center gap-4">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={onFormat}
                  disabled={formatting}
                  className="h-7 px-2 text-xs"
                >
                  {formatting ? 'Formatting…' : 'Format'}
                </Button>
                <div className="flex items-center gap-2 text-xs">
                  <span className="text-muted-foreground">skip_client</span>
                  <Switch
                    checked={skipClient}
                    onCheckedChange={onSkipClientChange}
                    className="scale-75"
                  />
                </div>
                <div className="flex items-center gap-2 text-xs">
                  <span className="text-muted-foreground">skip_server</span>
                  <Switch
                    checked={skipServer}
                    onCheckedChange={onSkipServerChange}
                    className="scale-75"
                  />
                </div>
              </div>
            </div>
            <div className="flex-1 overflow-hidden p-0">
              <Textarea
                value={idl}
                onChange={event => onIdlChange(event.target.value)}
                className="h-full resize-none rounded-none border-none font-mono text-xs leading-relaxed"
                placeholder="Enter your IDL code here..."
              />
            </div>
          </div>
        </ResizablePanel>

        <ResizableHandle withHandle />

        <ResizablePanel defaultSize={35} minSize={20} className="p-2">
          <Collapsible open={propsOpen} onOpenChange={onPropsOpenChange}>
            <div className="flex h-full flex-col border-border">
              <div className="flex items-center gap-2">
                <Settings className="h-3.5 w-3.5 text-muted-foreground" />
                <span className="font-medium text-sm">Properties</span>
                <span className="rounded-md bg-muted px-1.5 py-0.5 font-mono text-muted-foreground text-xs">
                  {propItems.filter(item => item.key.trim()).length}
                </span>
              </div>
              <div className="flex-1 overflow-hidden">
                <ScrollArea className="h-full">
                  <div className="space-y-2 p-3">
                    {propItems.map(item => (
                      <div
                        key={item.id}
                        className="flex gap-2 rounded border border-border p-2"
                      >
                        <Input
                          placeholder="key"
                          value={item.key}
                          onChange={event =>
                            onUpdateProp(item.id, 'key', event.target.value)
                          }
                          className="h-7 flex-1 font-mono text-xs"
                        />
                        <Input
                          placeholder="value"
                          value={item.value}
                          onChange={event =>
                            onUpdateProp(item.id, 'value', event.target.value)
                          }
                          className="h-7 flex-1 font-mono text-xs"
                        />
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => onRemoveProp(item.id)}
                          className="h-7 w-7 p-0"
                        >
                          <Trash2 className="h-3 w-3" />
                        </Button>
                      </div>
                    ))}
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={onAddProp}
                      className="h-7 w-full gap-1.5 text-xs"
                    >
                      <Plus className="h-3 w-3" />
                      Add Property
                    </Button>
                  </div>
                </ScrollArea>
              </div>
            </div>
          </Collapsible>
        </ResizablePanel>
      </ResizablePanelGroup>
    </ResizablePanel>
  );
}
