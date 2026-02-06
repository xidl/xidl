import { FileCode } from 'lucide-react';
import { CodeViewer } from '@/components/code-viewer';
import type { OutputFile } from '@/components/playground/types';
import { inferLanguage } from '@/components/playground/utils';
import { ResizablePanel } from '@/components/ui/resizable';
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';

type OutputPanelProps = {
  files: OutputFile[];
  lang: string;
  selectedTab: string;
  onSelectedFileChange: (value: string) => void;
};

export function OutputPanel({
  files,
  lang,
  selectedTab,
  onSelectedFileChange,
}: OutputPanelProps) {
  return (
    <ResizablePanel defaultSize={60} minSize={30}>
      <Tabs
        value={selectedTab}
        onValueChange={onSelectedFileChange}
        className="flex h-full flex-col"
      >
        <div className="flex flex-col border-border border-b bg-muted/30">
          <div className="flex items-center justify-between px-4 py-2.5">
            <div className="flex items-center gap-2">
              <FileCode className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="font-medium text-sm">Output Files</span>
              <span className="rounded-md bg-muted px-1.5 py-0.5 font-mono text-muted-foreground text-xs">
                {files.length}
              </span>
            </div>
          </div>
          <ScrollArea className="border-border border-t">
            <TabsList className="h-auto w-full justify-start rounded-none bg-transparent p-0">
              {files.length === 0 && (
                <TabsTrigger
                  value="empty"
                  className="rounded-none border-transparent border-b-2 border-none data-[state=active]:border-primary data-[state=active]:bg-transparent"
                >
                  No files
                </TabsTrigger>
              )}
              {files.map(file => (
                <TabsTrigger
                  key={file.path}
                  value={file.path}
                  className="group relative rounded-none border-transparent border-b-2 border-none px-3 py-2 data-[state=active]:border-primary data-[state=active]:bg-transparent"
                >
                  <span className="font-mono text-xs">{file.path}</span>
                </TabsTrigger>
              ))}
            </TabsList>
            <ScrollBar orientation="horizontal" />
          </ScrollArea>
        </div>

        <div className="flex-1 overflow-hidden">
          {files.length === 0 && (
            <TabsContent
              value="empty"
              className="m-0 flex h-full items-center justify-center"
            >
              <div className="text-center">
                <FileCode className="mx-auto h-12 w-12 text-muted-foreground/50" />
                <p className="mt-3 font-medium text-muted-foreground text-sm">
                  No output yet
                </p>
                <p className="mt-1 text-muted-foreground text-xs">
                  Click Generate to see the results
                </p>
              </div>
            </TabsContent>
          )}
          {files.map(file => (
            <TabsContent
              key={file.path}
              value={file.path}
              className="m-0 h-full overflow-hidden"
            >
              <ScrollArea className="h-full">
                <div className="p-4">
                  <CodeViewer
                    code={file.content}
                    language={inferLanguage(lang, file.path)}
                  />
                </div>
                <ScrollBar orientation="horizontal" />
              </ScrollArea>
            </TabsContent>
          ))}
        </div>
      </Tabs>
    </ResizablePanel>
  );
}
