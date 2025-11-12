import { useEffect, useRef, useMemo } from "react";
import { EditorView, basicSetup } from "codemirror";
import { EditorState } from "@codemirror/state";
import { javascript } from "@codemirror/lang-javascript";
import { markdown } from "@codemirror/lang-markdown";
import { DocHandle } from "@automerge/automerge-repo";
import { automergeSyncPlugin } from "@automerge/automerge-codemirror";
import { peerCursorField, createCursorSyncPlugin } from "./cursorPlugin";

interface AutomergeCodeMirrorProps {
  docHandle: DocHandle<any>;
  path: string[];
  language?: "javascript" | "markdown" | "plaintext";
  peerId?: string;
  peerName?: string;
}

export function AutomergeCodeMirror({
  docHandle,
  path,
  language = "javascript",
  peerId,
  peerName,
}: AutomergeCodeMirrorProps) {
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const initializedRef = useRef(false);

  // Memoize the path array to prevent unnecessary re-renders
  const pathKey = useMemo(() => path.join("."), [path]);

  // Generate or use provided peer ID
  const localPeerId = useMemo(
    () => peerId || `peer-${Math.random().toString(36).substr(2, 9)}`,
    [peerId]
  );

  const localPeerName = useMemo(
    () => peerName || `User ${localPeerId.slice(-4)}`,
    [peerName, localPeerId]
  );

  useEffect(() => {
    if (!editorRef.current || !docHandle || initializedRef.current) return;

    // Get language extension
    const languageExtensions = [];
    if (language === "javascript") {
      languageExtensions.push(javascript());
    } else if (language === "markdown") {
      languageExtensions.push(markdown());
    }

    // Get the initial document content
    const doc = docHandle.doc();
    const initialText = doc && path.length > 0 ? (doc as any)[path[0]] || "" : "";

    // Create editor state
    const startState = EditorState.create({
      doc: String(initialText),
      extensions: [
        basicSetup,
        ...languageExtensions,
        automergeSyncPlugin({
          handle: docHandle,
          path,
        }),
        peerCursorField,
        createCursorSyncPlugin(docHandle, localPeerId, localPeerName),
        EditorView.theme({
          "&": {
            height: "100%",
            border: "1px solid hsl(var(--border))",
            borderRadius: "0.375rem",
          },
          ".cm-scroller": {
            overflow: "auto",
            fontFamily: "ui-monospace, monospace",
          },
          ".cm-content": {
            caretColor: "hsl(var(--foreground))",
            color: "hsl(var(--foreground))",
          },
          "&.cm-focused .cm-cursor": {
            borderLeftColor: "hsl(var(--foreground))",
          },
          "&.cm-focused .cm-selectionBackground, ::selection": {
            backgroundColor: "hsl(var(--accent))",
          },
          ".cm-gutters": {
            backgroundColor: "hsl(var(--muted))",
            color: "hsl(var(--muted-foreground))",
            border: "none",
          },
          ".cm-remote-cursor": {
            position: "relative",
          },
          ".cm-cursor-label": {
            zIndex: "10",
          },
        }),
        EditorView.lineWrapping,
      ],
    });

    // Create editor view
    const view = new EditorView({
      state: startState,
      parent: editorRef.current,
    });

    viewRef.current = view;
    initializedRef.current = true;

    // Cleanup
    return () => {
      view.destroy();
      viewRef.current = null;
      initializedRef.current = false;
    };
  }, [docHandle, pathKey, language, localPeerId, localPeerName]);

  return <div ref={editorRef} className="h-full min-h-[200px]" />;
}
