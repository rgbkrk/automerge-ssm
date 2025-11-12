import { useEffect, useRef, useMemo } from "react";
import { EditorView, basicSetup } from "codemirror";
import { EditorState } from "@codemirror/state";
import { javascript } from "@codemirror/lang-javascript";
import { markdown } from "@codemirror/lang-markdown";
import { DocHandle } from "@automerge/automerge-repo";
import { automergeSyncPlugin } from "@automerge/automerge-codemirror";

interface AutomergeCodeMirrorProps {
  docHandle: DocHandle<any>;
  path: string[];
  language?: "javascript" | "markdown" | "plaintext";
}

export function AutomergeCodeMirror({
  docHandle,
  path,
  language = "javascript",
}: AutomergeCodeMirrorProps) {
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const initializedRef = useRef(false);

  // Memoize the path array to prevent unnecessary re-renders
  const pathKey = useMemo(() => path.join("."), [path]);

  useEffect(() => {
    if (!editorRef.current || !docHandle || initializedRef.current) return;

    // Get language extension
    const languageExtensions = [];
    if (language === "javascript") {
      languageExtensions.push(javascript());
    } else if (language === "markdown") {
      languageExtensions.push(markdown());
    }

    // Get the initial document content by traversing the path
    const doc = docHandle.doc();
    let initialText = "";
    if (doc && path.length > 0) {
      let current: any = doc;
      for (const key of path) {
        if (current && typeof current === "object") {
          current = current[key];
        } else {
          current = undefined;
          break;
        }
      }
      initialText = current ? String(current) : "";
    }

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
  }, [docHandle, pathKey, language]);

  return <div ref={editorRef} className="h-full min-h-[200px]" />;
}
