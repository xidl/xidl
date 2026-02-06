'use client';

import { EditorPanel } from '@/components/playground/editor-panel';
import { ErrorToast } from '@/components/playground/error-toast';
import { HeaderBar } from '@/components/playground/header-bar';
import { OutputPanel } from '@/components/playground/output-panel';
import { usePlaygroundState } from '@/components/playground/use-playground-state';
import {
  ResizableHandle,
  ResizablePanelGroup,
} from '@/components/ui/resizable';

export function Playground() {
  const {
    lang,
    enableMetadata,
    loading,
    copySuccess,
    idl,
    formatting,
    skipClient,
    skipServer,
    propsOpen,
    propItems,
    files,
    selectedTab,
    error,
    setEnableMetadata,
    setIdl,
    setSkipClient,
    setSkipServer,
    setPropsOpen,
    setSelectedFile,
    clearError,
    handleLangChange,
    handleOpenGithub,
    handleShare,
    runGenerate,
    runFormat,
    addProp,
    updateProp,
    removeProp,
  } = usePlaygroundState();

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      <HeaderBar
        lang={lang}
        enableMetadata={enableMetadata}
        loading={loading}
        copySuccess={copySuccess}
        onLangChange={handleLangChange}
        onMetadataChange={setEnableMetadata}
        onGenerate={runGenerate}
        onOpenGithub={handleOpenGithub}
        onShare={handleShare}
      />

      <div className="flex-1 overflow-hidden p-4">
        <ResizablePanelGroup
          orientation="horizontal"
          className="h-full rounded-lg border border-border bg-card shadow-sm"
        >
          <EditorPanel
            idl={idl}
            formatting={formatting}
            skipClient={skipClient}
            skipServer={skipServer}
            propsOpen={propsOpen}
            propItems={propItems}
            onIdlChange={setIdl}
            onFormat={runFormat}
            onSkipClientChange={setSkipClient}
            onSkipServerChange={setSkipServer}
            onPropsOpenChange={setPropsOpen}
            onAddProp={addProp}
            onUpdateProp={updateProp}
            onRemoveProp={removeProp}
          />

          <ResizableHandle withHandle />

          <OutputPanel
            files={files}
            lang={lang}
            selectedTab={selectedTab}
            onSelectedFileChange={setSelectedFile}
          />
        </ResizablePanelGroup>
      </div>

      <ErrorToast error={error} onClose={clearError} />
    </div>
  );
}
