import { useEffect, useState } from "react";
import { Repo, DocHandle, type AutomergeUrl } from "@automerge/automerge-repo";
import { BrowserWebSocketClientAdapter } from "@automerge/automerge-repo-network-websocket";
import { IndexedDBStorageAdapter } from "@automerge/automerge-repo-storage-indexeddb";
import { ImmutableString } from "@automerge/automerge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { Slider } from "@/components/ui/slider";
import { Checkbox } from "@/components/ui/checkbox";
import { Plus, Minus, Trash2, Copy, Check } from "lucide-react";
import { AutomergeCodeMirror } from "./components/AutomergeCodeMirror";
import { NotebookCell } from "./components/NotebookCell";

interface TodoItem {
  id: ImmutableString | string;
  text: ImmutableString | string;
  completed: boolean;
}

interface NotebookCell {
  id: ImmutableString | string;
  cellType: ImmutableString | string; // "code" or "markdown"
  source: ImmutableString | string; // CRDT text
  executionCount: number | null;
  outputRefs: (ImmutableString | string)[]; // URLs to output artifacts
}

interface NotebookMetadata {
  title?: ImmutableString | string;
  createdAt?: number;
  lastModified?: number;
}

// Output interface used by NotebookCell component
// Kept in App.tsx for shared types
export interface Output {
  outputType: "stream" | "display_data" | "execute_result" | "error";
  data: Record<string, string>;
  text?: string;
}

interface Doc {
  // Basic scalar types
  counter: number;
  temperature: number;
  darkMode: boolean;

  // Text type (CRDT)
  notes: ImmutableString | string;
  code: ImmutableString | string;

  // List types
  todos: TodoItem[];
  tags: (ImmutableString | string)[];

  // Map/Object type
  metadata?: {
    createdAt?: number;
    lastModified?: number;
    title?: ImmutableString | string;
  };

  // Notebook fields
  cells: NotebookCell[];
  notebookMetadata: NotebookMetadata;
}

function App() {
  const [docHandle, setDocHandle] = useState<DocHandle<Doc> | null>(null);
  const [doc, setDoc] = useState<Doc | null>(null);
  const [newTodo, setNewTodo] = useState("");
  const [newTag, setNewTag] = useState("");
  const [copied, setCopied] = useState(false);
  const [peerId, setPeerId] = useState<string>("");

  // Helper to safely extract boolean value from darkMode
  // Handles both plain boolean and {val: boolean} object formats
  const getDarkMode = (value: unknown): boolean => {
    if (value === null || value === undefined) return false;
    if (typeof value === "boolean") return value;
    if (typeof value === "object" && "val" in value) {
      return Boolean((value as { val: unknown }).val);
    }
    return false;
  };

  // Helper to convert ImmutableString or string to plain string
  // ImmutableString.toString() returns the underlying string value
  const toStr = (value: ImmutableString | string): string => {
    if (typeof value === "string") return value;
    return value.toString();
  };

  // Sync darkMode state with document class
  useEffect(() => {
    if (doc?.darkMode) {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  }, [doc?.darkMode]);

  useEffect(() => {
    let cleanup: (() => void) | null = null;

    const initRepo = async () => {
      const repo = new Repo({
        network: [new BrowserWebSocketClientAdapter("ws://localhost:3030")],
        storage: new IndexedDBStorageAdapter(),
      });

      // Store the peer ID for cursor tracking
      setPeerId(repo.networkSubsystem.peerId);

      const hash = window.location.hash.slice(1);
      let handle;

      if (hash) {
        handle = await repo.find<Doc>(hash as AutomergeUrl);
      } else {
        handle = repo.create<Doc>();
        handle.change((d: Doc) => {
          // Initialize document with all data types
          d.counter = 0;
          d.temperature = 20;
          d.darkMode = false;
          d.notes = "";
          d.code = "// Try collaborative coding!\nfunction hello() {\n  console.log('Hello, Automerge!');\n}\n";
          d.todos = [];
          d.tags = [];
          d.metadata = {
            createdAt: Date.now(),
            lastModified: Date.now(),
            title: "Autodash Demo",
          };
          // Initialize notebook
          d.cells = [
            {
              id: `cell-${Date.now()}-1`,
              cellType: "code",
              source: "// First code cell\nconsole.log('Hello from notebook!');",
              executionCount: null,
              outputRefs: [],
            },
            {
              id: `cell-${Date.now()}-2`,
              cellType: "markdown",
              source: "# Welcome to Collaborative Notebooks\n\nThis is a markdown cell. Try editing it!",
              executionCount: null,
              outputRefs: [],
            },
          ];
          d.notebookMetadata = {
            title: "Untitled Notebook",
            createdAt: Date.now(),
            lastModified: Date.now(),
          };
        });
        window.location.hash = handle.url;
      }

      setDocHandle(handle);
      await handle.whenReady();

      const updateDoc = () => {
        const currentDoc = handle.doc();
        if (currentDoc) {
          setDoc(currentDoc);
        }
      };

      updateDoc();

      const changeListener = () => {
        updateDoc();
      };
      handle.on("change", changeListener);
      cleanup = () => {
        handle.off("change", changeListener);
      };
    };

    initRepo().catch(console.error);

    return () => {
      if (cleanup) cleanup();
    };
  }, []);

  // Sync darkMode state with document class
  useEffect(() => {
    const darkModeValue = getDarkMode(doc?.darkMode);
    if (darkModeValue) {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  }, [doc?.darkMode]);

  const incrementCounter = () => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      d.counter = (d.counter || 0) + 1;
      if (!d.metadata) d.metadata = {};
      d.metadata.lastModified = Date.now();
    });
  };

  const decrementCounter = () => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      d.counter = (d.counter || 0) - 1;
      if (!d.metadata) d.metadata = {};
      d.metadata.lastModified = Date.now();
    });
  };

  const setTemperature = (value: number[]) => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      d.temperature = value[0];
      if (!d.metadata) d.metadata = {};
      d.metadata.lastModified = Date.now();
    });
  };

  const toggleDarkMode = (checked: boolean) => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      d.darkMode = checked;
      if (!d.metadata) d.metadata = {};
      d.metadata.lastModified = Date.now();
    });
  };

  const updateNotes = (newNotes: string) => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      d.notes = newNotes;
      if (!d.metadata) d.metadata = {};
      d.metadata.lastModified = Date.now();
    });
  };

  const addTodo = () => {
    if (!docHandle || !newTodo.trim()) return;
    docHandle.change((d: Doc) => {
      if (!d.todos) d.todos = [];
      d.todos.push({
        id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        text: newTodo,
        completed: false,
      });
      if (!d.metadata) d.metadata = {};
      d.metadata.lastModified = Date.now();
    });
    setNewTodo("");
  };

  const toggleTodo = (id: string) => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      const todo = d.todos?.find((t) => toStr(t.id) === id);
      if (todo) {
        todo.completed = !todo.completed;
        if (!d.metadata) d.metadata = {};
        d.metadata.lastModified = Date.now();
      }
    });
  };

  const deleteTodo = (id: string) => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      if (!d.todos) return;
      const index = d.todos.findIndex((t) => toStr(t.id) === id);
      if (index !== -1) {
        d.todos.splice(index, 1);
        if (!d.metadata) d.metadata = {};
        d.metadata.lastModified = Date.now();
      }
    });
  };

  const addTag = () => {
    if (!docHandle || !newTag.trim()) return;
    docHandle.change((d: Doc) => {
      if (!d.tags) d.tags = [];
      if (!d.tags.includes(newTag)) {
        d.tags.push(newTag);
        if (!d.metadata) d.metadata = {};
        d.metadata.lastModified = Date.now();
      }
    });
    setNewTag("");
  };

  const removeTag = (tag: string) => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      if (!d.tags) return;
      const index = d.tags.indexOf(tag);
      if (index !== -1) {
        d.tags.splice(index, 1);
        if (!d.metadata) d.metadata = {};
        d.metadata.lastModified = Date.now();
      }
    });
  };

  // Notebook functions
  const addCell = (cellType: "code" | "markdown" = "code") => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      if (!d.cells) d.cells = [];
      d.cells.push({
        id: `cell-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        cellType,
        source: cellType === "code" ? "// New code cell" : "# New markdown cell",
        executionCount: null,
        outputRefs: [],
      });
      if (!d.notebookMetadata) d.notebookMetadata = {};
      d.notebookMetadata.lastModified = Date.now();
    });
  };

  const deleteCell = (index: number) => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      if (!d.cells || index < 0 || index >= d.cells.length) return;
      d.cells.splice(index, 1);
      if (!d.notebookMetadata) d.notebookMetadata = {};
      d.notebookMetadata.lastModified = Date.now();
    });
  };

  const moveCellUp = (index: number) => {
    if (!docHandle || index <= 0) return;
    docHandle.change((d: Doc) => {
      if (!d.cells || index >= d.cells.length) return;
      const cell = d.cells[index];
      d.cells.splice(index, 1);
      d.cells.splice(index - 1, 0, cell);
      if (!d.notebookMetadata) d.notebookMetadata = {};
      d.notebookMetadata.lastModified = Date.now();
    });
  };

  const moveCellDown = (index: number) => {
    if (!docHandle) return;
    docHandle.change((d: Doc) => {
      if (!d.cells || index < 0 || index >= d.cells.length - 1) return;
      const cell = d.cells[index];
      d.cells.splice(index, 1);
      d.cells.splice(index + 1, 0, cell);
      if (!d.notebookMetadata) d.notebookMetadata = {};
      d.notebookMetadata.lastModified = Date.now();
    });
  };

  const executeCell = async (index: number) => {
    if (!docHandle) return;

    // Mock execution - in real implementation, this would:
    // 1. Send code to runtimed via WebSocket/HTTP
    // 2. Receive execution result
    // 3. Store output to artifact service
    // 4. Add outputRef URL to cell

    docHandle.change((d: Doc) => {
      if (!d.cells || index < 0 || index >= d.cells.length) return;
      const cell = d.cells[index];

      // Increment execution count
      cell.executionCount = (cell.executionCount || 0) + 1;

      // Mock: Add a fake output ref
      const outputId = `output-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      const outputRef = `https://artifacts.example.com/${outputId}`;

      if (!cell.outputRefs) cell.outputRefs = [];
      cell.outputRefs = [outputRef]; // Replace previous outputs

      if (!d.notebookMetadata) d.notebookMetadata = {};
      d.notebookMetadata.lastModified = Date.now();
    });

    // TODO: Integrate with runtimed for actual code execution
    console.log(`Executed cell ${index}: Mock execution`);
  };

  const copyUrl = async () => {
    await navigator.clipboard.writeText(window.location.href);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  if (!doc) {
    return (
      <div className="min-h-screen bg-background flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary mx-auto mb-4"></div>
          <p className="text-muted-foreground">Loading Autodash...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="border-b">
        <div className="container mx-auto px-4 py-4">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-3xl font-bold bg-gradient-to-r from-blue-600 to-purple-600 bg-clip-text text-transparent">
                Autodash
              </h1>
              <p className="text-sm text-muted-foreground">
                Comprehensive Automerge CRDT Demo
              </p>
            </div>
            <Button onClick={copyUrl} variant="outline" size="sm">
              {copied ? (
                <Check className="h-4 w-4 mr-2" />
              ) : (
                <Copy className="h-4 w-4 mr-2" />
              )}
              {copied ? "Copied!" : "Share"}
            </Button>
          </div>
        </div>
      </header>

      <div className="container mx-auto px-4 py-8">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {/* Counter (CRDT-friendly integer) */}
          <Card>
            <CardHeader>
              <CardTitle>Counter</CardTitle>
              <CardDescription>
                Concurrent increments merge correctly
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-center gap-4">
                <Button
                  onClick={decrementCounter}
                  variant="outline"
                  size="icon"
                >
                  <Minus className="h-4 w-4" />
                </Button>
                <div className="text-5xl font-bold text-primary w-24 text-center">
                  {doc.counter || 0}
                </div>
                <Button
                  onClick={incrementCounter}
                  variant="outline"
                  size="icon"
                >
                  <Plus className="h-4 w-4" />
                </Button>
              </div>
              <p className="text-xs text-muted-foreground text-center mt-4">
                Type: <code>number</code>
              </p>
            </CardContent>
          </Card>

          {/* Slider (number) */}
          <Card>
            <CardHeader>
              <CardTitle>Temperature</CardTitle>
              <CardDescription>Slider value syncs in real-time</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <div className="text-4xl font-bold text-center text-orange-500">
                  {doc.temperature}°C
                </div>
                <Slider
                  value={[doc.temperature ?? 20]}
                  onValueChange={setTemperature}
                  min={0}
                  max={40}
                  step={1}
                />
                <p className="text-xs text-muted-foreground text-center">
                  Type: <code>number</code>
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Boolean (switch) */}
          <Card>
            <CardHeader>
              <CardTitle>Dark Mode</CardTitle>
              <CardDescription>Boolean state synchronized</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex flex-col items-center justify-center space-y-4 py-4">
                <div className="text-4xl">
                  {getDarkMode(doc.darkMode) ? "🌙" : "☀️"}
                </div>
                <div className="flex items-center space-x-2">
                  <Switch
                    checked={getDarkMode(doc.darkMode)}
                    onCheckedChange={toggleDarkMode}
                  />
                  <span className="text-sm font-medium">
                    {getDarkMode(doc.darkMode) ? "Dark" : "Light"}
                  </span>
                </div>
                <p className="text-xs text-muted-foreground text-center">
                  Type: <code>boolean</code>
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Text (CRDT Text) */}
          <Card>
            <CardHeader>
              <CardTitle>Collaborative Notes</CardTitle>
              <CardDescription>
                CRDT text merges character-by-character
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Textarea
                value={toStr(doc.notes)}
                onChange={(e) => updateNotes(e.target.value)}
                placeholder="Start typing... changes sync in real-time!"
                rows={3}
                className="resize-none"
              />
              <p className="text-xs text-muted-foreground mt-2">
                Type: <code>string</code> (stored as Automerge Text)
              </p>
            </CardContent>
          </Card>

          {/* List of Objects (Todos) */}
          <Card className="md:col-span-1">
            <CardHeader>
              <CardTitle>Todo List</CardTitle>
              <CardDescription>List with complex objects</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <div className="flex gap-2">
                  <Input
                    value={newTodo}
                    onChange={(e) => setNewTodo(e.target.value)}
                    placeholder="New todo"
                    onKeyDown={(e) => e.key === "Enter" && addTodo()}
                  />
                  <Button onClick={addTodo} size="icon" variant="outline">
                    <Plus className="h-4 w-4" />
                  </Button>
                </div>
                <div className="space-y-2 max-h-48 overflow-y-auto">
                  {doc.todos && doc.todos.length > 0 ? (
                    doc.todos.map((todo) => (
                      <div
                        key={toStr(todo.id)}
                        className="flex items-center gap-2 p-2 rounded-md border"
                      >
                        <Checkbox
                          checked={todo.completed}
                          onCheckedChange={() => toggleTodo(toStr(todo.id))}
                        />
                        <span
                          className={`flex-1 text-sm ${
                            todo.completed
                              ? "line-through text-muted-foreground"
                              : ""
                          }`}
                        >
                          {toStr(todo.text)}
                        </span>
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => deleteTodo(toStr(todo.id))}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    ))
                  ) : (
                    <p className="text-sm text-muted-foreground text-center py-4">
                      No todos yet
                    </p>
                  )}
                </div>
                <p className="text-xs text-muted-foreground">
                  Type: <code>Array&lt;Object&gt;</code>
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Simple List (Tags) */}
          <Card>
            <CardHeader>
              <CardTitle>Tags</CardTitle>
              <CardDescription>Simple string array</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <div className="flex gap-2">
                  <Input
                    value={newTag}
                    onChange={(e) => setNewTag(e.target.value)}
                    placeholder="Add tag"
                    onKeyDown={(e) => e.key === "Enter" && addTag()}
                  />
                  <Button onClick={addTag} size="icon" variant="outline">
                    <Plus className="h-4 w-4" />
                  </Button>
                </div>
                <div className="flex flex-wrap gap-2">
                  {doc.tags && doc.tags.length > 0 ? (
                    doc.tags.map((tag) => (
                      <Badge key={toStr(tag)} variant="secondary">
                        {toStr(tag)}
                        <button
                          onClick={() => removeTag(toStr(tag))}
                          className="ml-2 hover:text-destructive"
                        >
                          ×
                        </button>
                      </Badge>
                    ))
                  ) : (
                    <p className="text-sm text-muted-foreground">No tags</p>
                  )}
                </div>
                <p className="text-xs text-muted-foreground">
                  Type: <code>Array&lt;string&gt;</code>
                </p>
              </div>
            </CardContent>
          </Card>

          {/* CodeMirror Editor (automerge-codemirror) */}
          <Card className="md:col-span-2 lg:col-span-3">
            <CardHeader>
              <CardTitle>Collaborative Code Editor</CardTitle>
              <CardDescription>
                Powered by automerge-codemirror with syntax highlighting
              </CardDescription>
            </CardHeader>
            <CardContent>
              {docHandle && (
                <AutomergeCodeMirror
                  docHandle={docHandle}
                  path={["code"]}
                  language="javascript"
                  peerId={peerId}
                  peerName={`User ${peerId.slice(-4)}`}
                />
              )}
              <p className="text-xs text-muted-foreground mt-2">
                Type: <code>string</code> (CRDT Text via automerge-codemirror)
              </p>
              <p className="text-xs text-muted-foreground mt-1">
                Collaborator cursors will appear in color when others are editing
              </p>
            </CardContent>
          </Card>

          {/* Notebook Section */}
          <Card className="md:col-span-2 lg:col-span-3">
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle>Collaborative Notebook</CardTitle>
                  <CardDescription>
                    Jupyter-style notebook with real-time collaboration
                  </CardDescription>
                </div>
                <div className="flex gap-2">
                  <Button onClick={() => addCell("code")} variant="outline" size="sm">
                    <Plus className="h-4 w-4 mr-2" />
                    Code Cell
                  </Button>
                  <Button onClick={() => addCell("markdown")} variant="outline" size="sm">
                    <Plus className="h-4 w-4 mr-2" />
                    Markdown Cell
                  </Button>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              {docHandle && doc.cells && doc.cells.length > 0 ? (
                <div className="space-y-4">
                  {doc.cells.map((cell, index) => (
                    <NotebookCell
                      key={toStr(cell.id)}
                      docHandle={docHandle}
                      cellIndex={index}
                      cell={cell}
                      onExecute={executeCell}
                      onDelete={deleteCell}
                      onMoveUp={moveCellUp}
                      onMoveDown={moveCellDown}
                      isFirst={index === 0}
                      isLast={index === doc.cells.length - 1}
                    />
                  ))}
                </div>
              ) : (
                <div className="text-center py-8 text-muted-foreground">
                  <p>No cells yet. Add a code or markdown cell to get started!</p>
                </div>
              )}
              <p className="text-xs text-muted-foreground mt-4">
                Type: <code>Array&lt;NotebookCell&gt;</code> - Outputs stored as external refs
              </p>
            </CardContent>
          </Card>

          {/* Metadata (Nested Object) */}
          <Card className="md:col-span-2 lg:col-span-3">
            <CardHeader>
              <CardTitle>Document Metadata</CardTitle>
              <CardDescription>
                Nested object with timestamps and stats
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <p className="text-sm font-medium text-muted-foreground">
                    Title
                  </p>
                  <p className="text-lg font-semibold">
                    {doc.metadata?.title
                      ? toStr(doc.metadata.title)
                      : "Untitled"}
                  </p>
                </div>
                <div>
                  <p className="text-sm font-medium text-muted-foreground">
                    Created
                  </p>
                  <p className="text-lg font-semibold">
                    {doc.metadata?.createdAt
                      ? new Date(doc.metadata.createdAt).toLocaleDateString()
                      : "—"}
                  </p>
                </div>
              </div>
              <Separator className="my-4" />
              <p className="text-xs text-muted-foreground">
                Types: <code>Object</code> with <code>number</code> (timestamp)
                and <code>string</code> fields
              </p>
            </CardContent>
          </Card>
        </div>
      </div>

      {/* Footer */}
      <footer className="border-t mt-12">
        <div className="container mx-auto px-4 py-6">
          <div className="flex flex-col md:flex-row justify-between items-center gap-4 text-sm text-muted-foreground">
            <div>
              <p>
                Document ID:{" "}
                <code className="bg-muted px-2 py-1 rounded">
                  {docHandle?.url}
                </code>
              </p>
            </div>
            <div className="flex gap-4">
              <Badge variant="outline">Automerge</Badge>
              <Badge variant="outline">CRDTs</Badge>
              <Badge variant="outline">Real-time</Badge>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
}

export default App;
